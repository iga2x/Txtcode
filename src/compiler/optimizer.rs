use crate::compiler::bytecode::Bytecode;
use crate::parser::ast::*;

/// Code optimizer
///
/// **SIMPLIFIED**: Optimizer focuses on essential optimizations only:
/// - Constant folding (evaluate constant expressions at compile time)
/// - Dead code elimination (remove unreachable code)
///
/// Aggressive optimizations (function inlining, loop optimization, constant propagation)
/// are removed to keep the codebase focused on cyber orchestration use cases.
pub struct Optimizer {
    optimization_level: OptimizationLevel,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OptimizationLevel {
    None,
    Basic, // Constant folding + dead code elimination
           // Aggressive removed - not needed for cyber orchestration use case
}

impl Optimizer {
    pub fn new(level: OptimizationLevel) -> Self {
        Self {
            optimization_level: level,
        }
    }

    /// Optimize AST
    pub fn optimize_ast(&self, program: &mut Program) {
        match self.optimization_level {
            OptimizationLevel::None => {}
            OptimizationLevel::Basic => {
                self.constant_folding(program);
                self.dead_code_elimination(program);
            }
        }
    }

    /// Optimize bytecode
    pub fn optimize_bytecode(&self, bytecode: &Bytecode) -> Result<Bytecode, String> {
        let mut optimized = bytecode.clone();

        match self.optimization_level {
            OptimizationLevel::None => {}
            OptimizationLevel::Basic => {
                self.peephole_optimization(&mut optimized);
            }
        }

        Ok(optimized)
    }

    /// Constant folding - evaluate constant expressions at compile time
    fn constant_folding(&self, program: &mut Program) {
        for statement in &mut program.statements {
            self.fold_constants_statement(statement);
        }
    }

    fn fold_constants_statement(&self, statement: &mut Statement) {
        match statement {
            Statement::Assignment { value, .. } => {
                *value = self.fold_constants_expression(value);
            }
            Statement::CompoundAssignment { value, .. } => {
                *value = self.fold_constants_expression(value);
            }
            Statement::Assert {
                condition, message, ..
            } => {
                *condition = self.fold_constants_expression(condition);
                if let Some(msg) = message {
                    *msg = self.fold_constants_expression(msg);
                }
            }
            Statement::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
                ..
            } => {
                *condition = self.fold_constants_expression(condition);
                for stmt in then_branch {
                    self.fold_constants_statement(stmt);
                }
                for (cond, body) in else_if_branches {
                    *cond = self.fold_constants_expression(cond);
                    for stmt in body {
                        self.fold_constants_statement(stmt);
                    }
                }
                if let Some(else_body) = else_branch {
                    for stmt in else_body {
                        self.fold_constants_statement(stmt);
                    }
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                *condition = self.fold_constants_expression(condition);
                for stmt in body {
                    self.fold_constants_statement(stmt);
                }
            }
            Statement::DoWhile {
                body, condition, ..
            } => {
                for stmt in body {
                    self.fold_constants_statement(stmt);
                }
                *condition = self.fold_constants_expression(condition);
            }
            Statement::For { iterable, body, .. } => {
                *iterable = self.fold_constants_expression(iterable);
                for stmt in body {
                    self.fold_constants_statement(stmt);
                }
            }
            Statement::Repeat { count, body, .. } => {
                *count = self.fold_constants_expression(count);
                for stmt in body {
                    self.fold_constants_statement(stmt);
                }
            }
            Statement::Return {
                value: Some(expr), ..
            } => {
                *expr = self.fold_constants_expression(expr);
            }
            Statement::Expression(expr) => {
                *expr = self.fold_constants_expression(expr);
            }
            _ => {}
        }
    }

