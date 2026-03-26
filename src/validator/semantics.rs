// Semantic validation - checks for semantic errors before execution

use super::ValidationError;
use crate::parser::ast::expressions::Expression;
use crate::parser::ast::{Program, Statement};
use std::collections::{HashMap, HashSet};

pub struct SemanticsValidator;

impl SemanticsValidator {
    /// Check program for semantic errors.
    pub fn check_program(program: &Program) -> Result<(), ValidationError> {
        Self::check_duplicate_functions(&program.statements)?;
        Self::check_return_outside_function(&program.statements, false)?;
        Self::check_break_continue_outside_loop(&program.statements, false)?;
        // Q.1 + Q.2: advisory warnings — printed but do not halt by default.
        let warnings = Self::collect_advisory_warnings(&program.statements);
        for w in &warnings {
            eprintln!("[WARNING] validator: {}", w);
        }
        Ok(())
    }

    // ── Q.1 + Q.2: Advisory checks ────────────────────────────────────────────

    /// Collect advisory warnings: undefined variables and arity mismatches.
    ///
    /// Returns a list of human-readable warning strings.  Does not halt
    /// execution — callers decide whether to treat these as hard errors.
    pub fn collect_advisory_warnings(stmts: &[Statement]) -> Vec<String> {
        let mut warnings = Vec::new();
        // First pass: collect user-defined function names and their arity.
        let mut fn_arity: HashMap<String, usize> = HashMap::new();
        Self::collect_fn_defs(stmts, &mut fn_arity);

        // Second pass: scope-aware undefined-variable + arity scan.
        let mut scope_stack: Vec<HashSet<String>> = vec![HashSet::new()];
        // Pre-populate top scope with all defined function names.
        for name in fn_arity.keys() {
            if let Some(top) = scope_stack.first_mut() {
                top.insert(name.clone());
            }
        }
        Self::scan_stmts(stmts, &mut scope_stack, &fn_arity, &mut warnings);
        warnings
    }

    /// Recursively collect `FunctionDef` names → param count.
    fn collect_fn_defs(stmts: &[Statement], out: &mut HashMap<String, usize>) {
        for stmt in stmts {
            if let Statement::FunctionDef { name, params, body, .. } = stmt {
                out.insert(name.clone(), params.len());
                Self::collect_fn_defs(body, out);
            }
        }
    }

    /// Known stdlib / built-in names that are always in scope.
    ///
    /// Authoritative source: `crate::stdlib::stdlib_function_names()`, which is
    /// built from `STDLIB_DISPATCH` + a supplemental list of executor-dependent
    /// and prefix-routed functions.  This replaces the old hardcoded array.
    fn stdlib_names() -> &'static HashSet<&'static str> {
        crate::stdlib::stdlib_function_names()
    }

    /// Return true if `name` matches a known stdlib prefix pattern (str_*, http_*, etc.).
    fn is_stdlib_prefix(name: &str) -> bool {
        crate::stdlib::is_stdlib_prefix(name)
    }

    fn in_scope(name: &str, scope_stack: &[HashSet<String>]) -> bool {
        scope_stack.iter().any(|s| s.contains(name))
    }

    fn scan_stmts(
        stmts: &[Statement],
        scope_stack: &mut Vec<HashSet<String>>,
        fn_arity: &HashMap<String, usize>,
        warnings: &mut Vec<String>,
    ) {
        for stmt in stmts {
            Self::scan_stmt(stmt, scope_stack, fn_arity, warnings);
        }
    }

