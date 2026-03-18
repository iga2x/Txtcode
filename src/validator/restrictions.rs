// Restriction checking - security constraint validation for capability declarations

use super::ValidationError;
use crate::parser::ast::{CapabilityExpr, Expression, Program, Statement};

pub struct RestrictionChecker;

impl RestrictionChecker {
    /// Check program against security restrictions.
    pub fn check_program(program: &Program) -> Result<(), ValidationError> {
        for statement in &program.statements {
            Self::check_statement(statement)?;
        }
        Ok(())
    }

    fn check_statement(stmt: &Statement) -> Result<(), ValidationError> {
        match stmt {
            Statement::FunctionDef { name, allowed_actions, forbidden_actions, body, .. } => {
                // If the function body calls privileged operations, it should declare
                // the corresponding capabilities in `allowed_actions`.
                //
                // This is a WARNING-grade check: we emit an error only when a
                // *forbidden* action is actually called, not when an `allowed`
                // declaration is simply absent (that would break all existing scripts
                // that predate capability declarations).
                let called = Self::collect_privileged_calls(body);

                for call in &called {
                    let cap_needed = Self::required_capability(call);

                    // Hard error: function explicitly forbids a capability it is using.
                    if let Some(cap) = cap_needed {
                        for forbidden in forbidden_actions {
                            if Self::capability_matches(forbidden, cap) {
                                return Err(ValidationError::Restriction(format!(
                                    "Function '{}' forbids '{}' but its body calls '{}'. \
                                     Remove the `forbidden {}` declaration or remove the call.",
                                    name, cap, call, cap
                                )));
                            }
                        }
                    }
                }

                // Validate that allowed_actions strings are well-formed.
                for cap in allowed_actions.iter().chain(forbidden_actions.iter()) {
                    if let CapabilityExpr::Simple { resource, action, .. } = cap {
                        let known_resources = ["fs", "net", "sys", "process", "filesystem", "network", "system", "proc"];
                        if !known_resources.contains(&resource.as_str()) {
                            return Err(ValidationError::Restriction(format!(
                                "Function '{}': unknown capability resource '{}'. \
                                 Valid resources: fs, net, sys, process.",
                                name, resource
                            )));
                        }
                        if action.is_empty() {
                            return Err(ValidationError::Restriction(format!(
                                "Function '{}': capability '{}' has an empty action.",
                                name, resource
                            )));
                        }
                    }
                }

                // Recurse into nested function definitions.
                for body_stmt in body {
                    Self::check_statement(body_stmt)?;
                }
            }

            // Recurse into control-flow bodies.
            Statement::If { then_branch, else_if_branches, else_branch, .. } => {
                for s in then_branch { Self::check_statement(s)?; }
                for (_, b) in else_if_branches { for s in b { Self::check_statement(s)?; } }
                if let Some(b) = else_branch { for s in b { Self::check_statement(s)?; } }
            }
            Statement::While { body, .. }
            | Statement::DoWhile { body, .. }
            | Statement::For { body, .. }
            | Statement::Repeat { body, .. } => {
                for s in body { Self::check_statement(s)?; }
            }
            Statement::Try { body, catch, finally, .. } => {
                for s in body { Self::check_statement(s)?; }
                if let Some((_, b)) = catch { for s in b { Self::check_statement(s)?; } }
                if let Some(b) = finally { for s in b { Self::check_statement(s)?; } }
            }
            Statement::Match { cases, default, .. } => {
                for (_, _, b) in cases { for s in b { Self::check_statement(s)?; } }
                if let Some(b) = default { for s in b { Self::check_statement(s)?; } }
            }
            _ => {}
        }
        Ok(())
    }

    /// Public wrapper for use by the permissions report CLI command.
    pub fn collect_privileged_calls_pub(stmts: &[Statement]) -> Vec<String> {
        Self::collect_privileged_calls(stmts)
    }

