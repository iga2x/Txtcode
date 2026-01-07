use crate::parser::ast::*;
use crate::compiler::bytecode::Bytecode;

/// Code optimizer
pub struct Optimizer {
    optimization_level: OptimizationLevel,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OptimizationLevel {
    None,
    Basic,
    Aggressive,
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
            OptimizationLevel::Aggressive => {
                self.constant_folding(program);
                self.dead_code_elimination(program);
                self.inline_functions(program);
                self.loop_optimization(program);
            }
        }
    }

    /// Optimize bytecode
    pub fn optimize_bytecode(&self, bytecode: &mut Vec<Bytecode>) {
        match self.optimization_level {
            OptimizationLevel::None => {}
            OptimizationLevel::Basic => {
                self.peephole_optimization(bytecode);
            }
            OptimizationLevel::Aggressive => {
                self.peephole_optimization(bytecode);
                self.constant_propagation_bytecode(bytecode);
            }
        }
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
            Statement::If { condition, then_branch, else_if_branches, else_branch, .. } => {
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
            Statement::While { condition, body, .. } => {
                *condition = self.fold_constants_expression(condition);
                for stmt in body {
                    self.fold_constants_statement(stmt);
                }
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
            Statement::Return { value, .. } => {
                if let Some(expr) = value {
                    *expr = self.fold_constants_expression(expr);
                }
            }
            Statement::Expression(expr) => {
                *expr = self.fold_constants_expression(expr);
            }
            _ => {}
        }
    }

    fn fold_constants_expression(&self, expr: &Expression) -> Expression {
        match expr {
            Expression::BinaryOp { left, op, right, span } => {
                let left_folded = self.fold_constants_expression(left);
                let right_folded = self.fold_constants_expression(right);

                // Try to evaluate if both are literals
                if let (Expression::Literal(left_lit), Expression::Literal(right_lit)) = (&left_folded, &right_folded) {
                    if let Some(result) = self.evaluate_binary_op(left_lit, op, right_lit) {
                        return Expression::Literal(result);
                    }
                }

                Expression::BinaryOp {
                    left: Box::new(left_folded),
                    op: op.clone(),
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
                    op: op.clone(),
                    operand: Box::new(operand_folded),
                    span: span.clone(),
                }
            }
            Expression::Array { elements, span } => {
                Expression::Array {
                    elements: elements.iter().map(|e| self.fold_constants_expression(e)).collect(),
                    span: span.clone(),
                }
            }
            Expression::Map { entries, span } => {
                Expression::Map {
                    entries: entries.iter().map(|(k, v)| (k.clone(), self.fold_constants_expression(v))).collect(),
                    span: span.clone(),
                }
            }
            _ => expr.clone(),
        }
    }

    fn evaluate_binary_op(&self, left: &Literal, op: &BinaryOperator, right: &Literal) -> Option<Literal> {
        match (left, op, right) {
            (Literal::Integer(a), BinaryOperator::Add, Literal::Integer(b)) => Some(Literal::Integer(a + b)),
            (Literal::Integer(a), BinaryOperator::Subtract, Literal::Integer(b)) => Some(Literal::Integer(a - b)),
            (Literal::Integer(a), BinaryOperator::Multiply, Literal::Integer(b)) => Some(Literal::Integer(a * b)),
            (Literal::Integer(a), BinaryOperator::Divide, Literal::Integer(b)) => {
                if *b != 0 { Some(Literal::Integer(a / b)) } else { None }
            }
            (Literal::Integer(a), BinaryOperator::Modulo, Literal::Integer(b)) => {
                if *b != 0 { Some(Literal::Integer(a % b)) } else { None }
            }
            (Literal::Integer(a), BinaryOperator::Power, Literal::Integer(b)) => {
                Some(Literal::Integer((*a as f64).powi(*b as i32) as i64))
            }
            (Literal::Integer(a), BinaryOperator::Equal, Literal::Integer(b)) => Some(Literal::Boolean(a == b)),
            (Literal::Integer(a), BinaryOperator::NotEqual, Literal::Integer(b)) => Some(Literal::Boolean(a != b)),
            (Literal::Integer(a), BinaryOperator::Less, Literal::Integer(b)) => Some(Literal::Boolean(a < b)),
            (Literal::Integer(a), BinaryOperator::Greater, Literal::Integer(b)) => Some(Literal::Boolean(a > b)),
            (Literal::Integer(a), BinaryOperator::LessEqual, Literal::Integer(b)) => Some(Literal::Boolean(a <= b)),
            (Literal::Integer(a), BinaryOperator::GreaterEqual, Literal::Integer(b)) => Some(Literal::Boolean(a >= b)),
            (Literal::Float(a), BinaryOperator::Add, Literal::Float(b)) => Some(Literal::Float(a + b)),
            (Literal::Float(a), BinaryOperator::Subtract, Literal::Float(b)) => Some(Literal::Float(a - b)),
            (Literal::Float(a), BinaryOperator::Multiply, Literal::Float(b)) => Some(Literal::Float(a * b)),
            (Literal::Float(a), BinaryOperator::Divide, Literal::Float(b)) => {
                if *b != 0.0 { Some(Literal::Float(a / b)) } else { None }
            }
            (Literal::String(a), BinaryOperator::Add, Literal::String(b)) => {
                Some(Literal::String(format!("{}{}", a, b)))
            }
            (Literal::Boolean(a), BinaryOperator::And, Literal::Boolean(b)) => Some(Literal::Boolean(*a && *b)),
            (Literal::Boolean(a), BinaryOperator::Or, Literal::Boolean(b)) => Some(Literal::Boolean(*a || *b)),
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
            Statement::If { then_branch, else_branch, .. } => {
                // Remove unreachable code after return in branches
                self.remove_after_return(then_branch);
                if let Some(else_body) = else_branch {
                    self.remove_after_return(else_body);
                }
            }
            Statement::While { body, .. } => {
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
            if matches!(stmt, Statement::Return { .. } | Statement::Break { .. } | Statement::Continue { .. }) {
                return_index = Some(i);
                break;
            }
        }
        
        if let Some(index) = return_index {
            statements.truncate(index + 1);
        }
    }

    /// Function inlining (aggressive optimization)
    fn inline_functions(&self, _program: &mut Program) {
        // Function inlining would be implemented here
        // This is a complex optimization that requires careful analysis
    }

    /// Loop optimization
    fn loop_optimization(&self, program: &mut Program) {
        for statement in &mut program.statements {
            self.optimize_loop(statement);
        }
    }

    fn optimize_loop(&self, statement: &mut Statement) {
        match statement {
            Statement::While { condition, body: _body, .. } => {
                // Check if condition is always true/false
                if let Expression::Literal(Literal::Boolean(false)) = condition {
                    // Loop never executes - remove it
                    *statement = Statement::Expression(Expression::Literal(Literal::Null));
                }
            }
            Statement::Repeat { count, .. } => {
                // Check if count is 0 or negative
                if let Expression::Literal(Literal::Integer(n)) = count {
                    if *n <= 0 {
                        *statement = Statement::Expression(Expression::Literal(Literal::Null));
                    }
                }
            }
            _ => {}
        }
    }

    /// Peephole optimization on bytecode
    fn peephole_optimization(&self, bytecode: &mut Vec<Bytecode>) {
        let mut i = 0;
        while i < bytecode.len().saturating_sub(1) {
            // Remove redundant operations
            match (&bytecode[i], &bytecode[i + 1]) {
                (Bytecode::PushInt(0), Bytecode::Add) => {
                    // 0 + x = x
                    bytecode.remove(i);
                    bytecode.remove(i);
                    continue;
                }
                (Bytecode::PushInt(0), Bytecode::Subtract) => {
                    // x - 0 = x, but we need to swap
                    bytecode.remove(i);
                    bytecode.remove(i);
                    continue;
                }
                (Bytecode::PushInt(1), Bytecode::Multiply) => {
                    // 1 * x = x
                    bytecode.remove(i);
                    bytecode.remove(i);
                    continue;
                }
                (Bytecode::PushInt(0), Bytecode::Multiply) => {
                    // 0 * x = 0
                    bytecode.remove(i);
                    bytecode.remove(i);
                    bytecode.insert(i, Bytecode::PushInt(0));
                    continue;
                }
                _ => {}
            }
            i += 1;
        }
    }

    /// Constant propagation in bytecode
    fn constant_propagation_bytecode(&self, _bytecode: &mut Vec<Bytecode>) {
        // Constant propagation would track constant values through bytecode
        // This is a more advanced optimization
    }
}

impl Default for Optimizer {
    fn default() -> Self {
        Self::new(OptimizationLevel::Basic)
    }
}
