// Syntax validation - checks for forbidden patterns and syntax violations

use super::ValidationError;
use crate::parser::ast::{BinaryOperator, Expression, Program, Statement};

pub struct SyntaxValidator;

impl SyntaxValidator {
    /// Check program for syntax violations.
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
            Statement::Assignment { value, .. } => {
                Self::check_expression(value)?;
            }
            Statement::FunctionDef { body, .. } => {
                for body_stmt in body {
                    Self::check_statement(body_stmt)?;
                }
            }
            Statement::Return { value: Some(expr), .. } => {
                Self::check_expression(expr)?;
            }
            Statement::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
                ..
            } => {
                Self::check_expression(condition)?;
                for stmt in then_branch {
                    Self::check_statement(stmt)?;
                }
                for (cond, branch) in else_if_branches {
                    Self::check_expression(cond)?;
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
            Statement::While { condition, body, .. }
            | Statement::DoWhile { condition, body, .. } => {
                Self::check_expression(condition)?;
                for stmt in body {
                    Self::check_statement(stmt)?;
                }
            }
            Statement::For { iterable, body, .. } => {
                Self::check_expression(iterable)?;
                for stmt in body {
                    Self::check_statement(stmt)?;
                }
            }
            Statement::Repeat { count, body, .. } => {
                Self::check_expression(count)?;
                for stmt in body {
                    Self::check_statement(stmt)?;
                }
            }
            Statement::IndexAssignment { target, index, value, .. } => {
                Self::check_expression(target)?;
                Self::check_expression(index)?;
                Self::check_expression(value)?;
            }
            Statement::CompoundAssignment { value, .. } => {
                Self::check_expression(value)?;
            }
            Statement::Assert { condition, message, .. } => {
                Self::check_expression(condition)?;
                if let Some(msg) = message {
                    Self::check_expression(msg)?;
                }
            }
            Statement::Const { value, .. } => {
                Self::check_expression(value)?;
            }
            Statement::NamedError { message, .. } => {
                Self::check_expression(message)?;
            }
            Statement::Enum { variants, .. } => {
                for (_, val) in variants {
                    if let Some(v) = val {
                        Self::check_expression(v)?;
                    }
                }
            }
            Statement::Match { value, cases, default, .. } => {
                Self::check_expression(value)?;
                for (_, guard, body) in cases {
                    if let Some(g) = guard {
                        Self::check_expression(g)?;
                    }
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
            _ => {}
        }
        Ok(())
    }

    fn check_expression(expr: &Expression) -> Result<(), ValidationError> {
        match expr {
            Expression::FunctionCall { name, arguments, .. } => {
                // eval() is never allowed.
                if name == "eval" {
                    return Err(ValidationError::Syntax(
                        "eval() is not allowed in Txtcode for security reasons".to_string(),
                    ));
                }

                // Detect command-injection pattern: exec/spawn/pipe_exec where
                // any argument is or contains a string concatenation with a
                // variable, or an interpolated string.
                //
                //   Flagged:  exec("nmap " + target)
                //   Flagged:  exec(cmd, user_arg)      ← any arg with identifier
                //   Allowed:  exec("nmap -sV -p 80")
                //
                // Concatenating a variable into a command string is the canonical
                // injection vector. Callers should validate/sanitise values first
                // and pass clean string literals.
                if matches!(name.as_str(), "exec" | "spawn" | "pipe_exec") {
                    for arg in arguments {
                        if Self::contains_add_with_identifier(arg) {
                            return Err(ValidationError::Syntax(format!(
                                "{}(): command argument must not concatenate variables with '+' \
                                 — this is a command-injection risk. \
                                 Validate the value first and pass a clean string literal.",
                                name
                            )));
                        }
                    }
                }

                // Recurse into all arguments.
                // The original code matched on FunctionCall but never recursed
                // into its arguments — meaning eval() inside a function arg was
                // invisible to the validator.
                for arg in arguments {
                    Self::check_expression(arg)?;
                }
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
            Expression::Set { elements, .. } => {
                for elem in elements {
                    Self::check_expression(elem)?;
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
            Expression::Index { target, index, .. } => {
                Self::check_expression(target)?;
                Self::check_expression(index)?;
            }
            Expression::Member { target, .. } => {
                Self::check_expression(target)?;
            }
            Expression::MethodCall { object, arguments, .. } => {
                Self::check_expression(object)?;
                for arg in arguments {
                    Self::check_expression(arg)?;
                }
            }
            Expression::OptionalCall { target, arguments, .. } => {
                Self::check_expression(target)?;
                for arg in arguments {
                    Self::check_expression(arg)?;
                }
            }
            Expression::OptionalMember { target, .. } => {
                Self::check_expression(target)?;
            }
            Expression::OptionalIndex { target, index, .. } => {
                Self::check_expression(target)?;
                Self::check_expression(index)?;
            }
            Expression::Slice { target, start, end, step, .. } => {
                Self::check_expression(target)?;
                if let Some(e) = start { Self::check_expression(e)?; }
                if let Some(e) = end   { Self::check_expression(e)?; }
                if let Some(e) = step  { Self::check_expression(e)?; }
            }
            Expression::Await { expression, .. } => {
                Self::check_expression(expression)?;
            }
            Expression::StructLiteral { fields, .. } => {
                for (_, val) in fields {
                    Self::check_expression(val)?;
                }
            }
            Expression::Spread { value, .. } => {
                Self::check_expression(value)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Returns `true` when `expr` is or contains a `+` BinaryOp where at
    /// least one operand is an `Identifier` (variable reference).
    ///
    /// This catches the classic injection patterns:
    ///   `"nmap " + target`   ← right operand is an identifier
    ///   `prefix + " -sV"`    ← left operand is an identifier
    ///   `a + " " + b`        ← chained — caught by recursion
    ///
    /// It does NOT flag `"nmap" + " -sV"` — both sides are string literals.
    fn contains_add_with_identifier(expr: &Expression) -> bool {
        match expr {
            Expression::BinaryOp { op: BinaryOperator::Add, left, right, .. } => {
                matches!(left.as_ref(), Expression::Identifier(_))
                    || matches!(right.as_ref(), Expression::Identifier(_))
                    || Self::contains_add_with_identifier(left)
                    || Self::contains_add_with_identifier(right)
            }
            // f-strings embed variables directly — also a potential injection
            // vector when passed to exec().
            Expression::InterpolatedString { .. } => true,
            _ => false,
        }
    }
}
