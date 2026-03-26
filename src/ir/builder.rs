//! `IrBuilder` — lowers a `Program` AST into a backend-agnostic `ProgramIr`.
//!
//! All constant folding and dead-branch elimination happens here, before any
//! backend sees the IR.  The bytecode optimizer (`compiler/optimizer.rs`)
//! retains only bytecode-specific peephole passes (Nop removal, stack-level
//! constant folding).

use crate::parser::ast::common::{BinaryOperator, Literal, Pattern, Span, UnaryOperator};
use crate::parser::ast::expressions::Expression;
use crate::parser::ast::statements::{Program, Statement};

use super::instruction::{CapabilityCall, IrNode, IrParam};
use super::program::ProgramIr;

/// Known stdlib function names that touch guarded resources.
/// Exact name OR a prefix followed by `_` must match the call site.
/// Format: (fn_name_or_prefix, resource, action)
const CAPABILITY_FNS: &[(&str, &str, &str)] = &[
    ("read_file",     "fs",      "read"),
    ("write_file",    "fs",      "write"),
    ("append_file",   "fs",      "write"),
    ("delete_file",   "fs",      "write"),
    ("list_dir",      "fs",      "read"),
    ("http_get",      "net",     "connect"),
    ("http_post",     "net",     "connect"),
    ("http_put",      "net",     "connect"),
    ("http_delete",   "net",     "connect"),
    ("http_patch",    "net",     "connect"),
    ("http_serve",    "net",     "listen"),
    ("ws_connect",    "net",     "connect"),
    ("ws_serve",      "net",     "listen"),
    ("db_connect",    "db",      "connect"),
    ("db_query",      "db",      "read"),
    ("db_execute",    "db",      "write"),
    ("db_transaction","db",      "write"),
    ("exec",          "process", "exec"),
    ("spawn",         "process", "exec"),
    ("ffi_load",      "sys",     "ffi"),
    ("ffi_call",      "sys",     "ffi"),
    ("plugin_load",   "sys",     "plugin"),
    ("plugin_call",   "sys",     "plugin"),
];

/// Lowers a `Program` AST into a `ProgramIr`.
///
/// Applies constant folding and dead-branch elimination in a single pass;
/// wraps guarded stdlib calls in `IrNode::CapabilityCall`.
pub struct IrBuilder {
    fold_count: usize,
    dead_branch_count: usize,
}

impl IrBuilder {
    pub fn new() -> Self {
        Self { fold_count: 0, dead_branch_count: 0 }
    }

    pub fn fold_count(&self) -> usize { self.fold_count }
    pub fn dead_branch_count(&self) -> usize { self.dead_branch_count }

    /// Apply constant folding and dead-branch elimination directly to the AST.
    ///
    /// Unlike `lower()`, this mutates the `Program` in place so the AST VM
    /// immediately benefits from the optimizations without any IR middleman.
    pub fn apply_to_ast(&mut self, program: &mut Program) {
        let stmts = std::mem::take(&mut program.statements);
        let mut out = Vec::with_capacity(stmts.len());
        for stmt in stmts {
            self.fold_stmt_into(stmt, &mut out);
        }
        program.statements = out;
    }

