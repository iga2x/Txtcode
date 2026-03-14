// Tool standard library - safe wrapper for tool_exec()
// Replaces raw exec() with permission-checked tool execution

use crate::policy::PolicyEngine;
use crate::runtime::audit::{AIMetadata, AuditTrail};
use crate::runtime::tools::{ToolContext, ToolExecutor};
use crate::runtime::{RuntimeError, Value};
use crate::tools::logger::log_debug;

/// Tool standard library functions
pub struct ToolLib;

impl ToolLib {
    /// Call a tool library function.
    ///
    /// - `tool_exec`: requires `permission_checker`; uses `audit_trail` and `policy` when
    ///   provided. If `audit_trail` is `None`, execution is still permitted but the run is
    ///   logged to the debug channel only (best-effort — the VM layer must provide a real
    ///   audit trail for production use).
    /// - `tool_list` / `tool_info`: read-only metadata; no permission checker required.
    pub fn call_function(
        name: &str,
        args: &[Value],
        permission_checker: Option<&dyn crate::stdlib::permission_checker::PermissionChecker>,
        audit_trail: Option<&mut AuditTrail>,
        ai_metadata: Option<&AIMetadata>,
        policy: Option<&mut PolicyEngine>,
    ) -> Result<Value, RuntimeError> {
        match name {
            "tool_exec" => {
                if args.is_empty() {
                    return Err(RuntimeError::new(
                        "tool_exec() requires at least tool name argument".to_string(),
                    ));
                }

                // First argument is tool name
                let tool_name = match &args[0] {
                    Value::String(name) => name.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            "tool_exec() first argument must be tool name (string)".to_string(),
                        ))
                    }
                };

                // Remaining arguments are tool arguments.
                // If a single Array is passed (e.g. tool_exec("nmap", ["-sV", "host"])),
                // flatten it into individual string args.
                let tool_args: Vec<String> = {
                    let remaining: Vec<&Value> = args.iter().skip(1).collect();
                    if remaining.len() == 1 {
                        if let Value::Array(arr) = remaining[0] {
                            arr.iter().map(|v| v.to_string()).collect()
                        } else {
                            remaining.iter().map(|v| v.to_string()).collect()
                        }
                    } else {
                        remaining.iter().map(|v| v.to_string()).collect()
                    }
                };

                // Fail secure: tool execution without a permission checker is not allowed.
                let checker = permission_checker.ok_or_else(|| {
                    RuntimeError::new(
                        "tool_exec() requires a permission checker. \
                         Cannot execute tools without VM permission enforcement."
                            .to_string(),
                    )
                })?;

                log_debug(&format!(
                    "Executing tool '{}' with args: {:?}",
                    tool_name, tool_args
                ));

                let executor = ToolExecutor::new();
                let context = ToolContext::new();

                match audit_trail {
                    Some(audit) => {
                        // Full path: audit trail present — execute with logging and policy.
                        let result = executor.execute_tool(
                            &tool_name,
                            tool_args,
                            Some(context),
                            audit,
                            ai_metadata,
                            checker,
                            policy,
                        )?;
                        if result.success {
                            Ok(Value::String(result.output.stdout))
                        } else {
                            Err(RuntimeError::new(format!(
                                "Tool '{}' failed with exit code {}: {}",
                                tool_name, result.output.exit_code, result.output.stderr
                            )))
                        }
                    }
                    None => {
                        // Degraded path: no audit trail provided by VM.
                        // Log a debug warning and execute with a no-op audit trail.
                        // Production VM paths must supply a real audit trail.
                        log_debug(&format!(
                            "tool_exec: WARNING — no audit trail available for '{}'. \
                             Execution proceeds but is not audited.",
                            tool_name
                        ));
                        let mut fallback_audit = crate::runtime::audit::AuditTrail::new();
                        let result = executor.execute_tool(
                            &tool_name,
                            tool_args,
                            Some(context),
                            &mut fallback_audit,
                            ai_metadata,
                            checker,
                            policy,
                        )?;
                        if result.success {
                            Ok(Value::String(result.output.stdout))
                        } else {
                            Err(RuntimeError::new(format!(
                                "Tool '{}' failed with exit code {}: {}",
                                tool_name, result.output.exit_code, result.output.stderr
                            )))
                        }
                    }
                }
            }

            "tool_list" => {
                // List available tools — no permission check required (read-only metadata).
                let executor = ToolExecutor::new();
                let tools = executor.registry().list();
                let tool_names: Vec<Value> = tools
                    .iter()
                    .map(|t| Value::String(t.name.clone()))
                    .collect();
                Ok(Value::Array(tool_names))
            }

            "tool_info" => {
                // Get tool information — no permission check required (read-only metadata).
                if args.is_empty() {
                    return Err(RuntimeError::new(
                        "tool_info() requires tool name argument".to_string(),
                    ));
                }
                let tool_name = match &args[0] {
                    Value::String(name) => name.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            "tool_info() requires tool name (string)".to_string(),
                        ))
                    }
                };

                let executor = ToolExecutor::new();
                if let Some(tool) = executor.registry().get(&tool_name) {
                    use std::collections::HashMap;
                    let mut info = HashMap::new();
                    info.insert("name".to_string(), Value::String(tool.name.clone()));
                    info.insert("command".to_string(), Value::String(tool.command.clone()));
                    info.insert(
                        "description".to_string(),
                        Value::String(tool.description.clone()),
                    );
                    info.insert(
                        "requires_sudo".to_string(),
                        Value::Boolean(tool.requires_sudo),
                    );
                    info.insert(
                        "default_timeout".to_string(),
                        Value::Integer(tool.default_timeout as i64),
                    );
                    info.insert(
                        "allowed_actions".to_string(),
                        Value::Array(
                            tool.allowed_actions
                                .iter()
                                .map(|a| Value::String(a.clone()))
                                .collect(),
                        ),
                    );
                    Ok(Value::Map(info))
                } else {
                    Err(RuntimeError::new(format!(
                        "Tool '{}' not found in registry",
                        tool_name
                    )))
                }
            }

            _ => Err(RuntimeError::new(format!(
                "Unknown tool function: {}",
                name
            ))),
        }
    }
}