    fn fold_constants_expression(&self, expr: &Expression) -> Expression {
        match expr {
            Expression::BinaryOp {
                left,
                op,
                right,
                span,
            } => {
                let left_folded = self.fold_constants_expression(left);
                let right_folded = self.fold_constants_expression(right);

                // Try to evaluate if both are literals
                if let (Expression::Literal(left_lit), Expression::Literal(right_lit)) =
                    (&left_folded, &right_folded)
                {
                    if let Some(result) = self.evaluate_binary_op(left_lit, op, right_lit) {
                        return Expression::Literal(result);
                    }
                }

                Expression::BinaryOp {
                    left: Box::new(left_folded),
                    op: *op,
                    right: Box::new(right_folded),
                    span: span.clone(),
                }
            }
            Expression::UnaryOp { op, operand, span } => {
                let operand_folded = self.fold_constants_expression(operand);

                if let Expression::Literal(lit) = &operand_folded {
                    if let Some(result) = self.evaluate_unary_op(op, lit) {
                        return Expression::Literal(result);
                    }
                }

                Expression::UnaryOp {
                    op: *op,
                    operand: Box::new(operand_folded),
                    span: span.clone(),
                }
            }
            Expression::Array { elements, span } => Expression::Array {
                elements: elements
                    .iter()
                    .map(|e| self.fold_constants_expression(e))
                    .collect(),
                span: span.clone(),
            },
            Expression::Map { entries, span } => Expression::Map {
                entries: entries
                    .iter()
                    .map(|(k, v)| (k.clone(), self.fold_constants_expression(v)))
                    .collect(),
                span: span.clone(),
            },
            _ => expr.clone(),
        }
    }

    fn evaluate_binary_op(
        &self,
        left: &Literal,
        op: &BinaryOperator,
        right: &Literal,
    ) -> Option<Literal> {
        match (left, op, right) {
            (Literal::Integer(a), BinaryOperator::Add, Literal::Integer(b)) => {
                Some(Literal::Integer(a + b))
            }
            (Literal::Integer(a), BinaryOperator::Subtract, Literal::Integer(b)) => {
                Some(Literal::Integer(a - b))
            }
            (Literal::Integer(a), BinaryOperator::Multiply, Literal::Integer(b)) => {
                Some(Literal::Integer(a * b))
            }
            (Literal::Integer(a), BinaryOperator::Divide, Literal::Integer(b)) => {
                if *b != 0 {
                    Some(Literal::Integer(a / b))
                } else {
                    None
                }
            }
            (Literal::Integer(a), BinaryOperator::Modulo, Literal::Integer(b)) => {
                if *b != 0 {
                    Some(Literal::Integer(a % b))
                } else {
                    None
                }
            }
            (Literal::Integer(a), BinaryOperator::Power, Literal::Integer(b)) => {
                Some(Literal::Integer((*a as f64).powi(*b as i32) as i64))
            }
            (Literal::Integer(a), BinaryOperator::Equal, Literal::Integer(b)) => {
                Some(Literal::Boolean(a == b))
            }
            (Literal::Integer(a), BinaryOperator::NotEqual, Literal::Integer(b)) => {
                Some(Literal::Boolean(a != b))
            }
            (Literal::Integer(a), BinaryOperator::Less, Literal::Integer(b)) => {
                Some(Literal::Boolean(a < b))
            }
            (Literal::Integer(a), BinaryOperator::Greater, Literal::Integer(b)) => {
                Some(Literal::Boolean(a > b))
            }
            (Literal::Integer(a), BinaryOperator::LessEqual, Literal::Integer(b)) => {
                Some(Literal::Boolean(a <= b))
            }
            (Literal::Integer(a), BinaryOperator::GreaterEqual, Literal::Integer(b)) => {
                Some(Literal::Boolean(a >= b))
            }
            (Literal::Float(a), BinaryOperator::Add, Literal::Float(b)) => {
                Some(Literal::Float(a + b))
            }
            (Literal::Float(a), BinaryOperator::Subtract, Literal::Float(b)) => {
                Some(Literal::Float(a - b))
            }
            (Literal::Float(a), BinaryOperator::Multiply, Literal::Float(b)) => {
                Some(Literal::Float(a * b))
            }
            (Literal::Float(a), BinaryOperator::Divide, Literal::Float(b)) => {
                if *b != 0.0 {
                    Some(Literal::Float(a / b))
                } else {
                    None
                }
            }
            (Literal::String(a), BinaryOperator::Add, Literal::String(b)) => {
                Some(Literal::String(format!("{}{}", a, b)))
            }
            (Literal::Boolean(a), BinaryOperator::And, Literal::Boolean(b)) => {
                Some(Literal::Boolean(*a && *b))
            }
            (Literal::Boolean(a), BinaryOperator::Or, Literal::Boolean(b)) => {
                Some(Literal::Boolean(*a || *b))
            }
            _ => None,
        }
    }