    /// Fold a single statement and push the result(s) into `out`.
    /// An `if → true` expands to its then-branch (multiple statements).
    fn fold_stmt_into(&mut self, stmt: Statement, out: &mut Vec<Statement>) {
        match stmt {
            Statement::Assignment { pattern, type_annotation, value, span } => {
                out.push(Statement::Assignment {
                    pattern,
                    type_annotation,
                    value: self.fold_expr(value),
                    span,
                });
            }
            Statement::CompoundAssignment { name, op, value, span } => {
                out.push(Statement::CompoundAssignment { name, op, value: self.fold_expr(value), span });
            }
            Statement::IndexAssignment { target, index, value, span } => {
                out.push(Statement::IndexAssignment {
                    target: self.fold_expr(target),
                    index: self.fold_expr(index),
                    value: self.fold_expr(value),
                    span,
                });
            }
            Statement::If { condition, then_branch, else_if_branches, else_branch, span } => {
                let cond = self.fold_expr(condition);
                if let Expression::Literal(Literal::Boolean(b)) = cond {
                    // Dead-branch elimination.
                    self.dead_branch_count += 1;
                    let body = if b { then_branch } else { else_branch.unwrap_or_default() };
                    for s in body {
                        self.fold_stmt_into(s, out);
                    }
                    return;
                }
                let then_branch = self.fold_stmts(then_branch);
                let else_if_branches = else_if_branches
                    .into_iter()
                    .map(|(c, body)| (self.fold_expr(c), self.fold_stmts(body)))
                    .collect();
                let else_branch = else_branch.map(|body| self.fold_stmts(body));
                out.push(Statement::If { condition: cond, then_branch, else_if_branches, else_branch, span });
            }
            Statement::While { condition, body, span } => {
                out.push(Statement::While {
                    condition: self.fold_expr(condition),
                    body: self.fold_stmts(body),
                    span,
                });
            }
            Statement::DoWhile { body, condition, span } => {
                out.push(Statement::DoWhile {
                    body: self.fold_stmts(body),
                    condition: self.fold_expr(condition),
                    span,
                });
            }
            Statement::For { variable, iterable, body, span } => {
                out.push(Statement::For {
                    variable,
                    iterable: self.fold_expr(iterable),
                    body: self.fold_stmts(body),
                    span,
                });
            }
            Statement::Repeat { count, body, span } => {
                out.push(Statement::Repeat {
                    count: self.fold_expr(count),
                    body: self.fold_stmts(body),
                    span,
                });
            }
            Statement::Return { value, span } => {
                out.push(Statement::Return { value: value.map(|e| self.fold_expr(e)), span });
            }
            Statement::Yield { value, span } => {
                out.push(Statement::Yield { value: self.fold_expr(value), span });
            }
            Statement::Expression(expr) => {
                out.push(Statement::Expression(self.fold_expr(expr)));
            }
            Statement::Assert { condition, message, span } => {
                out.push(Statement::Assert {
                    condition: self.fold_expr(condition),
                    message: message.map(|e| self.fold_expr(e)),
                    span,
                });
            }
            Statement::FunctionDef { name, type_params, params, return_type, body, is_async, intent, ai_hint, allowed_actions, forbidden_actions, span } => {
                out.push(Statement::FunctionDef {
                    name,
                    type_params,
                    params,
                    return_type,
                    body: self.fold_stmts(body),
                    is_async,
                    intent,
                    ai_hint,
                    allowed_actions,
                    forbidden_actions,
                    span,
                });
            }
            // All other statements pass through unchanged.
            other => out.push(other),
        }
    }

    fn fold_stmts(&mut self, stmts: Vec<Statement>) -> Vec<Statement> {
        let mut out = Vec::with_capacity(stmts.len());
        for s in stmts {
            self.fold_stmt_into(s, &mut out);
        }
        out
    }