    fn scan_stmt(
        stmt: &Statement,
        scope_stack: &mut Vec<HashSet<String>>,
        fn_arity: &HashMap<String, usize>,
        warnings: &mut Vec<String>,
    ) {
        let stdlib = Self::stdlib_names();
        match stmt {
            Statement::Assignment { pattern, value, .. } => {
                Self::scan_expr(value, scope_stack, fn_arity, &stdlib, warnings);
                // Bring assigned name(s) into scope.
                if let crate::parser::ast::common::Pattern::Identifier(name) = pattern {
                    if let Some(top) = scope_stack.last_mut() {
                        top.insert(name.clone());
                    }
                }
            }
            Statement::IndexAssignment { target, index, value, .. } => {
                Self::scan_expr(target, scope_stack, fn_arity, &stdlib, warnings);
                Self::scan_expr(index, scope_stack, fn_arity, &stdlib, warnings);
                Self::scan_expr(value, scope_stack, fn_arity, &stdlib, warnings);
            }
            Statement::CompoundAssignment { name, value, .. } => {
                if !Self::in_scope(name, scope_stack)
                    && !stdlib.contains(name.as_str())
                    && !Self::is_stdlib_prefix(name)
                {
                    warnings.push(format!("Possible undefined variable '{}'.", name));
                }
                Self::scan_expr(value, scope_stack, fn_arity, &stdlib, warnings);
            }
            Statement::FunctionDef { name, params, body, .. } => {
                // Add function name to current scope before scanning body so
                // recursive calls don't produce a false positive.
                if let Some(top) = scope_stack.last_mut() {
                    top.insert(name.clone());
                }
                scope_stack.push(HashSet::new());
                for p in params {
                    if let Some(top) = scope_stack.last_mut() {
                        top.insert(p.name.clone());
                    }
                }
                Self::scan_stmts(body, scope_stack, fn_arity, warnings);
                scope_stack.pop();
            }
            Statement::Return { value, .. } => {
                if let Some(e) = value {
                    Self::scan_expr(e, scope_stack, fn_arity, &stdlib, warnings);
                }
            }
            Statement::Yield { value, .. } => {
                Self::scan_expr(value, scope_stack, fn_arity, &stdlib, warnings);
            }
            Statement::Expression(e) => {
                Self::scan_expr(e, scope_stack, fn_arity, &stdlib, warnings);
            }
            Statement::If { condition, then_branch, else_if_branches, else_branch, .. } => {
                Self::scan_expr(condition, scope_stack, fn_arity, &stdlib, warnings);
                scope_stack.push(HashSet::new());
                Self::scan_stmts(then_branch, scope_stack, fn_arity, warnings);
                scope_stack.pop();
                for (cond, branch) in else_if_branches {
                    Self::scan_expr(cond, scope_stack, fn_arity, &stdlib, warnings);
                    scope_stack.push(HashSet::new());
                    Self::scan_stmts(branch, scope_stack, fn_arity, warnings);
                    scope_stack.pop();
                }
                if let Some(branch) = else_branch {
                    scope_stack.push(HashSet::new());
                    Self::scan_stmts(branch, scope_stack, fn_arity, warnings);
                    scope_stack.pop();
                }
            }
            Statement::While { condition, body, .. }
            | Statement::DoWhile { condition, body, .. } => {
                Self::scan_expr(condition, scope_stack, fn_arity, &stdlib, warnings);
                scope_stack.push(HashSet::new());
                Self::scan_stmts(body, scope_stack, fn_arity, warnings);
                scope_stack.pop();
            }
            Statement::For { variable, iterable, body, .. } => {
                Self::scan_expr(iterable, scope_stack, fn_arity, &stdlib, warnings);
                scope_stack.push(HashSet::new());
                if let Some(top) = scope_stack.last_mut() {
                    top.insert(variable.clone());
                }
                Self::scan_stmts(body, scope_stack, fn_arity, warnings);
                scope_stack.pop();
            }
            Statement::Repeat { count, body, .. } => {
                Self::scan_expr(count, scope_stack, fn_arity, &stdlib, warnings);
                scope_stack.push(HashSet::new());
                Self::scan_stmts(body, scope_stack, fn_arity, warnings);
                scope_stack.pop();
            }
            Statement::Try { body, catch, finally, .. } => {
                scope_stack.push(HashSet::new());
                Self::scan_stmts(body, scope_stack, fn_arity, warnings);
                scope_stack.pop();
                if let Some((var, branch)) = catch {
                    scope_stack.push(HashSet::new());
                    if let Some(top) = scope_stack.last_mut() {
                        top.insert(var.clone());
                    }
                    Self::scan_stmts(branch, scope_stack, fn_arity, warnings);
                    scope_stack.pop();
                }
                if let Some(branch) = finally {
                    scope_stack.push(HashSet::new());
                    Self::scan_stmts(branch, scope_stack, fn_arity, warnings);
                    scope_stack.pop();
                }
            }
            Statement::Match { value, cases, default, .. } => {
                Self::scan_expr(value, scope_stack, fn_arity, &stdlib, warnings);
                for (_, _, body) in cases {
                    scope_stack.push(HashSet::new());
                    Self::scan_stmts(body, scope_stack, fn_arity, warnings);
                    scope_stack.pop();
                }
                if let Some(body) = default {
                    scope_stack.push(HashSet::new());
                    Self::scan_stmts(body, scope_stack, fn_arity, warnings);
                    scope_stack.pop();
                }
            }
            _ => {} // Break, Continue, Pass, Nursery, etc. — no sub-expressions to scan
        }
    }