    /// Public wrapper for use by the permissions report CLI command.
    pub fn required_capability_pub(fn_name: &str) -> Option<&'static str> {
        Self::required_capability(fn_name)
    }

    /// Collect names of all privileged stdlib functions called anywhere in `stmts`.
    fn collect_privileged_calls(stmts: &[Statement]) -> Vec<String> {
        let mut calls = Vec::new();
        for stmt in stmts {
            Self::collect_from_statement(stmt, &mut calls);
        }
        calls
    }

    fn collect_from_statement(stmt: &Statement, out: &mut Vec<String>) {
        match stmt {
            Statement::Expression(expr) | Statement::Return { value: Some(expr), .. } => {
                Self::collect_from_expression(expr, out);
            }
            Statement::Assignment { value, .. } => {
                Self::collect_from_expression(value, out);
            }
            Statement::FunctionDef { body, .. } => {
                for s in body { Self::collect_from_statement(s, out); }
            }
            Statement::If { condition, then_branch, else_if_branches, else_branch, .. } => {
                Self::collect_from_expression(condition, out);
                for s in then_branch { Self::collect_from_statement(s, out); }
                for (c, b) in else_if_branches {
                    Self::collect_from_expression(c, out);
                    for s in b { Self::collect_from_statement(s, out); }
                }
                if let Some(b) = else_branch { for s in b { Self::collect_from_statement(s, out); } }
            }
            Statement::While { condition, body, .. }
            | Statement::DoWhile { condition, body, .. } => {
                Self::collect_from_expression(condition, out);
                for s in body { Self::collect_from_statement(s, out); }
            }
            Statement::For { iterable, body, .. } => {
                Self::collect_from_expression(iterable, out);
                for s in body { Self::collect_from_statement(s, out); }
            }
            Statement::Repeat { count, body, .. } => {
                Self::collect_from_expression(count, out);
                for s in body { Self::collect_from_statement(s, out); }
            }
            Statement::IndexAssignment { target, index, value, .. } => {
                Self::collect_from_expression(target, out);
                Self::collect_from_expression(index, out);
                Self::collect_from_expression(value, out);
            }
            Statement::CompoundAssignment { value, .. } => {
                Self::collect_from_expression(value, out);
            }
            Statement::Assert { condition, message, .. } => {
                Self::collect_from_expression(condition, out);
                if let Some(m) = message { Self::collect_from_expression(m, out); }
            }
            Statement::Const { value, .. } => {
                Self::collect_from_expression(value, out);
            }
            Statement::NamedError { message, .. } => {
                Self::collect_from_expression(message, out);
            }
            Statement::Enum { variants, .. } => {
                for (_, v) in variants {
                    if let Some(val) = v { Self::collect_from_expression(val, out); }
                }
            }
            Statement::Try { body, catch, finally, .. } => {
                for s in body { Self::collect_from_statement(s, out); }
                if let Some((_, b)) = catch { for s in b { Self::collect_from_statement(s, out); } }
                if let Some(b) = finally { for s in b { Self::collect_from_statement(s, out); } }
            }
            Statement::Match { cases, default, .. } => {
                for (_, _, b) in cases { for s in b { Self::collect_from_statement(s, out); } }
                if let Some(b) = default { for s in b { Self::collect_from_statement(s, out); } }
            }
            _ => {}
        }
    }

    fn collect_from_expression(expr: &Expression, out: &mut Vec<String>) {
        match expr {
            Expression::FunctionCall { name, arguments, .. } => {
                if Self::required_capability(name).is_some() {
                    out.push(name.clone());
                }
                for arg in arguments {
                    Self::collect_from_expression(arg, out);
                }
            }
            Expression::BinaryOp { left, right, .. } => {
                Self::collect_from_expression(left, out);
                Self::collect_from_expression(right, out);
            }
            Expression::UnaryOp { operand, .. } => {
                Self::collect_from_expression(operand, out);
            }
            Expression::Array { elements, .. } | Expression::Set { elements, .. } => {
                for e in elements { Self::collect_from_expression(e, out); }
            }
            Expression::Map { entries, .. } => {
                for (k, v) in entries {
                    Self::collect_from_expression(k, out);
                    Self::collect_from_expression(v, out);
                }
            }
            Expression::Ternary { condition, true_expr, false_expr, .. } => {
                Self::collect_from_expression(condition, out);
                Self::collect_from_expression(true_expr, out);
                Self::collect_from_expression(false_expr, out);
            }
            Expression::Lambda { body, .. } => {
                Self::collect_from_expression(body, out);
            }
            Expression::MethodCall { object, arguments, .. } => {
                Self::collect_from_expression(object, out);
                for a in arguments { Self::collect_from_expression(a, out); }
            }
            Expression::Index { target, index, .. } => {
                Self::collect_from_expression(target, out);
                Self::collect_from_expression(index, out);
            }
            Expression::Member { target, .. } => {
                Self::collect_from_expression(target, out);
            }
            Expression::OptionalCall { target, arguments, .. } => {
                Self::collect_from_expression(target, out);
                for a in arguments { Self::collect_from_expression(a, out); }
            }
            Expression::OptionalMember { target, .. } => {
                Self::collect_from_expression(target, out);
            }
            Expression::OptionalIndex { target, index, .. } => {
                Self::collect_from_expression(target, out);
                Self::collect_from_expression(index, out);
            }
            Expression::Slice { target, start, end, step, .. } => {
                Self::collect_from_expression(target, out);
                if let Some(e) = start { Self::collect_from_expression(e, out); }
                if let Some(e) = end   { Self::collect_from_expression(e, out); }
                if let Some(e) = step  { Self::collect_from_expression(e, out); }
            }
            Expression::Spread { value, .. } => {
                Self::collect_from_expression(value, out);
            }
            Expression::StructLiteral { fields, .. } => {
                for (_, v) in fields { Self::collect_from_expression(v, out); }
            }
            Expression::Await { expression, .. } => {
                Self::collect_from_expression(expression, out);
            }
            _ => {}
        }
    }

    /// Maps a stdlib function name to the capability string it requires,
    /// or `None` if the function is unprivileged.
    fn required_capability(fn_name: &str) -> Option<&'static str> {
        match fn_name {
            "exec" | "exec_status" | "exec_lines" | "exec_json"
            | "spawn" | "pipe_exec" | "kill" | "signal_send" => Some("sys.exec"),
            "http_get" | "http_post" | "http_put" | "http_delete" | "http_patch"
            | "tcp_connect" | "udp_send" | "resolve" => Some("net.connect"),
            "write_file" | "append_file" | "delete" | "mkdir" | "rmdir"
            | "copy_file" | "move_file" | "symlink_create" | "csv_write" => Some("fs.write"),
            "read_file" | "file_exists" | "is_file" | "is_dir" | "list_dir"
            | "read_lines" | "csv_read" => Some("fs.read"),
            "setenv" | "getenv" | "env_list" => Some("sys.env"),
            "cpu_count" | "memory" | "disk_space" => Some("sys.info"),
            _ => None,
        }
    }

    /// Returns true if a CapabilityExpr covers the required capability string
    /// (e.g., `CapabilityExpr::Simple { resource: "sys", action: "exec" }` matches "sys.exec").
    fn capability_matches(cap: &CapabilityExpr, required: &str) -> bool {
        if let CapabilityExpr::Simple { resource, action, .. } = cap {
            let formed = format!("{}.{}", resource, action);
            // Normalize aliases
            let required_norm = required
                .replace("filesystem.", "fs.")
                .replace("network.", "net.")
                .replace("system.", "sys.");
            formed == required_norm
                || formed == required
        } else {
            false
        }
    }
}
