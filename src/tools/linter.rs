use crate::lexer::Lexer;
use crate::parser::ast::common::InterpolatedSegment;
use crate::parser::ast::*;
use crate::parser::Parser;
use crate::typecheck::TypeChecker;
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct LintIssue {
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub severity: Severity,
    /// Machine-readable rule identifier (e.g. "L010"). None for legacy rules.
    pub rule_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
            Severity::Info => write!(f, "info"),
        }
    }
}

/// Code linter for Txt-code source files.
///
/// Checks performed:
/// - **unused variable**: variables assigned but never read
/// - **unreachable code**: statements after return/break/continue
/// - **duplicate map keys**: duplicate literal keys in map literals
/// - **suspicious comparisons**: `x == true`, `x == false` (prefer `x`, `not x`)
/// - **recursion depth risk**: functions that directly call themselves
/// - **import not found**: import paths that can't be resolved
/// - **style**: line length, trailing whitespace
pub struct Linter {
    check_types: bool,
    check_style: bool,
}

impl Linter {
    pub fn new() -> Self {
        Self {
            check_types: true,
            check_style: true,
        }
    }

    /// Lint source from a string. Uses current directory for import resolution.
    pub fn lint_source(source: &str) -> Result<Vec<LintIssue>, Box<dyn std::error::Error>> {
        Self::lint_source_with_path(source, None)
    }

    /// Lint source from a string, using `file_path` to resolve relative imports.
    pub fn lint_source_with_path(
        source: &str,
        file_path: Option<&Path>,
    ) -> Result<Vec<LintIssue>, Box<dyn std::error::Error>> {
        let linter = Self::new();
        let mut issues = Vec::new();

        let mut lexer = Lexer::new(source.to_string());
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        let program = parser.parse()?;

        // Type checking
        if linter.check_types {
            let mut type_checker = TypeChecker::new();
            if let Err(msgs) = type_checker.check(&program) {
                for msg in msgs {
                    issues.push(LintIssue {
                        line: 1,
                        column: 1,
                        message: msg,
                        severity: Severity::Error,
                        rule_id: None,
                    });
                }
            }
        }

        // Semantic checks
        issues.extend(check_unused_variables(&program));
        issues.extend(check_unreachable_code(&program));
        issues.extend(check_duplicate_map_keys(&program));
        issues.extend(check_suspicious_comparisons(&program));
        issues.extend(check_recursion_risk(&program));
        issues.extend(check_imports_exist(&program, file_path));
        issues.extend(check_shadowed_variables(&program));
        issues.extend(check_mutable_globals(&program));

        // New rules L010–L019
        issues.extend(check_division_by_zero_literal(&program));
        issues.extend(check_unreachable_match_arm(&program));
        issues.extend(check_empty_catch(&program));
        issues.extend(check_unused_functions(&program));
        issues.extend(check_missing_return_type(&program));
        issues.extend(check_negated_condition(&program));
        issues.extend(check_string_concat_in_loop(&program));
        issues.extend(check_null_comparison(&program));
        issues.extend(check_broad_permission(&program));
        issues.extend(check_match_no_wildcard(&program));

        // Style checks
        if linter.check_style {
            issues.extend(check_style(source));
        }

        issues.sort_by_key(|i| i.line);
        Ok(issues)
    }
}