    fn scan_expr(
        expr: &Expression,
        scope_stack: &mut Vec<HashSet<String>>,
        fn_arity: &HashMap<String, usize>,
        stdlib: &HashSet<&'static str>,
        warnings: &mut Vec<String>,
    ) {
        match expr {
            Expression::Identifier(name) => {
                if !Self::in_scope(name, scope_stack)
                    && !stdlib.contains(name.as_str())
                    && !Self::is_stdlib_prefix(name)
                {
                    // Skip names that look like struct constructors (PascalCase) or
                    // special runtime names (__xxx__).
                    if !name.starts_with("__")
                        && !name.starts_with('_')
                        && !name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
                    {
                        warnings.push(format!("Possible undefined variable '{}'.", name));
                    }
                }
            }
            Expression::FunctionCall { name, arguments, .. } => {
                // Q.2: arity check for user-defined functions.
                if let Some(&expected) = fn_arity.get(name) {
                    if arguments.len() != expected {
                        warnings.push(format!(
                            "Function '{}' expects {} argument(s) but called with {}.",
                            name, expected, arguments.len()
                        ));
                    }
                }
                for arg in arguments {
                    Self::scan_expr(arg, scope_stack, fn_arity, stdlib, warnings);
                }
            }
            Expression::BinaryOp { left, right, .. } => {
                Self::scan_expr(left, scope_stack, fn_arity, stdlib, warnings);
                Self::scan_expr(right, scope_stack, fn_arity, stdlib, warnings);
            }
            Expression::UnaryOp { operand, .. } => {
                Self::scan_expr(operand, scope_stack, fn_arity, stdlib, warnings);
            }
            Expression::Array { elements, .. } => {
                for e in elements {
                    Self::scan_expr(e, scope_stack, fn_arity, stdlib, warnings);
                }
            }
            Expression::Map { entries, .. } => {
                for (k, v) in entries {
                    Self::scan_expr(k, scope_stack, fn_arity, stdlib, warnings);
                    Self::scan_expr(v, scope_stack, fn_arity, stdlib, warnings);
                }
            }
            Expression::Set { elements, .. } => {
                for e in elements {
                    Self::scan_expr(e, scope_stack, fn_arity, stdlib, warnings);
                }
            }
            Expression::Index { target, index, .. } => {
                Self::scan_expr(target, scope_stack, fn_arity, stdlib, warnings);
                Self::scan_expr(index, scope_stack, fn_arity, stdlib, warnings);
            }
            Expression::Member { target, .. } => {
                Self::scan_expr(target, scope_stack, fn_arity, stdlib, warnings);
            }
            Expression::Lambda { params, body, .. } => {
                scope_stack.push(HashSet::new());
                for p in params {
                    if let Some(top) = scope_stack.last_mut() {
                        top.insert(p.name.clone());
                    }
                }
                Self::scan_expr(body, scope_stack, fn_arity, stdlib, warnings);
                scope_stack.pop();
            }
            Expression::Ternary { condition, true_expr, false_expr, .. } => {
                Self::scan_expr(condition, scope_stack, fn_arity, stdlib, warnings);
                Self::scan_expr(true_expr, scope_stack, fn_arity, stdlib, warnings);
                Self::scan_expr(false_expr, scope_stack, fn_arity, stdlib, warnings);
            }
            Expression::InterpolatedString { segments, .. } => {
                for seg in segments {
                    if let crate::parser::ast::common::InterpolatedSegment::Expression(e) = seg {
                        Self::scan_expr(e, scope_stack, fn_arity, stdlib, warnings);
                    }
                }
            }
            _ => {} // Literal, Slice, Spread, Propagate, MethodCall, Pipe, etc.
        }
    }

    /// Detect functions defined more than once at the same scope level.
    ///
    /// Redefining a function silently overwrites the previous definition at
    /// runtime, making code hard to audit. Catch it early.
    fn check_duplicate_functions(stmts: &[Statement]) -> Result<(), ValidationError> {
        let mut seen: HashMap<&str, usize> = HashMap::new();
        for stmt in stmts {
            if let Statement::FunctionDef { name, span, .. } = stmt {
                if let Some(prev_line) = seen.get(name.as_str()) {
                    return Err(ValidationError::Semantic(format!(
                        "Function '{}' is defined more than once \
                         (first definition at line {}, redefinition at line {}). \
                         Rename one of them.",
                        name, prev_line, span.line
                    )));
                }
                seen.insert(name.as_str(), span.line);
            }
        }
        Ok(())
    }