    fn fold_expr(&mut self, expr: Expression) -> Expression {
        match expr {
            Expression::BinaryOp { left, op, right, span } => {
                let left = self.fold_expr(*left);
                let right = self.fold_expr(*right);
                if let (Expression::Literal(l), Expression::Literal(r)) = (&left, &right) {
                    if let Some(result) = eval_binop(l, op, r) {
                        self.fold_count += 1;
                        return Expression::Literal(result);
                    }
                }
                Expression::BinaryOp { left: Box::new(left), op, right: Box::new(right), span }
            }
            Expression::UnaryOp { op, operand, span } => {
                let operand = self.fold_expr(*operand);
                if let Expression::Literal(lit) = &operand {
                    if let Some(result) = eval_unary(op, lit) {
                        self.fold_count += 1;
                        return Expression::Literal(result);
                    }
                }
                Expression::UnaryOp { op, operand: Box::new(operand), span }
            }
            Expression::FunctionCall { name, type_arguments, arguments, span } => {
                let arguments = arguments.into_iter().map(|a| self.fold_expr(a)).collect();
                Expression::FunctionCall { name, type_arguments, arguments, span }
            }
            Expression::Array { elements, span } => {
                Expression::Array { elements: elements.into_iter().map(|e| self.fold_expr(e)).collect(), span }
            }
            Expression::Map { entries, span } => {
                Expression::Map {
                    entries: entries.into_iter().map(|(k, v)| (self.fold_expr(k), self.fold_expr(v))).collect(),
                    span,
                }
            }
            Expression::Ternary { condition, true_expr, false_expr, span } => {
                let condition = self.fold_expr(*condition);
                if let Expression::Literal(Literal::Boolean(b)) = condition {
                    self.dead_branch_count += 1;
                    return if b { self.fold_expr(*true_expr) } else { self.fold_expr(*false_expr) };
                }
                Expression::Ternary {
                    condition: Box::new(condition),
                    true_expr: Box::new(self.fold_expr(*true_expr)),
                    false_expr: Box::new(self.fold_expr(*false_expr)),
                    span,
                }
            }
            Expression::Index { target, index, span } => {
                Expression::Index { target: Box::new(self.fold_expr(*target)), index: Box::new(self.fold_expr(*index)), span }
            }
            Expression::Member { target, name, span } => {
                Expression::Member { target: Box::new(self.fold_expr(*target)), name, span }
            }
            Expression::Await { expression, span } => {
                Expression::Await { expression: Box::new(self.fold_expr(*expression)), span }
            }
            // Pass through everything else unchanged.
            other => other,
        }
    }

    /// Lower an entire program.
    pub fn lower(&mut self, program: &Program) -> ProgramIr {
        let nodes = program.statements.iter()
            .map(|s| self.lower_stmt(s))
            .collect();
        ProgramIr {
            nodes,
            fold_count: self.fold_count,
            dead_branch_count: self.dead_branch_count,
        }
    }

    // ── Statements ─────────────────────────────────────────────────────────────

