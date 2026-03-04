// Syntax validation - checks for forbidden patterns and syntax violations

use crate::parser::ast::{Program, Statement, Expression};
use super::ValidationError;

pub struct SyntaxValidator;

impl SyntaxValidator {
    /// Check program for syntax violations
    pub fn check_program(program: &Program) -> Result<(), ValidationError> {
        for statement in &program.statements {
            Self::check_statement(statement)?;
        }
        Ok(())
    }
    
    fn check_statement(stmt: &Statement) -> Result<(), ValidationError> {
        match stmt {
            Statement::Expression(expr) => {
                Self::check_expression(expr)?;
            }
            Statement::FunctionDef { body, .. } => {
                for body_stmt in body {
                    Self::check_statement(body_stmt)?;
                }
            }
            Statement::If { then_branch, else_if_branches, else_branch, .. } => {
                for stmt in then_branch {
                    Self::check_statement(stmt)?;
                }
                for (_, branch) in else_if_branches {
                    for stmt in branch {
                        Self::check_statement(stmt)?;
                    }
                }
                if let Some(branch) = else_branch {
                    for stmt in branch {
                        Self::check_statement(stmt)?;
                    }
                }
            }
            Statement::While { body, .. } | Statement::DoWhile { body, .. } |
            Statement::For { body, .. } | Statement::Repeat { body, .. } => {
                for stmt in body {
                    Self::check_statement(stmt)?;
                }
            }
            Statement::Match { cases, default, .. } => {
                for (_, _, body) in cases {
                    for stmt in body {
                        Self::check_statement(stmt)?;
                    }
                }
                if let Some(body) = default {
                    for stmt in body {
                        Self::check_statement(stmt)?;
                    }
                }
            }
            Statement::Try { body, catch, finally, .. } => {
                for stmt in body {
                    Self::check_statement(stmt)?;
                }
                if let Some((_, body)) = catch {
                    for stmt in body {
                        Self::check_statement(stmt)?;
                    }
                }
                if let Some(body) = finally {
                    for stmt in body {
                        Self::check_statement(stmt)?;
                    }
                }
            }
            _ => {
                // Other statements don't need syntax checking
            }
        }
        Ok(())
    }
    
    fn check_expression(expr: &Expression) -> Result<(), ValidationError> {
        match expr {
            Expression::FunctionCall { name, .. } => {
                // Reject eval() calls - not allowed in Txtcode
                if name == "eval" {
                    return Err(ValidationError::Syntax(
                        "eval() function is not allowed in Txtcode for security reasons".to_string()
                    ));
                }
                // Check other dangerous patterns
            }
            Expression::BinaryOp { left, right, .. } => {
                Self::check_expression(left)?;
                Self::check_expression(right)?;
            }
            Expression::UnaryOp { operand, .. } => {
                Self::check_expression(operand)?;
            }
            Expression::Array { elements, .. } => {
                for elem in elements {
                    Self::check_expression(elem)?;
                }
            }
            Expression::Map { entries, .. } => {
                for (key, value) in entries {
                    Self::check_expression(key)?;
                    Self::check_expression(value)?;
                }
            }
            Expression::Lambda { body, .. } => {
                Self::check_expression(body)?;
            }
            Expression::Ternary { condition, true_expr, false_expr, .. } => {
                Self::check_expression(condition)?;
                Self::check_expression(true_expr)?;
                Self::check_expression(false_expr)?;
            }
            _ => {
                // Other expressions are fine
            }
        }
        Ok(())
    }
}

