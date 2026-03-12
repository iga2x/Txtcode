// Restriction checking - pentest-specific rules and constraints

use super::ValidationError;
use crate::parser::ast::{Program, Statement};

pub struct RestrictionChecker;

impl RestrictionChecker {
    /// Check program against pentest-specific restrictions
    pub fn check_program(program: &Program) -> Result<(), ValidationError> {
        for statement in &program.statements {
            Self::check_statement(statement)?;
        }
        Ok(())
    }

    fn check_statement(stmt: &Statement) -> Result<(), ValidationError> {
        match stmt {
            Statement::FunctionDef {
                name: _name,
                allowed_actions,
                forbidden_actions,
                body,
                ..
            } => {
                // Check capability declarations are not empty
                if allowed_actions.is_empty() && forbidden_actions.is_empty() {
                    // Note: Empty capabilities are allowed - they just mean no explicit restrictions
                    // We could warn about this in the future, but it's not an error
                }

                // Validate capability expressions (already validated during parsing, but double-check)
                for cap in allowed_actions {
                    // Validate capability format - all valid formats are handled in CapabilityExpr::from_string
                    // Additional validation could go here (e.g., check resource exists, action is valid)
                    let _ = cap; // Use variable to avoid unused warning
                }
                for cap in forbidden_actions {
                    let _ = cap; // Use variable to avoid unused warning
                }

                // Check nested functions
                for body_stmt in body {
                    Self::check_statement(body_stmt)?;
                }

                // Future restrictions:
                // - Reject functions without intent declarations in AI-generated code
                // - Require timeout declarations for network operations
                // - Validate capability scope matches usage
            }
            Statement::If { .. }
            | Statement::While { .. }
            | Statement::DoWhile { .. }
            | Statement::For { .. }
            | Statement::Repeat { .. }
            | Statement::Match { .. }
            | Statement::Try { .. } => {
                // Recursively check nested statements
                for stmt in Self::extract_nested_statements(stmt) {
                    Self::check_statement(&stmt)?;
                }
            }
            _ => {
                // Other statements don't need restriction checking
            }
        }
        Ok(())
    }

    fn extract_nested_statements(stmt: &Statement) -> Vec<Statement> {
        match stmt {
            Statement::If {
                then_branch,
                else_if_branches,
                else_branch,
                ..
            } => {
                let mut result = then_branch.clone();
                for (_, branch) in else_if_branches {
                    result.extend(branch.clone());
                }
                if let Some(branch) = else_branch {
                    result.extend(branch.clone());
                }
                result
            }
            Statement::While { body, .. }
            | Statement::DoWhile { body, .. }
            | Statement::For { body, .. }
            | Statement::Repeat { body, .. } => body.clone(),
            Statement::Match { cases, default, .. } => {
                let mut result = Vec::new();
                for (_, _, body) in cases {
                    result.extend(body.clone());
                }
                if let Some(body) = default {
                    result.extend(body.clone());
                }
                result
            }
            Statement::Try {
                body,
                catch,
                finally,
                ..
            } => {
                let mut result = body.clone();
                if let Some((_, body)) = catch {
                    result.extend(body.clone());
                }
                if let Some(body) = finally {
                    result.extend(body.clone());
                }
                result
            }
            Statement::FunctionDef { body, .. } => body.clone(),
            _ => Vec::new(),
        }
    }
}