    fn lower_stmt(&mut self, stmt: &Statement) -> IrNode {
        match stmt {
            Statement::Assignment { pattern, value, span, .. } => {
                IrNode::Assign {
                    name: pattern_name(pattern),
                    value: Box::new(self.lower_expr(value)),
                    span: span.clone(),
                }
            }
            Statement::CompoundAssignment { name, op, value, span } => {
                // Desugar: `x += v` → `x = x + v` then attempt constant fold.
                let lhs = IrNode::Var(name.clone());
                let rhs = self.lower_expr(value);
                let folded = self.fold_binop(lhs, *op, rhs, span.clone());
                IrNode::Assign {
                    name: name.clone(),
                    value: Box::new(folded),
                    span: span.clone(),
                }
            }
            Statement::IndexAssignment { target, index, value, span } => {
                IrNode::IndexAssign {
                    target: Box::new(self.lower_expr(target)),
                    index: Box::new(self.lower_expr(index)),
                    value: Box::new(self.lower_expr(value)),
                    span: span.clone(),
                }
            }
            Statement::FunctionDef { name, params, body, is_async, span, .. } => {
                let ir_params = params.iter().map(|p| IrParam {
                    name: p.name.clone(),
                    type_hint: p.type_annotation.as_ref().map(|t| format!("{:?}", t)),
                }).collect();
                IrNode::FunctionDef {
                    name: name.clone(),
                    params: ir_params,
                    body: Box::new(IrNode::Block(
                        body.iter().map(|s| self.lower_stmt(s)).collect()
                    )),
                    is_async: *is_async,
                    span: span.clone(),
                }
            }
            Statement::Return { value, span: _ } => {
                IrNode::Return(value.as_ref().map(|e| Box::new(self.lower_expr(e))))
            }
            Statement::Break { .. } => IrNode::Break,
            Statement::Continue { .. } => IrNode::Continue,
            Statement::Yield { value, .. } => {
                // Yield is treated as an expression statement at IR level.
                self.lower_expr(value)
            }
            Statement::If {
                condition, then_branch, else_if_branches, else_branch, span,
            } => {
                let cond = self.lower_expr(condition);
                // Dead-branch elimination: constant boolean condition.
                if let IrNode::Const(Literal::Boolean(b)) = &cond {
                    self.dead_branch_count += 1;
                    return if *b {
                        IrNode::Block(then_branch.iter().map(|s| self.lower_stmt(s)).collect())
                    } else if let Some(else_body) = else_branch {
                        IrNode::Block(else_body.iter().map(|s| self.lower_stmt(s)).collect())
                    } else {
                        IrNode::Nop
                    };
                }
                let else_ifs = else_if_branches.iter().map(|(c, body)| {
                    let ic = self.lower_expr(c);
                    let ib = IrNode::Block(body.iter().map(|s| self.lower_stmt(s)).collect());
                    (ic, ib)
                }).collect();
                let else_block = else_branch.as_ref().map(|body| {
                    Box::new(IrNode::Block(body.iter().map(|s| self.lower_stmt(s)).collect()))
                });
                IrNode::If {
                    condition: Box::new(cond),
                    then_block: Box::new(IrNode::Block(
                        then_branch.iter().map(|s| self.lower_stmt(s)).collect()
                    )),
                    else_ifs,
                    else_block,
                    span: span.clone(),
                }
            }
            Statement::While { condition, body, span } => {
                IrNode::Loop {
                    condition: Some(Box::new(self.lower_expr(condition))),
                    body: Box::new(IrNode::Block(
                        body.iter().map(|s| self.lower_stmt(s)).collect()
                    )),
                    span: span.clone(),
                }
            }
            Statement::DoWhile { body, condition, span } => {
                // do-while: emit loop-forever with condition check at end.
                // Represented as Loop { condition: None } with the check inside.
                IrNode::Loop {
                    condition: Some(Box::new(self.lower_expr(condition))),
                    body: Box::new(IrNode::Block(
                        body.iter().map(|s| self.lower_stmt(s)).collect()
                    )),
                    span: span.clone(),
                }
            }
            Statement::For { variable, iterable, body, span } => {
                IrNode::ForEach {
                    variable: variable.clone(),
                    iterable: Box::new(self.lower_expr(iterable)),
                    body: Box::new(IrNode::Block(
                        body.iter().map(|s| self.lower_stmt(s)).collect()
                    )),
                    span: span.clone(),
                }
            }
            Statement::Repeat { count, body, span } => {
                // `repeat n { body }` → ForEach over range(0, n).
                IrNode::Loop {
                    condition: Some(Box::new(self.lower_expr(count))),
                    body: Box::new(IrNode::Block(
                        body.iter().map(|s| self.lower_stmt(s)).collect()
                    )),
                    span: span.clone(),
                }
            }
            Statement::Expression(expr) => self.lower_expr(expr),
            Statement::Assert { condition, message, span } => {
                // Emit as a Call node so backends can decide how to implement it.
                let cond = self.lower_expr(condition);
                let mut args = vec![cond];
                if let Some(msg) = message {
                    args.push(self.lower_expr(msg));
                }
                IrNode::Call { name: "__assert__".to_string(), args, span: span.clone() }
            }
            Statement::Try { body, catch, .. } => {
                // Emit the try-body as a Block; catch body follows.
                let mut nodes: Vec<IrNode> = body.iter().map(|s| self.lower_stmt(s)).collect();
                if let Some((_var, catch_body)) = catch {
                    nodes.extend(catch_body.iter().map(|s| self.lower_stmt(s)));
                }
                IrNode::Block(nodes)
            }
            // Type declarations, imports, struct/enum/protocol defs → Nop.
            // They are metadata consumed by the type checker, not execution steps.
            _ => IrNode::Nop,
        }
    }