    /// Detect `break` and `continue` statements outside any loop body.
    ///
    /// `in_loop` tracks whether we are currently inside a loop body.
    /// Function bodies reset the flag because a `break` inside a function
    /// defined inside a loop refers to the enclosing scope at call time,
    /// not the outer loop at definition time — so we conservatively flag it.
    fn check_break_continue_outside_loop(stmts: &[Statement], in_loop: bool) -> Result<(), ValidationError> {
        for stmt in stmts {
            match stmt {
                Statement::Break { span } if !in_loop => {
                    return Err(ValidationError::Semantic(format!(
                        "Line {}: `break` outside a loop body.",
                        span.line
                    )));
                }
                Statement::Continue { span } if !in_loop => {
                    return Err(ValidationError::Semantic(format!(
                        "Line {}: `continue` outside a loop body.",
                        span.line
                    )));
                }
                Statement::FunctionDef { body, .. } => {
                    // Reset: break/continue inside a function cannot target the outer loop.
                    Self::check_break_continue_outside_loop(body, false)?;
                }
                Statement::While { body, .. }
                | Statement::DoWhile { body, .. }
                | Statement::For { body, .. }
                | Statement::Repeat { body, .. } => {
                    Self::check_break_continue_outside_loop(body, true)?;
                }
                Statement::If { then_branch, else_if_branches, else_branch, .. } => {
                    Self::check_break_continue_outside_loop(then_branch, in_loop)?;
                    for (_, b) in else_if_branches {
                        Self::check_break_continue_outside_loop(b, in_loop)?;
                    }
                    if let Some(b) = else_branch {
                        Self::check_break_continue_outside_loop(b, in_loop)?;
                    }
                }
                Statement::Try { body, catch, finally, .. } => {
                    Self::check_break_continue_outside_loop(body, in_loop)?;
                    if let Some((_, b)) = catch {
                        Self::check_break_continue_outside_loop(b, in_loop)?;
                    }
                    if let Some(b) = finally {
                        Self::check_break_continue_outside_loop(b, in_loop)?;
                    }
                }
                Statement::Match { cases, default, .. } => {
                    for (_, _, b) in cases {
                        Self::check_break_continue_outside_loop(b, in_loop)?;
                    }
                    if let Some(b) = default {
                        Self::check_break_continue_outside_loop(b, in_loop)?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Detect `return` statements that appear outside any function body.
    ///
    /// Top-level `return` is a no-op in the AST VM and could mask logic errors.
    /// `in_function` tracks whether we are currently inside a FunctionDef body.
    fn check_return_outside_function(stmts: &[Statement], in_function: bool) -> Result<(), ValidationError> {
        for stmt in stmts {
            match stmt {
                Statement::Return { span, .. } if !in_function => {
                    return Err(ValidationError::Semantic(format!(
                        "Line {}: `return` outside a function body has no effect. \
                         Did you mean to wrap this code in a function?",
                        span.line
                    )));
                }
                Statement::FunctionDef { body, .. } => {
                    // Inside a function — nested returns are valid.
                    Self::check_return_outside_function(body, true)?;
                }
                Statement::If { then_branch, else_if_branches, else_branch, .. } => {
                    Self::check_return_outside_function(then_branch, in_function)?;
                    for (_, branch) in else_if_branches {
                        Self::check_return_outside_function(branch, in_function)?;
                    }
                    if let Some(b) = else_branch {
                        Self::check_return_outside_function(b, in_function)?;
                    }
                }
                Statement::While { body, .. }
                | Statement::DoWhile { body, .. }
                | Statement::For { body, .. }
                | Statement::Repeat { body, .. } => {
                    Self::check_return_outside_function(body, in_function)?;
                }
                Statement::Try { body, catch, finally, .. } => {
                    Self::check_return_outside_function(body, in_function)?;
                    if let Some((_, b)) = catch {
                        Self::check_return_outside_function(b, in_function)?;
                    }
                    if let Some(b) = finally {
                        Self::check_return_outside_function(b, in_function)?;
                    }
                }
                Statement::Match { cases, default, .. } => {
                    for (_, _, body) in cases {
                        Self::check_return_outside_function(body, in_function)?;
                    }
                    if let Some(b) = default {
                        Self::check_return_outside_function(b, in_function)?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}