    fn evaluate_unary_op(&self, op: &UnaryOperator, operand: &Literal) -> Option<Literal> {
        match (op, operand) {
            (UnaryOperator::Not, Literal::Boolean(b)) => Some(Literal::Boolean(!b)),
            (UnaryOperator::Minus, Literal::Integer(n)) => Some(Literal::Integer(-n)),
            (UnaryOperator::Minus, Literal::Float(n)) => Some(Literal::Float(-n)),
            _ => None,
        }
    }

    /// Dead code elimination - remove unreachable code
    fn dead_code_elimination(&self, program: &mut Program) {
        // Remove statements after return/break/continue
        for statement in &mut program.statements {
            self.eliminate_dead_code_statement(statement);
        }
    }

    fn eliminate_dead_code_statement(&self, statement: &mut Statement) {
        match statement {
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => {
                // Remove unreachable code after return in branches
                self.remove_after_return(then_branch);
                if let Some(else_body) = else_branch {
                    self.remove_after_return(else_body);
                }
            }
            Statement::While { body, .. } => {
                self.remove_after_return(body);
            }
            Statement::DoWhile { body, .. } => {
                self.remove_after_return(body);
            }
            Statement::For { body, .. } => {
                self.remove_after_return(body);
            }
            Statement::Repeat { body, .. } => {
                self.remove_after_return(body);
            }
            _ => {}
        }
    }

    fn remove_after_return(&self, statements: &mut Vec<Statement>) {
        let mut return_index = None;
        for (i, stmt) in statements.iter().enumerate() {
            if matches!(
                stmt,
                Statement::Return { .. } | Statement::Break { .. } | Statement::Continue { .. }
            ) {
                return_index = Some(i);
                break;
            }
        }

        if let Some(index) = return_index {
            statements.truncate(index + 1);
        }
    }

    // Function inlining and loop optimization removed - not needed for cyber orchestration
    // Keep optimizer focused on essential optimizations only

    /// Peephole optimization on bytecode
    fn peephole_optimization(&self, bytecode: &mut Bytecode) {
        use crate::compiler::bytecode::{Constant, Instruction};

        // Pass 1: remove Nop instructions
        bytecode
            .instructions
            .retain(|inst| !matches!(inst, Instruction::Nop));

        // Pass 2: constant folding — PushConst(a) PushConst(b) <arith-op> → PushConst(result)
        let mut i = 0;
        while i + 2 < bytecode.instructions.len() {
            let folded: Option<Constant> = match (
                &bytecode.instructions[i],
                &bytecode.instructions[i + 1],
                &bytecode.instructions[i + 2],
            ) {
                (
                    Instruction::PushConstant(ia),
                    Instruction::PushConstant(ib),
                    op,
                ) => {
                    let ca = bytecode.constants.get(*ia);
                    let cb = bytecode.constants.get(*ib);
                    match (ca, cb, op) {
                        (Some(Constant::Integer(a)), Some(Constant::Integer(b)), Instruction::Add) =>
                            Some(Constant::Integer(a.wrapping_add(*b))),
                        (Some(Constant::Integer(a)), Some(Constant::Integer(b)), Instruction::Subtract) =>
                            Some(Constant::Integer(a.wrapping_sub(*b))),
                        (Some(Constant::Integer(a)), Some(Constant::Integer(b)), Instruction::Multiply) =>
                            Some(Constant::Integer(a.wrapping_mul(*b))),
                        (Some(Constant::Float(a)), Some(Constant::Float(b)), Instruction::Add) =>
                            Some(Constant::Float(a + b)),
                        (Some(Constant::Float(a)), Some(Constant::Float(b)), Instruction::Subtract) =>
                            Some(Constant::Float(a - b)),
                        (Some(Constant::Float(a)), Some(Constant::Float(b)), Instruction::Multiply) =>
                            Some(Constant::Float(a * b)),
                        _ => None,
                    }
                }
                _ => None,
            };

            if let Some(result_const) = folded {
                // Replace 3 instructions with one PushConstant(new_idx)
                let new_idx = bytecode.constants.len();
                bytecode.constants.push(result_const);
                bytecode.instructions.drain(i..i + 3);
                bytecode.instructions.insert(i, Instruction::PushConstant(new_idx));
                // Don't advance i — the new PushConstant might be folded further
            } else {
                i += 1;
            }
        }
    }
}

impl Default for Optimizer {
    fn default() -> Self {
        Self::new(OptimizationLevel::Basic)
    }
}