    // ── Expressions ────────────────────────────────────────────────────────────

    fn lower_expr(&mut self, expr: &Expression) -> IrNode {
        match expr {
            Expression::Literal(lit) => IrNode::Const(lit.clone()),
            Expression::Identifier(name) => IrNode::Var(name.clone()),
            Expression::BinaryOp { left, op, right, span } => {
                let l = self.lower_expr(left);
                let r = self.lower_expr(right);
                self.fold_binop(l, *op, r, span.clone())
            }
            Expression::UnaryOp { op, operand, span } => {
                let inner = self.lower_expr(operand);
                self.fold_unary(*op, inner, span.clone())
            }
            Expression::FunctionCall { name, arguments, span, .. } => {
                let args: Vec<IrNode> = arguments.iter().map(|a| self.lower_expr(a)).collect();
                let call = IrNode::Call { name: name.clone(), args, span: span.clone() };
                self.maybe_wrap_capability(call, name, span.clone())
            }
            Expression::Array { elements, .. } => {
                IrNode::Array(elements.iter().map(|e| self.lower_expr(e)).collect())
            }
            Expression::Map { entries, .. } => {
                IrNode::Map(entries.iter().map(|(k, v)| {
                    (self.lower_expr(k), self.lower_expr(v))
                }).collect())
            }
            Expression::Ternary { condition, true_expr, false_expr, span } => {
                let cond = self.lower_expr(condition);
                // Dead-branch elimination for constant ternary.
                if let IrNode::Const(Literal::Boolean(b)) = &cond {
                    self.dead_branch_count += 1;
                    return if *b {
                        self.lower_expr(true_expr)
                    } else {
                        self.lower_expr(false_expr)
                    };
                }
                IrNode::If {
                    condition: Box::new(cond),
                    then_block: Box::new(self.lower_expr(true_expr)),
                    else_ifs: vec![],
                    else_block: Some(Box::new(self.lower_expr(false_expr))),
                    span: span.clone(),
                }
            }
            Expression::Index { target, index, span } => {
                IrNode::Call {
                    name: "__index__".to_string(),
                    args: vec![self.lower_expr(target), self.lower_expr(index)],
                    span: span.clone(),
                }
            }
            Expression::Member { target, name, span } => {
                IrNode::Call {
                    name: format!("__member__{}", name),
                    args: vec![self.lower_expr(target)],
                    span: span.clone(),
                }
            }
            Expression::Await { expression, .. } => self.lower_expr(expression),
            // Interpolated strings, slices, optional chaining, etc. → Nop.
            // These are handled by the AST VM and do not need IR representation yet.
            _ => IrNode::Nop,
        }
    }

    // ── Constant folding ───────────────────────────────────────────────────────

    fn fold_binop(&mut self, left: IrNode, op: BinaryOperator, right: IrNode, span: Span) -> IrNode {
        if let (IrNode::Const(l), IrNode::Const(r)) = (&left, &right) {
            if let Some(result) = eval_binop(l, op, r) {
                self.fold_count += 1;
                return IrNode::Const(result);
            }
        }
        IrNode::BinOp { left: Box::new(left), op, right: Box::new(right), span }
    }

    fn fold_unary(&mut self, op: UnaryOperator, operand: IrNode, span: Span) -> IrNode {
        if let IrNode::Const(lit) = &operand {
            if let Some(result) = eval_unary(op, lit) {
                self.fold_count += 1;
                return IrNode::Const(result);
            }
        }
        IrNode::UnaryOp { op, operand: Box::new(operand), span }
    }

    // ── Capability wrapping ────────────────────────────────────────────────────

    fn maybe_wrap_capability(&self, call: IrNode, fn_name: &str, span: Span) -> IrNode {
        for (prefix, resource, action) in CAPABILITY_FNS {
            if fn_name == *prefix {
                return IrNode::CapabilityCall {
                    call: Box::new(call),
                    capability: CapabilityCall {
                        resource: resource.to_string(),
                        action: action.to_string(),
                        span,
                    },
                };
            }
        }
        call
    }
}