impl Default for Linter {
    fn default() -> Self {
        Self::new()
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Check 1: Unused variables
// ────────────────────────────────────────────────────────────────────────────

fn check_unused_variables(program: &Program) -> Vec<LintIssue> {
    let mut issues = Vec::new();

    // ── Top-level: variables defined at top level that are never used anywhere
    let mut top_defined: HashMap<String, usize> = HashMap::new();
    for stmt in &program.statements {
        if let Statement::Assignment { pattern, span, .. } = stmt {
            for name in collect_pattern_names(pattern) {
                if !name.starts_with('_') {
                    top_defined.insert(name, span.line);
                }
            }
        }
    }

    let mut all_used: HashSet<String> = HashSet::new();
    for stmt in &program.statements {
        collect_stmt_used(stmt, &mut all_used);
    }

    for (name, line) in &top_defined {
        if !all_used.contains(name) {
            issues.push(LintIssue {
                line: *line,
                column: 1,
                message: format!("Unused variable '{}'", name),
                severity: Severity::Warning,
                rule_id: None,
            });
        }
    }

    // ── Function scope: unused params + local variables
    for stmt in &program.statements {
        if let Statement::FunctionDef {
            params, body, span, ..
        } = stmt
        {
            let mut used_in_body: HashSet<String> = HashSet::new();
            for s in body {
                collect_stmt_used(s, &mut used_in_body);
            }

            // Unused parameters
            for param in params {
                if !param.name.starts_with('_') && !used_in_body.contains(&param.name) {
                    issues.push(LintIssue {
                        line: span.line,
                        column: 1,
                        message: format!("Unused parameter '{}'", param.name),
                        severity: Severity::Warning,
                        rule_id: None,
                    });
                }
            }

            // Unused local variables (assignments in function body)
            let mut local_defined: HashMap<String, usize> = HashMap::new();
            for s in body {
                if let Statement::Assignment {
                    pattern, span: sp, ..
                } = s
                {
                    for n in collect_pattern_names(pattern) {
                        if !n.starts_with('_') {
                            local_defined.insert(n, sp.line);
                        }
                    }
                }
            }
            for (name, line) in &local_defined {
                if !used_in_body.contains(name) {
                    issues.push(LintIssue {
                        line: *line,
                        column: 1,
                        message: format!("Unused variable '{}'", name),
                        severity: Severity::Warning,
                        rule_id: None,
                    });
                }
            }
        }
    }

    issues
}

/// Extract all bound names from a pattern (for destructuring support).
fn collect_pattern_names(pattern: &Pattern) -> Vec<String> {
    match pattern {
        Pattern::Identifier(n) => vec![n.clone()],
        Pattern::Array(patterns) => patterns.iter().flat_map(collect_pattern_names).collect(),
        Pattern::Struct { fields, rest } => {
            let mut names: Vec<String> = fields
                .iter()
                .flat_map(|(_, p)| collect_pattern_names(p))
                .collect();
            if let Some(r) = rest {
                names.push(r.clone());
            }
            names
        }
        Pattern::Constructor { args, .. } => args.iter().flat_map(collect_pattern_names).collect(),
        Pattern::Ignore => vec![],
        Pattern::Or(pats) => pats.iter().flat_map(collect_pattern_names).collect(),
        Pattern::Range(..) => vec![],
        Pattern::Rest(name) => vec![name.clone()],
        Pattern::Literal(_) => vec![],
    }
}

/// Collect all identifier names referenced in expressions within a statement.
fn collect_stmt_used(stmt: &Statement, used: &mut HashSet<String>) {
    match stmt {
        Statement::Assignment { value, .. } => collect_expr_idents(value, used),
        Statement::IndexAssignment {
            target,
            index,
            value,
            ..
        } => {
            collect_expr_idents(target, used);
            collect_expr_idents(index, used);
            collect_expr_idents(value, used);
        }
        Statement::CompoundAssignment { name, value, .. } => {
            used.insert(name.clone());
            collect_expr_idents(value, used);
        }
        Statement::FunctionDef { body, .. } => {
            for s in body {
                collect_stmt_used(s, used);
            }
        }
        Statement::Return { value, .. } => {
            if let Some(v) = value {
                collect_expr_idents(v, used);
            }
        }
        Statement::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
            ..
        } => {
            collect_expr_idents(condition, used);
            for s in then_branch {
                collect_stmt_used(s, used);
            }
            for (cond, branch) in else_if_branches {
                collect_expr_idents(cond, used);
                for s in branch {
                    collect_stmt_used(s, used);
                }
            }
            if let Some(branch) = else_branch {
                for s in branch {
                    collect_stmt_used(s, used);
                }
            }
        }
        Statement::While {
            condition, body, ..
        } => {
            collect_expr_idents(condition, used);
            for s in body {
                collect_stmt_used(s, used);
            }
        }
        Statement::DoWhile {
            body, condition, ..
        } => {
            for s in body {
                collect_stmt_used(s, used);
            }
            collect_expr_idents(condition, used);
        }
        Statement::For {
            variable,
            iterable,
            body,
            ..
        } => {
            used.insert(variable.clone());
            collect_expr_idents(iterable, used);
            for s in body {
                collect_stmt_used(s, used);
            }
        }
        Statement::Repeat { count, body, .. } => {
            collect_expr_idents(count, used);
            for s in body {
                collect_stmt_used(s, used);
            }
        }
        Statement::Expression(e) => collect_expr_idents(e, used),
        Statement::Assert {
            condition, message, ..
        } => {
            collect_expr_idents(condition, used);
            if let Some(m) = message {
                collect_expr_idents(m, used);
            }
        }
        Statement::Try {
            body,
            catch,
            finally,
            ..
        } => {
            for s in body {
                collect_stmt_used(s, used);
            }
            if let Some((var, catch_body)) = catch {
                used.insert(var.clone());
                for s in catch_body {
                    collect_stmt_used(s, used);
                }
            }
            if let Some(finally_body) = finally {
                for s in finally_body {
                    collect_stmt_used(s, used);
                }
            }
        }
        Statement::Match {
            value,
            cases,
            default,
            ..
        } => {
            collect_expr_idents(value, used);
            for (_, guard, case_body) in cases {
                if let Some(g) = guard {
                    collect_expr_idents(g, used);
                }
                for s in case_body {
                    collect_stmt_used(s, used);
                }
            }
            if let Some(default_body) = default {
                for s in default_body {
                    collect_stmt_used(s, used);
                }
            }
        }
        Statement::Const { value, .. } => collect_expr_idents(value, used),
        Statement::Export { names, .. } => {
            for n in names {
                used.insert(n.clone());
            }
        }
        Statement::Enum { .. }
        | Statement::Struct { .. }
        | Statement::Import { .. }
        | Statement::Permission { .. }
        | Statement::Break { .. }
        | Statement::Continue { .. }
        | Statement::TypeAlias { .. }
        | Statement::Impl { .. }
        | Statement::Protocol { .. }
        | Statement::Error { .. } => {}
        Statement::Yield { value, .. } => collect_expr_idents(value, used),
        Statement::Nursery { body, .. } => {
            for s in body { collect_stmt_used(s, used); }
        }
        Statement::NamedError { message, .. } => collect_expr_idents(message, used),
    }
}

/// Collect all `Identifier` names referenced within an expression (recursively).
fn collect_expr_idents(expr: &Expression, used: &mut HashSet<String>) {
    match expr {
        Expression::Identifier(name) => {
            used.insert(name.clone());
        }
        Expression::BinaryOp { left, right, .. } => {
            collect_expr_idents(left, used);
            collect_expr_idents(right, used);
        }
        Expression::UnaryOp { operand, .. } => collect_expr_idents(operand, used),
        Expression::FunctionCall { arguments, .. } => {
            for arg in arguments {
                collect_expr_idents(arg, used);
            }
        }
        Expression::Array { elements, .. } => {
            for e in elements {
                collect_expr_idents(e, used);
            }
        }
        Expression::Map { entries, .. } => {
            for (k, v) in entries {
                collect_expr_idents(k, used);
                collect_expr_idents(v, used);
            }
        }
        Expression::Set { elements, .. } => {
            for e in elements {
                collect_expr_idents(e, used);
            }
        }
        Expression::Index { target, index, .. } => {
            collect_expr_idents(target, used);
            collect_expr_idents(index, used);
        }
        Expression::Member { target, .. } | Expression::OptionalMember { target, .. } => {
            collect_expr_idents(target, used);
        }
        Expression::Lambda { body, .. } => collect_expr_idents(body, used),
        Expression::Ternary {
            condition,
            true_expr,
            false_expr,
            ..
        } => {
            collect_expr_idents(condition, used);
            collect_expr_idents(true_expr, used);
            collect_expr_idents(false_expr, used);
        }
        Expression::Slice {
            target,
            start,
            end,
            step,
            ..
        } => {
            collect_expr_idents(target, used);
            if let Some(s) = start {
                collect_expr_idents(s, used);
            }
            if let Some(e) = end {
                collect_expr_idents(e, used);
            }
            if let Some(st) = step {
                collect_expr_idents(st, used);
            }
        }
        Expression::InterpolatedString { segments, .. } => {
            for seg in segments {
                if let InterpolatedSegment::Expression(e) = seg {
                    collect_expr_idents(e, used);
                }
            }
        }
        Expression::Await { expression, .. } => collect_expr_idents(expression, used),
        Expression::OptionalCall {
            target, arguments, ..
        } => {
            collect_expr_idents(target, used);
            for arg in arguments {
                collect_expr_idents(arg, used);
            }
        }
        Expression::OptionalIndex { target, index, .. } => {
            collect_expr_idents(target, used);
            collect_expr_idents(index, used);
        }
        Expression::MethodCall {
            object, arguments, ..
        } => {
            collect_expr_idents(object, used);
            for arg in arguments {
                collect_expr_idents(arg, used);
            }
        }
        Expression::StructLiteral { fields, .. } => {
            for (_, field_expr) in fields {
                collect_expr_idents(field_expr, used);
            }
        }
        Expression::Spread { value, .. } => {
            collect_expr_idents(value, used);
        }
        Expression::Propagate { value, .. } => {
            collect_expr_idents(value, used);
        }
        Expression::Literal(_) => {}
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Check 2: Unreachable code
// ────────────────────────────────────────────────────────────────────────────

fn check_unreachable_code(program: &Program) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    check_stmts_for_unreachable(&program.statements, &mut issues);
    issues
}

fn check_stmts_for_unreachable(stmts: &[Statement], issues: &mut Vec<LintIssue>) {
    let mut terminated = false;
    for stmt in stmts {
        if terminated {
            issues.push(LintIssue {
                line: stmt_line(stmt),
                column: 1,
                message: "Unreachable code after return/break/continue".to_string(),
                severity: Severity::Warning,
                rule_id: None,
            });
            // Report only the first unreachable statement per block
            break;
        }
        if is_unconditional_terminator(stmt) {
            terminated = true;
        }
        // Recurse into nested blocks
        recurse_stmts(stmt, issues);
    }
}

fn is_unconditional_terminator(stmt: &Statement) -> bool {
    matches!(
        stmt,
        Statement::Return { .. } | Statement::Break { .. } | Statement::Continue { .. }
    )
}

fn recurse_stmts(stmt: &Statement, issues: &mut Vec<LintIssue>) {
    match stmt {
        Statement::FunctionDef { body, .. } => {
            check_stmts_for_unreachable(body, issues);
        }
        Statement::If {
            then_branch,
            else_if_branches,
            else_branch,
            ..
        } => {
            check_stmts_for_unreachable(then_branch, issues);
            for (_, branch) in else_if_branches {
                check_stmts_for_unreachable(branch, issues);
            }
            if let Some(branch) = else_branch {
                check_stmts_for_unreachable(branch, issues);
            }
        }
        Statement::While { body, .. }
        | Statement::For { body, .. }
        | Statement::Repeat { body, .. } => {
            check_stmts_for_unreachable(body, issues);
        }
        Statement::DoWhile { body, .. } => {
            check_stmts_for_unreachable(body, issues);
        }
        Statement::Try {
            body,
            catch,
            finally,
            ..
        } => {
            check_stmts_for_unreachable(body, issues);
            if let Some((_, catch_body)) = catch {
                check_stmts_for_unreachable(catch_body, issues);
            }
            if let Some(finally_body) = finally {
                check_stmts_for_unreachable(finally_body, issues);
            }
        }
        Statement::Match { cases, default, .. } => {
            for (_, _, case_body) in cases {
                check_stmts_for_unreachable(case_body, issues);
            }
            if let Some(default_body) = default {
                check_stmts_for_unreachable(default_body, issues);
            }
        }
        _ => {}
    }
}

fn stmt_line(stmt: &Statement) -> usize {
    match stmt {
        Statement::Assignment { span, .. }
        | Statement::IndexAssignment { span, .. }
        | Statement::CompoundAssignment { span, .. }
        | Statement::FunctionDef { span, .. }
        | Statement::Return { span, .. }
        | Statement::Break { span }
        | Statement::Continue { span }
        | Statement::If { span, .. }
        | Statement::While { span, .. }
        | Statement::DoWhile { span, .. }
        | Statement::For { span, .. }
        | Statement::Repeat { span, .. }
        | Statement::Assert { span, .. }
        | Statement::Enum { span, .. }
        | Statement::Struct { span, .. }
        | Statement::Match { span, .. }
        | Statement::Try { span, .. }
        | Statement::Import { span, .. }
        | Statement::Export { span, .. }
        | Statement::Const { span, .. }
        | Statement::Permission { span, .. }
        | Statement::TypeAlias { span, .. }
        | Statement::NamedError { span, .. }
        | Statement::Impl { span, .. }
        | Statement::Yield { span, .. }
        | Statement::Nursery { span, .. }
        | Statement::Protocol { span, .. }
        | Statement::Error { span, .. } => span.line,
        Statement::Expression(e) => expr_span_line(e),
    }
}

fn expr_span_line(expr: &Expression) -> usize {
    match expr {
        Expression::BinaryOp { span, .. }
        | Expression::UnaryOp { span, .. }
        | Expression::FunctionCall { span, .. }
        | Expression::Array { span, .. }
        | Expression::Map { span, .. }
        | Expression::Set { span, .. }
        | Expression::Index { span, .. }
        | Expression::Member { span, .. }
        | Expression::Lambda { span, .. }
        | Expression::Ternary { span, .. }
        | Expression::Slice { span, .. }
        | Expression::InterpolatedString { span, .. }
        | Expression::Await { span, .. }
        | Expression::OptionalMember { span, .. }
        | Expression::OptionalCall { span, .. }
        | Expression::OptionalIndex { span, .. }
        | Expression::MethodCall { span, .. }
        | Expression::StructLiteral { span, .. }
        | Expression::Spread { span, .. }
        | Expression::Propagate { span, .. } => span.line,
        Expression::Literal(_) | Expression::Identifier(_) => 1,
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Check 3: Duplicate map keys
// ────────────────────────────────────────────────────────────────────────────

fn check_duplicate_map_keys(program: &Program) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    for stmt in &program.statements {
        check_stmt_map_keys(stmt, &mut issues);
    }
    issues
}

fn check_stmt_map_keys(stmt: &Statement, issues: &mut Vec<LintIssue>) {
    match stmt {
        Statement::Expression(e) => check_expr_map_keys(e, issues),
        Statement::Assignment { value, .. } => check_expr_map_keys(value, issues),
        Statement::Return { value: Some(v), .. } => check_expr_map_keys(v, issues),
        Statement::FunctionDef { body, .. } => {
            for s in body {
                check_stmt_map_keys(s, issues);
            }
        }
        Statement::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
            ..
        } => {
            check_expr_map_keys(condition, issues);
            for s in then_branch {
                check_stmt_map_keys(s, issues);
            }
            for (cond, branch) in else_if_branches {
                check_expr_map_keys(cond, issues);
                for s in branch {
                    check_stmt_map_keys(s, issues);
                }
            }
            if let Some(branch) = else_branch {
                for s in branch {
                    check_stmt_map_keys(s, issues);
                }
            }
        }
        Statement::While {
            condition, body, ..
        }
        | Statement::For {
            iterable: condition,
            body,
            ..
        } => {
            check_expr_map_keys(condition, issues);
            for s in body {
                check_stmt_map_keys(s, issues);
            }
        }
        _ => {}
    }
}

fn check_expr_map_keys(expr: &Expression, issues: &mut Vec<LintIssue>) {
    if let Expression::Map { entries, span } = expr {
        let mut seen: HashSet<String> = HashSet::new();
        for (key_expr, val_expr) in entries {
            if let Some(key_str) = literal_to_string(key_expr) {
                if !seen.insert(key_str.clone()) {
                    issues.push(LintIssue {
                        line: span.line,
                        column: span.column,
                        message: format!("Duplicate map key '{}'", key_str),
                        severity: Severity::Error,
                        rule_id: None,
                    });
                }
            }
            // Recurse into value
            check_expr_map_keys(val_expr, issues);
        }
    }
    // Recurse into sub-expressions
    match expr {
        Expression::Map { .. } => {} // already handled above
        Expression::Array { elements, .. } => {
            for e in elements {
                check_expr_map_keys(e, issues);
            }
        }
        Expression::BinaryOp { left, right, .. } => {
            check_expr_map_keys(left, issues);
            check_expr_map_keys(right, issues);
        }
        Expression::FunctionCall { arguments, .. } => {
            for a in arguments {
                check_expr_map_keys(a, issues);
            }
        }
        Expression::Ternary {
            condition,
            true_expr,
            false_expr,
            ..
        } => {
            check_expr_map_keys(condition, issues);
            check_expr_map_keys(true_expr, issues);
            check_expr_map_keys(false_expr, issues);
        }
        _ => {}
    }
}

fn literal_to_string(expr: &Expression) -> Option<String> {
    match expr {
        Expression::Literal(Literal::String(s)) => Some(s.clone()),
        Expression::Literal(Literal::Integer(n)) => Some(n.to_string()),
        Expression::Literal(Literal::Boolean(b)) => Some(b.to_string()),
        _ => None,
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Check 4: Suspicious comparisons
// ────────────────────────────────────────────────────────────────────────────

fn check_suspicious_comparisons(program: &Program) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    for stmt in &program.statements {
        check_stmt_suspicious(stmt, &mut issues);
    }
    issues
}

fn check_stmt_suspicious(stmt: &Statement, issues: &mut Vec<LintIssue>) {
    match stmt {
        Statement::Expression(e) => check_expr_suspicious(e, issues),
        Statement::Assignment { value, .. } => check_expr_suspicious(value, issues),
        Statement::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
            ..
        } => {
            check_expr_suspicious(condition, issues);
            for s in then_branch {
                check_stmt_suspicious(s, issues);
            }
            for (cond, branch) in else_if_branches {
                check_expr_suspicious(cond, issues);
                for s in branch {
                    check_stmt_suspicious(s, issues);
                }
            }
            if let Some(branch) = else_branch {
                for s in branch {
                    check_stmt_suspicious(s, issues);
                }
            }
        }
        Statement::While {
            condition, body, ..
        } => {
            check_expr_suspicious(condition, issues);
            for s in body {
                check_stmt_suspicious(s, issues);
            }
        }
        Statement::FunctionDef { body, .. } => {
            for s in body {
                check_stmt_suspicious(s, issues);
            }
        }
        Statement::Return { value: Some(v), .. } => check_expr_suspicious(v, issues),
        _ => {}
    }
}

fn check_expr_suspicious(expr: &Expression, issues: &mut Vec<LintIssue>) {
    if let Expression::BinaryOp {
        left,
        op,
        right,
        span,
    } = expr
    {
        match op {
            BinaryOperator::Equal | BinaryOperator::NotEqual => {
                let op_str = if *op == BinaryOperator::Equal {
                    "=="
                } else {
                    "!="
                };
                let eq = *op == BinaryOperator::Equal;

                // x == true  → use `x` instead
                // x == false → use `not x` instead
                if let Expression::Literal(Literal::Boolean(b)) = right.as_ref() {
                    let suggestion = if *b == eq {
                        "use the expression directly"
                    } else {
                        "use `not expression` instead"
                    };
                    issues.push(LintIssue {
                        line: span.line,
                        column: span.column,
                        message: format!(
                            "Suspicious comparison with boolean literal ({} {}): {}",
                            op_str, b, suggestion
                        ),
                        severity: Severity::Warning,
                        rule_id: None,
                    });
                }
                // true == x  (reversed)
                if let Expression::Literal(Literal::Boolean(b)) = left.as_ref() {
                    let suggestion = if *b == eq {
                        "use the expression directly"
                    } else {
                        "use `not expression` instead"
                    };
                    issues.push(LintIssue {
                        line: span.line,
                        column: span.column,
                        message: format!(
                            "Suspicious comparison with boolean literal ({} {}): {}",
                            op_str, b, suggestion
                        ),
                        severity: Severity::Warning,
                        rule_id: None,
                    });
                }
                // x == x (comparing expression to itself)
                if left == right {
                    issues.push(LintIssue {
                        line: span.line,
                        column: span.column,
                        message: "Suspicious self-comparison: both sides are identical".to_string(),
                        severity: Severity::Warning,
                        rule_id: None,
                    });
                }
            }
            _ => {}
        }
        // Recurse
        check_expr_suspicious(left, issues);
        check_expr_suspicious(right, issues);
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Check 5: Recursion depth risk
// ────────────────────────────────────────────────────────────────────────────

fn check_recursion_risk(program: &Program) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    for stmt in &program.statements {
        if let Statement::FunctionDef {
            name, body, span, ..
        } = stmt
        {
            if body_calls_function(body, name) {
                issues.push(LintIssue {
                    line: span.line,
                    column: 1,
                    message: format!(
                        "Function '{}' calls itself recursively — ensure a base case exists to avoid stack overflow",
                        name
                    ),
                    severity: Severity::Warning,
                    rule_id: None,
                });
            }
        }
    }
    issues
}

/// Returns true if any expression in `stmts` directly calls `func_name`.
fn body_calls_function(stmts: &[Statement], func_name: &str) -> bool {
    stmts.iter().any(|s| stmt_calls(s, func_name))
}

fn stmt_calls(stmt: &Statement, func_name: &str) -> bool {
    match stmt {
        Statement::Expression(e) => expr_calls(e, func_name),
        Statement::Assignment { value, .. } => expr_calls(value, func_name),
        Statement::Return { value: Some(v), .. } => expr_calls(v, func_name),
        Statement::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
            ..
        } => {
            expr_calls(condition, func_name)
                || body_calls_function(then_branch, func_name)
                || else_if_branches
                    .iter()
                    .any(|(c, b)| expr_calls(c, func_name) || body_calls_function(b, func_name))
                || else_branch
                    .as_ref()
                    .is_some_and(|b| body_calls_function(b, func_name))
        }
        Statement::While {
            condition, body, ..
        }
        | Statement::For {
            iterable: condition,
            body,
            ..
        } => expr_calls(condition, func_name) || body_calls_function(body, func_name),
        Statement::Repeat { count, body, .. } => {
            expr_calls(count, func_name) || body_calls_function(body, func_name)
        }
        Statement::Try {
            body,
            catch,
            finally,
            ..
        } => {
            body_calls_function(body, func_name)
                || catch
                    .as_ref()
                    .is_some_and(|(_, b)| body_calls_function(b, func_name))
                || finally
                    .as_ref()
                    .is_some_and(|b| body_calls_function(b, func_name))
        }
        _ => false,
    }
}

fn expr_calls(expr: &Expression, func_name: &str) -> bool {
    match expr {
        Expression::FunctionCall {
            name, arguments, ..
        } => name == func_name || arguments.iter().any(|a| expr_calls(a, func_name)),
        Expression::BinaryOp { left, right, .. } => {
            expr_calls(left, func_name) || expr_calls(right, func_name)
        }
        Expression::UnaryOp { operand, .. } => expr_calls(operand, func_name),
        Expression::Ternary {
            condition,
            true_expr,
            false_expr,
            ..
        } => {
            expr_calls(condition, func_name)
                || expr_calls(true_expr, func_name)
                || expr_calls(false_expr, func_name)
        }
        Expression::Array { elements, .. } => elements.iter().any(|e| expr_calls(e, func_name)),
        Expression::Map { entries, .. } => entries
            .iter()
            .any(|(k, v)| expr_calls(k, func_name) || expr_calls(v, func_name)),
        _ => false,
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Check 6: Import not found
// ────────────────────────────────────────────────────────────────────────────

/// Standard library module names that are always resolvable.
const STDLIB_MODULES: &[&str] = &[
    "core",
    "io",
    "net",
    "crypto",
    "sys",
    "time",
    "json",
    "log",
    "regex",
    "url",
    "path",
    "capabilities",
    "math",
    "string",
    "collections",
    "test",
];

fn check_imports_exist(program: &Program, file_path: Option<&Path>) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    let base_dir = file_path.and_then(|p| p.parent());

    for stmt in &program.statements {
        if let Statement::Import {
            modules,
            from,
            span,
            ..
        } = stmt
        {
            // `from "module" import name1, name2`
            if let Some(from_path) = from {
                if !is_stdlib(from_path) && !resolve_module(from_path, base_dir) {
                    issues.push(LintIssue {
                        line: span.line,
                        column: span.column,
                        message: format!("Module '{}' not found", from_path),
                        severity: Severity::Error,
                        rule_id: None,
                    });
                }
            } else {
                // `import module1, module2`
                for module in modules {
                    if !is_stdlib(module) && !resolve_module(module, base_dir) {
                        issues.push(LintIssue {
                            line: span.line,
                            column: span.column,
                            message: format!("Module '{}' not found", module),
                            severity: Severity::Error,
                            rule_id: None,
                        });
                    }
                }
            }
        }
    }
    issues
}

fn is_stdlib(name: &str) -> bool {
    STDLIB_MODULES.contains(&name)
}

fn resolve_module(name: &str, base_dir: Option<&Path>) -> bool {
    let base = match base_dir {
        Some(b) => b,
        None => return false,
    };
    // Try <name>.tc or <name>/ (directory with main.tc)
    base.join(format!("{}.tc", name)).exists() || base.join(name).join("main.tc").exists()
}

// ────────────────────────────────────────────────────────────────────────────
// Helper: build a LintIssue with an explicit rule ID
// ────────────────────────────────────────────────────────────────────────────

fn lint_issue(line: usize, col: usize, msg: impl Into<String>, sev: Severity, rule: &str) -> LintIssue {
    LintIssue {
        line,
        column: col,
        message: msg.into(),
        severity: sev,
        rule_id: Some(rule.to_string()),
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Rule L010: Division or modulo by literal zero
// ────────────────────────────────────────────────────────────────────────────

fn check_division_by_zero_literal(program: &Program) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    for stmt in &program.statements {
        collect_div_zero_stmt(stmt, &mut issues);
    }
    issues
}

fn collect_div_zero_stmt(stmt: &Statement, issues: &mut Vec<LintIssue>) {
    match stmt {
        Statement::Expression(e) | Statement::Return { value: Some(e), .. } | Statement::Assignment { value: e, .. } => {
            collect_div_zero_expr(e, issues);
        }
        Statement::FunctionDef { body, .. } | Statement::While { body, .. } | Statement::For { body, .. }
        | Statement::Repeat { body, .. } | Statement::DoWhile { body, .. } => {
            for s in body { collect_div_zero_stmt(s, issues); }
        }
        Statement::If { condition, then_branch, else_if_branches, else_branch, .. } => {
            collect_div_zero_expr(condition, issues);
            for s in then_branch { collect_div_zero_stmt(s, issues); }
            for (c, b) in else_if_branches {
                collect_div_zero_expr(c, issues);
                for s in b { collect_div_zero_stmt(s, issues); }
            }
            if let Some(b) = else_branch { for s in b { collect_div_zero_stmt(s, issues); } }
        }
        _ => {}
    }
}

fn collect_div_zero_expr(expr: &Expression, issues: &mut Vec<LintIssue>) {
    if let Expression::BinaryOp { left, op, right, span } = expr {
        let is_div_or_mod = matches!(op, BinaryOperator::Divide | BinaryOperator::Modulo);
        if is_div_or_mod {
            if matches!(right.as_ref(), Expression::Literal(Literal::Integer(0))) {
                let op_str = if matches!(op, BinaryOperator::Divide) { "/" } else { "%" };
                issues.push(lint_issue(span.line, span.column,
                    format!("Division by zero literal: `x {} 0` will always raise a runtime error", op_str),
                    Severity::Warning, "L010"));
            }
        }
        collect_div_zero_expr(left, issues);
        collect_div_zero_expr(right, issues);
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Rule L011: Unreachable match arm after wildcard pattern
// ────────────────────────────────────────────────────────────────────────────

fn check_unreachable_match_arm(program: &Program) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    for stmt in &program.statements {
        collect_unreachable_match_stmt(stmt, &mut issues);
    }
    issues
}

fn collect_unreachable_match_stmt(stmt: &Statement, issues: &mut Vec<LintIssue>) {
    match stmt {
        Statement::Match { cases, span, .. } => {
            let mut saw_wildcard = false;
            for (i, (pattern, _guard, _body)) in cases.iter().enumerate() {
                if saw_wildcard {
                    issues.push(lint_issue(span.line + i, 1,
                        "Unreachable match arm: pattern after `_` wildcard can never be matched",
                        Severity::Warning, "L011"));
                }
                if matches!(pattern, Pattern::Ignore)
                    || matches!(pattern, Pattern::Identifier(s) if s == "_") {
                    saw_wildcard = true;
                }
            }
        }
        Statement::FunctionDef { body, .. } | Statement::While { body, .. } | Statement::For { body, .. } => {
            for s in body { collect_unreachable_match_stmt(s, issues); }
        }
        _ => {}
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Rule L012: Empty catch block silently swallows errors
// ────────────────────────────────────────────────────────────────────────────

fn check_empty_catch(program: &Program) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    for stmt in &program.statements {
        collect_empty_catch_stmt(stmt, &mut issues);
    }
    issues
}

fn collect_empty_catch_stmt(stmt: &Statement, issues: &mut Vec<LintIssue>) {
    match stmt {
        Statement::Try { catch, span, body, finally, .. } => {
            if let Some((_var, catch_body)) = catch {
                if catch_body.is_empty() {
                    issues.push(lint_issue(span.line, 1,
                        "Empty catch block silently swallows errors — add error handling or re-raise",
                        Severity::Warning, "L012"));
                }
                for s in catch_body { collect_empty_catch_stmt(s, issues); }
            }
            for s in body { collect_empty_catch_stmt(s, issues); }
            if let Some(fb) = finally { for s in fb { collect_empty_catch_stmt(s, issues); } }
        }
        Statement::FunctionDef { body, .. } | Statement::While { body, .. } | Statement::For { body, .. } => {
            for s in body { collect_empty_catch_stmt(s, issues); }
        }
        _ => {}
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Rule L013: Function defined but never called
// ────────────────────────────────────────────────────────────────────────────

fn check_unused_functions(program: &Program) -> Vec<LintIssue> {
    let mut defined: HashMap<String, usize> = HashMap::new();
    let mut called: HashSet<String> = HashSet::new();

    for stmt in &program.statements {
        if let Statement::FunctionDef { name, span, .. } = stmt {
            if !name.starts_with('_') && !name.starts_with("__") {
                defined.insert(name.clone(), span.line);
            }
        }
    }
    for stmt in &program.statements {
        collect_called_functions(stmt, &mut called);
    }

    let mut issues = Vec::new();
    for (name, line) in &defined {
        if !called.contains(name) {
            issues.push(lint_issue(*line, 1,
                format!("Function '{}' is defined but never called", name),
                Severity::Warning, "L013"));
        }
    }
    issues
}

fn collect_called_functions(stmt: &Statement, called: &mut HashSet<String>) {
    match stmt {
        Statement::Expression(e) | Statement::Assignment { value: e, .. }
        | Statement::Return { value: Some(e), .. } => collect_expr_calls_to_set(e, called),
        Statement::FunctionDef { body, .. } | Statement::While { body, .. }
        | Statement::For { body, .. } | Statement::Repeat { body, .. } | Statement::DoWhile { body, .. } => {
            for s in body { collect_called_functions(s, called); }
        }
        Statement::If { condition, then_branch, else_if_branches, else_branch, .. } => {
            collect_expr_calls_to_set(condition, called);
            for s in then_branch { collect_called_functions(s, called); }
            for (c, b) in else_if_branches {
                collect_expr_calls_to_set(c, called);
                for s in b { collect_called_functions(s, called); }
            }
            if let Some(b) = else_branch { for s in b { collect_called_functions(s, called); } }
        }
        _ => {}
    }
}

fn collect_expr_calls_to_set(expr: &Expression, called: &mut HashSet<String>) {
    match expr {
        Expression::FunctionCall { name, arguments, .. } => {
            called.insert(name.clone());
            for a in arguments { collect_expr_calls_to_set(a, called); }
        }
        Expression::BinaryOp { left, right, .. } => {
            collect_expr_calls_to_set(left, called);
            collect_expr_calls_to_set(right, called);
        }
        Expression::UnaryOp { operand, .. } => collect_expr_calls_to_set(operand, called),
        Expression::Array { elements, .. } => {
            for e in elements { collect_expr_calls_to_set(e, called); }
        }
        Expression::Ternary { condition, true_expr, false_expr, .. } => {
            collect_expr_calls_to_set(condition, called);
            collect_expr_calls_to_set(true_expr, called);
            collect_expr_calls_to_set(false_expr, called);
        }
        _ => {}
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Rule L014: Function with declared return type but last statement is not return
// ────────────────────────────────────────────────────────────────────────────

fn check_missing_return_type(program: &Program) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    for stmt in &program.statements {
        if let Statement::FunctionDef { name, return_type, body, span, .. } = stmt {
            if return_type.is_some() && !body.is_empty() {
                let last = body.last().unwrap();
                if !matches!(last, Statement::Return { .. }) {
                    issues.push(lint_issue(span.line, 1,
                        format!("Function '{}' has a return type annotation but may not return a value on all paths", name),
                        Severity::Warning, "L014"));
                }
            }
        }
    }
    issues
}

// ────────────────────────────────────────────────────────────────────────────
// Rule L015: Negated condition without compound — suggest simplifying
// ────────────────────────────────────────────────────────────────────────────

fn check_negated_condition(program: &Program) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    for stmt in &program.statements {
        collect_negated_cond_stmt(stmt, &mut issues);
    }
    issues
}

fn collect_negated_cond_stmt(stmt: &Statement, issues: &mut Vec<LintIssue>) {
    match stmt {
        Statement::If { condition, then_branch, else_if_branches, else_branch, span, .. } => {
            if is_simple_negation(condition) && else_branch.is_none() && else_if_branches.is_empty() {
                issues.push(lint_issue(span.line, 1,
                    "Negated condition without else branch — consider inverting the condition for clarity",
                    Severity::Info, "L015"));
            }
            for s in then_branch { collect_negated_cond_stmt(s, issues); }
        }
        Statement::FunctionDef { body, .. } | Statement::While { body, .. }
        | Statement::For { body, .. } => {
            for s in body { collect_negated_cond_stmt(s, issues); }
        }
        _ => {}
    }
}

fn is_simple_negation(expr: &Expression) -> bool {
    matches!(expr, Expression::UnaryOp { op: UnaryOperator::Not, .. })
}

// ────────────────────────────────────────────────────────────────────────────
// Rule L016: String concatenation in loop (`s = s + item` pattern)
// ────────────────────────────────────────────────────────────────────────────

fn check_string_concat_in_loop(program: &Program) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    for stmt in &program.statements {
        collect_str_concat_stmt(stmt, false, &mut issues);
    }
    issues
}

fn collect_str_concat_stmt(stmt: &Statement, in_loop: bool, issues: &mut Vec<LintIssue>) {
    match stmt {
        Statement::While { body, .. } | Statement::For { body, .. }
        | Statement::Repeat { body, .. } | Statement::DoWhile { body, .. } => {
            for s in body { collect_str_concat_stmt(s, true, issues); }
        }
        Statement::Assignment { pattern, value, span, .. } if in_loop => {
            for bound_name in collect_pattern_names(pattern) {
                if is_self_concat(value, &bound_name) {
                    issues.push(lint_issue(span.line, span.column,
                        format!("String concatenation in loop ('{}' = '{}' + ...): consider building an array and using join()", bound_name, bound_name),
                        Severity::Warning, "L016"));
                }
            }
        }
        Statement::CompoundAssignment { name, op: BinaryOperator::Add, span, .. } if in_loop => {
            issues.push(lint_issue(span.line, span.column,
                format!("String concatenation in loop ('{}' += ...): consider building an array and using join()", name),
                Severity::Warning, "L016"));
        }
        Statement::FunctionDef { body, .. } => {
            for s in body { collect_str_concat_stmt(s, false, issues); }
        }
        Statement::If { then_branch, else_if_branches, else_branch, .. } if in_loop => {
            for s in then_branch { collect_str_concat_stmt(s, in_loop, issues); }
            for (_, b) in else_if_branches { for s in b { collect_str_concat_stmt(s, in_loop, issues); } }
            if let Some(b) = else_branch { for s in b { collect_str_concat_stmt(s, in_loop, issues); } }
        }
        _ => {}
    }
}

fn is_self_concat(expr: &Expression, name: &str) -> bool {
    if let Expression::BinaryOp { left, op: BinaryOperator::Add, .. } = expr {
        if let Expression::Identifier(id) = left.as_ref() {
            return id == name;
        }
    }
    false
}

// ────────────────────────────────────────────────────────────────────────────
// Rule L017: Null comparison (`x == null`) — prefer optional chain
// ────────────────────────────────────────────────────────────────────────────

fn check_null_comparison(program: &Program) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    for stmt in &program.statements {
        collect_null_cmp_stmt(stmt, &mut issues);
    }
    issues
}

fn collect_null_cmp_stmt(stmt: &Statement, issues: &mut Vec<LintIssue>) {
    match stmt {
        Statement::If { condition, then_branch, else_if_branches, else_branch, .. } => {
            collect_null_cmp_expr(condition, issues);
            for s in then_branch { collect_null_cmp_stmt(s, issues); }
            for (c, b) in else_if_branches {
                collect_null_cmp_expr(c, issues);
                for s in b { collect_null_cmp_stmt(s, issues); }
            }
            if let Some(b) = else_branch { for s in b { collect_null_cmp_stmt(s, issues); } }
        }
        Statement::FunctionDef { body, .. } | Statement::While { body, .. }
        | Statement::For { body, .. } => {
            for s in body { collect_null_cmp_stmt(s, issues); }
        }
        _ => {}
    }
}

fn collect_null_cmp_expr(expr: &Expression, issues: &mut Vec<LintIssue>) {
    if let Expression::BinaryOp { left, op, right, span } = expr {
        if matches!(op, BinaryOperator::Equal | BinaryOperator::NotEqual) {
            let is_null_cmp = matches!(right.as_ref(), Expression::Literal(Literal::Null))
                || matches!(left.as_ref(), Expression::Literal(Literal::Null));
            if is_null_cmp {
                issues.push(lint_issue(span.line, span.column,
                    "Null comparison: prefer `x?.field` optional chain or `is_null(x)` over `x == null`",
                    Severity::Info, "L017"));
            }
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Rule L018: Overly broad permission grant
// ────────────────────────────────────────────────────────────────────────────

fn check_broad_permission(program: &Program) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    for stmt in &program.statements {
        collect_broad_perm_stmt(stmt, &mut issues);
    }
    issues
}

fn collect_broad_perm_stmt(stmt: &Statement, issues: &mut Vec<LintIssue>) {
    match stmt {
        Statement::Expression(e) | Statement::Assignment { value: e, .. } => {
            collect_broad_perm_expr(e, issues);
        }
        Statement::FunctionDef { body, .. } | Statement::While { body, .. }
        | Statement::For { body, .. } => {
            for s in body { collect_broad_perm_stmt(s, issues); }
        }
        _ => {}
    }
}

fn collect_broad_perm_expr(expr: &Expression, issues: &mut Vec<LintIssue>) {
    if let Expression::FunctionCall { name, arguments, span, .. } = expr {
        if name == "grant_permission" || name == "allow_permission" {
            if let [Expression::Literal(Literal::String(perm))] = arguments.as_slice() {
                let broad = matches!(perm.as_str(), "fs" | "net" | "sys" | "exec" | "process");
                if broad {
                    issues.push(lint_issue(span.line, span.column,
                        format!("Overly broad permission '{}' granted without a specific scope — prefer `grant_permission(\"{}\", path)` to limit access", perm, perm),
                        Severity::Warning, "L018"));
                }
            }
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Rule L019: Match statement with no wildcard arm (potentially non-exhaustive)
// ────────────────────────────────────────────────────────────────────────────

fn check_match_no_wildcard(program: &Program) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    for stmt in &program.statements {
        collect_match_no_wildcard_stmt(stmt, &mut issues);
    }
    issues
}

fn collect_match_no_wildcard_stmt(stmt: &Statement, issues: &mut Vec<LintIssue>) {
    match stmt {
        Statement::Match { cases, default, span, .. } => {
            let is_wildcard = |p: &Pattern| matches!(p, Pattern::Ignore)
                || matches!(p, Pattern::Identifier(s) if s == "_");
            let has_wildcard = cases.iter().any(|(p, _, _)| is_wildcard(p))
                || default.is_some();
            if !has_wildcard && !cases.is_empty() {
                issues.push(lint_issue(span.line, 1,
                    "Match statement has no wildcard `_` arm — may be non-exhaustive; add `case _` to handle unexpected values",
                    Severity::Warning, "L019"));
            }
            for s in cases.iter().flat_map(|(_, _, b)| b.iter()) {
                collect_match_no_wildcard_stmt(s, issues);
            }
        }
        Statement::FunctionDef { body, .. } | Statement::While { body, .. }
        | Statement::For { body, .. } => {
            for s in body { collect_match_no_wildcard_stmt(s, issues); }
        }
        _ => {}
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Check 8: Mutable globals (original was here — now below)
// Style checks
// ────────────────────────────────────────────────────────────────────────────

// ────────────────────────────────────────────────────────────────────────────
// Check 7: Shadowed variables
// A local assignment inside a function that reuses a parameter name.
// ────────────────────────────────────────────────────────────────────────────

fn check_shadowed_variables(program: &Program) -> Vec<LintIssue> {
    let mut issues = Vec::new();

    // Collect top-level names (constants, functions, top-level stores)
    let mut top_names: HashSet<String> = HashSet::new();
    for stmt in &program.statements {
        match stmt {
            Statement::Assignment { pattern, .. } => {
                for n in collect_pattern_names(pattern) {
                    top_names.insert(n);
                }
            }
            Statement::Const { name, .. } | Statement::FunctionDef { name, .. } => {
                top_names.insert(name.clone());
            }
            _ => {}
        }
    }

    for stmt in &program.statements {
        if let Statement::FunctionDef {
            params, body, span, ..
        } = stmt
        {
            let param_names: HashSet<String> = params.iter().map(|p| p.name.clone()).collect();

            // Check if any local assignment in the body shadows a parameter
            check_body_for_shadowing(body, &param_names, span.line, &mut issues);
        }
    }

    issues
}

fn check_body_for_shadowing(
    stmts: &[Statement],
    outer: &HashSet<String>,
    fn_line: usize,
    issues: &mut Vec<LintIssue>,
) {
    for stmt in stmts {
        if let Statement::Assignment { pattern, span, .. } = stmt {
            for name in collect_pattern_names(pattern) {
                if outer.contains(&name) && !name.starts_with('_') {
                    issues.push(LintIssue {
                        line: span.line,
                        column: span.column,
                        message: format!(
                            "Variable '{}' shadows a parameter defined at line {}",
                            name, fn_line
                        ),
                        severity: Severity::Warning,
                        rule_id: None,
                    });
                }
            }
        }
        recurse_stmts_for_shadowing(stmt, outer, fn_line, issues);
    }
}

fn recurse_stmts_for_shadowing(
    stmt: &Statement,
    outer: &HashSet<String>,
    fn_line: usize,
    issues: &mut Vec<LintIssue>,
) {
    match stmt {
        Statement::If {
            then_branch,
            else_if_branches,
            else_branch,
            ..
        } => {
            check_body_for_shadowing(then_branch, outer, fn_line, issues);
            for (_, b) in else_if_branches {
                check_body_for_shadowing(b, outer, fn_line, issues);
            }
            if let Some(b) = else_branch {
                check_body_for_shadowing(b, outer, fn_line, issues);
            }
        }
        Statement::While { body, .. }
        | Statement::For { body, .. }
        | Statement::Repeat { body, .. }
        | Statement::DoWhile { body, .. } => {
            check_body_for_shadowing(body, outer, fn_line, issues);
        }
        Statement::Try {
            body,
            catch,
            finally,
            ..
        } => {
            check_body_for_shadowing(body, outer, fn_line, issues);
            if let Some((_, b)) = catch {
                check_body_for_shadowing(b, outer, fn_line, issues);
            }
            if let Some(b) = finally {
                check_body_for_shadowing(b, outer, fn_line, issues);
            }
        }
        _ => {}
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Check 8: Mutable globals
// A function body assigns to a top-level variable directly (not via a local).
// ────────────────────────────────────────────────────────────────────────────

fn check_mutable_globals(program: &Program) -> Vec<LintIssue> {
    let mut issues = Vec::new();

    // Collect all top-level assigned variable names
    let mut global_names: HashSet<String> = HashSet::new();
    for stmt in &program.statements {
        if let Statement::Assignment { pattern, .. } = stmt {
            for n in collect_pattern_names(pattern) {
                global_names.insert(n);
            }
        }
    }

    if global_names.is_empty() {
        return issues;
    }

    for stmt in &program.statements {
        if let Statement::FunctionDef {
            name: fn_name,
            params,
            body,
            span,
            ..
        } = stmt
        {
            // Exclude param names and local assignments from the global check
            let local_names: HashSet<String> = params.iter().map(|p| p.name.clone()).collect();
            check_body_for_global_mutation(
                body,
                &global_names,
                &local_names,
                fn_name,
                span.line,
                &mut issues,
            );
        }
    }

    issues
}

fn check_body_for_global_mutation(
    stmts: &[Statement],
    globals: &HashSet<String>,
    locals: &HashSet<String>,
    fn_name: &str,
    fn_line: usize,
    issues: &mut Vec<LintIssue>,
) {
    // Track names first assigned locally in this block (not globals)
    let mut locally_defined: HashSet<String> = locals.clone();

    for stmt in stmts {
        match stmt {
            Statement::Assignment { pattern, span, .. } => {
                for name in collect_pattern_names(pattern) {
                    if globals.contains(&name)
                        && !locally_defined.contains(&name)
                        && !name.starts_with('_')
                    {
                        issues.push(LintIssue {
                            line: span.line,
                            column: span.column,
                            message: format!(
                                "Function '{}' (line {}) modifies global variable '{}' — prefer returning a new value",
                                fn_name, fn_line, name
                            ),
                            severity: Severity::Warning,
                            rule_id: None,
                        });
                    }
                    locally_defined.insert(name);
                }
            }
            Statement::CompoundAssignment { name, span, .. } => {
                if globals.contains(name)
                    && !locally_defined.contains(name)
                    && !name.starts_with('_')
                {
                    issues.push(LintIssue {
                        line: span.line,
                        column: span.column,
                        message: format!(
                            "Function '{}' (line {}) modifies global variable '{}' — prefer returning a new value",
                            fn_name, fn_line, name
                        ),
                        severity: Severity::Warning,
                        rule_id: None,
                    });
                }
            }
            Statement::If {
                then_branch,
                else_if_branches,
                else_branch,
                ..
            } => {
                check_body_for_global_mutation(
                    then_branch,
                    globals,
                    &locally_defined,
                    fn_name,
                    fn_line,
                    issues,
                );
                for (_, b) in else_if_branches {
                    check_body_for_global_mutation(
                        b,
                        globals,
                        &locally_defined,
                        fn_name,
                        fn_line,
                        issues,
                    );
                }
                if let Some(b) = else_branch {
                    check_body_for_global_mutation(
                        b,
                        globals,
                        &locally_defined,
                        fn_name,
                        fn_line,
                        issues,
                    );
                }
            }
            Statement::While { body, .. }
            | Statement::For { body, .. }
            | Statement::Repeat { body, .. }
            | Statement::DoWhile { body, .. } => {
                check_body_for_global_mutation(
                    body,
                    globals,
                    &locally_defined,
                    fn_name,
                    fn_line,
                    issues,
                );
            }
            _ => {}
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Style checks
// ────────────────────────────────────────────────────────────────────────────

fn check_style(source: &str) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    for (i, line) in source.lines().enumerate() {
        let line_num = i + 1;
        let char_count = line.chars().count();
        if char_count > 120 {
            issues.push(LintIssue {
                line: line_num,
                column: 121,
                message: format!("Line too long ({} characters, max 120)", char_count),
                severity: Severity::Info,
                rule_id: None,
            });
        }
        if line.ends_with(' ') || line.ends_with('\t') {
            issues.push(LintIssue {
                line: line_num,
                column: line.trim_end().chars().count() + 1,
                message: "Trailing whitespace".to_string(),
                severity: Severity::Info,
                rule_id: None,
            });
        }
    }
    issues
}

// ────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn lint(src: &str) -> Vec<LintIssue> {
        Linter::lint_source(src).expect("lint failed")
    }

    fn has_warning(issues: &[LintIssue], msg_contains: &str) -> bool {
        issues
            .iter()
            .any(|i| i.message.contains(msg_contains) && i.severity == Severity::Warning)
    }

    fn has_error(issues: &[LintIssue], msg_contains: &str) -> bool {
        issues
            .iter()
            .any(|i| i.message.contains(msg_contains) && i.severity == Severity::Error)
    }

    #[test]
    fn test_unused_variable() {
        let src = "store → unused → 42\nprint → 1";
        let issues = lint(src);
        assert!(
            has_warning(&issues, "Unused variable 'unused'"),
            "got: {:?}",
            issues
        );
    }

    #[test]
    fn test_used_variable_no_warn() {
        let src = "store → x → 42\nprint → x";
        let issues = lint(src);
        assert!(!has_warning(&issues, "Unused variable 'x'"));
    }

    #[test]
    fn test_unreachable_code() {
        let src = "define → f → ()\nreturn → 1\nprint → 2\nend";
        let issues = lint(src);
        assert!(has_warning(&issues, "Unreachable"), "got: {:?}", issues);
    }

    #[test]
    fn test_duplicate_map_key() {
        let src = r#"store → m → {"a": 1, "a": 2}"#;
        let issues = lint(src);
        assert!(
            has_error(&issues, "Duplicate map key 'a'"),
            "got: {:?}",
            issues
        );
    }

    #[test]
    fn test_no_duplicate_map_key() {
        let src = r#"store → m → {"a": 1, "b": 2}"#;
        let issues = lint(src);
        assert!(
            !has_error(&issues, "Duplicate map key"),
            "got: {:?}",
            issues
        );
    }

    #[test]
    fn test_suspicious_comparison_true() {
        let src = "if → x == true\nprint → x\nend";
        let issues = lint(src);
        assert!(
            has_warning(&issues, "Suspicious comparison"),
            "got: {:?}",
            issues
        );
    }

    #[test]
    fn test_recursion_risk() {
        let src = "define → fib → (n)\nreturn → fib(n)\nend";
        let issues = lint(src);
        assert!(
            has_warning(&issues, "calls itself recursively"),
            "got: {:?}",
            issues
        );
    }

    #[test]
    fn test_no_recursion_warn_non_recursive() {
        let src = "define → add → (a, b)\nreturn → a + b\nend";
        let issues = lint(src);
        assert!(!has_warning(&issues, "calls itself recursively"));
    }

    #[test]
    fn test_shadowed_variable() {
        // Local assignment shadows parameter 'x'
        let src = "define → f → (x)\nstore → x → 99\nreturn → x\nend";
        let issues = lint(src);
        assert!(
            has_warning(&issues, "shadows a parameter"),
            "got: {:?}",
            issues
        );
    }

    #[test]
    fn test_no_shadow_different_name() {
        let src = "define → f → (x)\nstore → y → 99\nreturn → y\nend";
        let issues = lint(src);
        assert!(!has_warning(&issues, "shadows a parameter"));
    }

    #[test]
    fn test_mutable_global() {
        // Function modifies a global variable
        let src = "store → counter → 0\ndefine → inc → ()\nstore → counter → counter + 1\nend";
        let issues = lint(src);
        assert!(
            has_warning(&issues, "modifies global variable"),
            "got: {:?}",
            issues
        );
    }

    #[test]
    fn test_no_mutable_global_local_shadow() {
        // Function defines its own local with the same name — not a mutation
        let src = "store → x → 0\ndefine → f → (x)\nreturn → x + 1\nend";
        let issues = lint(src);
        assert!(!has_warning(&issues, "modifies global variable"));
    }

    // ── L010: Division by zero literal ──────────────────────────────────────

    #[test]
    fn test_l010_division_by_zero_literal() {
        let src = "store → x → 10 / 0";
        let issues = lint(src);
        assert!(
            issues.iter().any(|i| i.rule_id.as_deref() == Some("L010")),
            "expected L010, got: {:?}", issues
        );
    }

    #[test]
    fn test_l010_no_false_positive_nonzero() {
        let src = "store → x → 10 / 2";
        let issues = lint(src);
        assert!(!issues.iter().any(|i| i.rule_id.as_deref() == Some("L010")));
    }

    // ── L011: Unreachable match arm ──────────────────────────────────────────

    #[test]
    fn test_l011_unreachable_arm_after_wildcard() {
        // match with _ followed by another case → L011
        let src = "store → x → 1\nmatch x\n  case _\n    print → 0\n  case 1\n    print → 1\nend";
        let issues = lint(src);
        assert!(
            issues.iter().any(|i| i.rule_id.as_deref() == Some("L011")),
            "expected L011, got: {:?}", issues
        );
    }

    #[test]
    fn test_l011_no_false_positive_wildcard_last() {
        let src = "store → x → 1\nmatch x\n  case 1\n    print → 1\n  case _\n    print → 0\nend";
        let issues = lint(src);
        assert!(!issues.iter().any(|i| i.rule_id.as_deref() == Some("L011")));
    }

    // ── L012: Empty catch block ──────────────────────────────────────────────

    #[test]
    fn test_l012_empty_catch() {
        let src = "try\n  print → 1\ncatch e\nend";
        let issues = lint(src);
        assert!(
            issues.iter().any(|i| i.rule_id.as_deref() == Some("L012")),
            "expected L012, got: {:?}", issues
        );
    }

    #[test]
    fn test_l012_nonempty_catch_no_warn() {
        let src = "try\n  print → 1\ncatch e\n  print → e\nend";
        let issues = lint(src);
        assert!(!issues.iter().any(|i| i.rule_id.as_deref() == Some("L012")));
    }

    // ── L013: Unused function ────────────────────────────────────────────────

    #[test]
    fn test_l013_unused_function() {
        let src = "define → helper → ()\n  return → 1\nend\nprint → 0";
        let issues = lint(src);
        assert!(
            issues.iter().any(|i| i.rule_id.as_deref() == Some("L013") && i.message.contains("helper")),
            "expected L013 for 'helper', got: {:?}", issues
        );
    }

    #[test]
    fn test_l013_used_function_no_warn() {
        let src = "define → helper → ()\n  return → 1\nend\nstore → x → helper()";
        let issues = lint(src);
        assert!(!issues.iter().any(|i| i.rule_id.as_deref() == Some("L013") && i.message.contains("helper")));
    }

    // ── L017: Null comparison ────────────────────────────────────────────────

    #[test]
    fn test_l017_null_comparison() {
        let src = "store → x → null\nif → x == null\nprint → 1\nend";
        let issues = lint(src);
        assert!(
            issues.iter().any(|i| i.rule_id.as_deref() == Some("L017")),
            "expected L017, got: {:?}", issues
        );
    }

    #[test]
    fn test_l017_no_false_positive_nonnull() {
        let src = "store → x → 1\nif → x == 1\nprint → x\nend";
        let issues = lint(src);
        assert!(!issues.iter().any(|i| i.rule_id.as_deref() == Some("L017")));
    }

    // ── L019: Match with no wildcard arm ─────────────────────────────────────

    #[test]
    fn test_l019_match_no_wildcard() {
        let src = "store → x → 1\nmatch x\n  case 1\n    print → 1\n  case 2\n    print → 2\nend";
        let issues = lint(src);
        assert!(
            issues.iter().any(|i| i.rule_id.as_deref() == Some("L019")),
            "expected L019, got: {:?}", issues
        );
    }

    #[test]
    fn test_l019_match_with_wildcard_no_warn() {
        let src = "store → x → 1\nmatch x\n  case 1\n    print → 1\n  case _\n    print → 0\nend";
        let issues = lint(src);
        assert!(!issues.iter().any(|i| i.rule_id.as_deref() == Some("L019")));
    }

    // ── rule_id field is included in issues ──────────────────────────────────

    #[test]
    fn test_rule_id_field_exists_on_new_rules() {
        let src = "store → x → 10 / 0";
        let issues = lint(src);
        let l010 = issues.iter().find(|i| i.rule_id.as_deref() == Some("L010"));
        assert!(l010.is_some(), "L010 issue should have rule_id set");
        assert_eq!(l010.unwrap().rule_id.as_deref(), Some("L010"));
    }
}
