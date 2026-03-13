// Semantic validation - checks for semantic errors before execution

use super::ValidationError;
use crate::parser::ast::{Program, Statement};
use std::collections::HashMap;

pub struct SemanticsValidator;

impl SemanticsValidator {
    /// Check program for semantic errors.
    pub fn check_program(program: &Program) -> Result<(), ValidationError> {
        Self::check_duplicate_functions(&program.statements)?;
        Self::check_return_outside_function(&program.statements, false)?;
        Ok(())
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