impl Default for IrBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ── Pure constant-eval helpers ────────────────────────────────────────────────

fn eval_binop(left: &Literal, op: BinaryOperator, right: &Literal) -> Option<Literal> {
    match (left, op, right) {
        (Literal::Integer(a), BinaryOperator::Add,          Literal::Integer(b)) => Some(Literal::Integer(a.wrapping_add(*b))),
        (Literal::Integer(a), BinaryOperator::Subtract,     Literal::Integer(b)) => Some(Literal::Integer(a.wrapping_sub(*b))),
        (Literal::Integer(a), BinaryOperator::Multiply,     Literal::Integer(b)) => Some(Literal::Integer(a.wrapping_mul(*b))),
        (Literal::Integer(a), BinaryOperator::Divide,       Literal::Integer(b)) if *b != 0 => Some(Literal::Integer(a / b)),
        (Literal::Integer(a), BinaryOperator::Modulo,       Literal::Integer(b)) if *b != 0 => Some(Literal::Integer(a % b)),
        (Literal::Integer(a), BinaryOperator::Power,        Literal::Integer(b)) => Some(Literal::Integer((*a as f64).powi(*b as i32) as i64)),
        (Literal::Integer(a), BinaryOperator::Equal,        Literal::Integer(b)) => Some(Literal::Boolean(a == b)),
        (Literal::Integer(a), BinaryOperator::NotEqual,     Literal::Integer(b)) => Some(Literal::Boolean(a != b)),
        (Literal::Integer(a), BinaryOperator::Less,         Literal::Integer(b)) => Some(Literal::Boolean(a < b)),
        (Literal::Integer(a), BinaryOperator::Greater,      Literal::Integer(b)) => Some(Literal::Boolean(a > b)),
        (Literal::Integer(a), BinaryOperator::LessEqual,    Literal::Integer(b)) => Some(Literal::Boolean(a <= b)),
        (Literal::Integer(a), BinaryOperator::GreaterEqual, Literal::Integer(b)) => Some(Literal::Boolean(a >= b)),
        (Literal::Float(a),   BinaryOperator::Add,          Literal::Float(b))   => Some(Literal::Float(a + b)),
        (Literal::Float(a),   BinaryOperator::Subtract,     Literal::Float(b))   => Some(Literal::Float(a - b)),
        (Literal::Float(a),   BinaryOperator::Multiply,     Literal::Float(b))   => Some(Literal::Float(a * b)),
        (Literal::Float(a),   BinaryOperator::Divide,       Literal::Float(b)) if *b != 0.0 => Some(Literal::Float(a / b)),
        (Literal::String(a),  BinaryOperator::Add,          Literal::String(b))  => Some(Literal::String(format!("{}{}", a, b))),
        (Literal::Boolean(a), BinaryOperator::And,          Literal::Boolean(b)) => Some(Literal::Boolean(*a && *b)),
        (Literal::Boolean(a), BinaryOperator::Or,           Literal::Boolean(b)) => Some(Literal::Boolean(*a || *b)),
        _ => None,
    }
}

fn eval_unary(op: UnaryOperator, operand: &Literal) -> Option<Literal> {
    match (op, operand) {
        (UnaryOperator::Not,   Literal::Boolean(b)) => Some(Literal::Boolean(!b)),
        (UnaryOperator::Minus, Literal::Integer(n)) => Some(Literal::Integer(n.wrapping_neg())),
        (UnaryOperator::Minus, Literal::Float(n))   => Some(Literal::Float(-n)),
        _ => None,
    }
}

fn pattern_name(pattern: &Pattern) -> String {
    match pattern {
        Pattern::Identifier(name) => name.clone(),
        Pattern::Ignore => "_".to_string(),
        _ => "__pattern__".to_string(),
    }
}
