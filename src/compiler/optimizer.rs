use crate::compiler::bytecode::Bytecode;
use crate::parser::ast::*;

/// Bytecode-level peephole optimizer.
///
/// AST-level constant folding has moved to `src/ir/builder.rs` (`IrBuilder::lower()`),
/// which runs when the `ir` feature is enabled.  This optimizer retains only
/// bytecode-specific passes (Nop removal, stack-level constant folding).
pub struct Optimizer {
    optimization_level: OptimizationLevel,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OptimizationLevel {
    None,
    Basic, // Dead code elimination (AST) + peephole (bytecode)
}

impl Optimizer {
    pub fn new(level: OptimizationLevel) -> Self {
        Self {
            optimization_level: level,
        }
    }

    /// Optimize AST: remove unreachable code after early-exit statements.
    ///
    /// Constant folding at AST level has been moved to `IrBuilder::lower()`
    /// (enabled with `--features ir`).
    pub fn optimize_ast(&self, program: &mut Program) {
        match self.optimization_level {
            OptimizationLevel::None => {}
            OptimizationLevel::Basic => {
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

    /// Dead code elimination — remove unreachable code after early-exit statements.
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
