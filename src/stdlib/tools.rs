// Tool standard library - safe wrapper for tool_exec()
// Replaces raw exec() with permission-checked tool execution

use crate::runtime::permissions::PermissionResource;
use crate::runtime::tools::{ToolContext, ToolExecutor};
use crate::runtime::{RuntimeError, Value};
use crate::tools::logger::log_debug;

/// Tool standard library functions
pub struct ToolLib;

impl ToolLib {
    /// Call a tool library function
    /// This provides a safe interface for executing pentest tools
    pub fn call_function(
        name: &str,
        args: &[Value],
        permission_checker: Option<&dyn crate::stdlib::permission_checker::PermissionChecker>,
        audit_trail: Option<&mut crate::runtime::audit::AuditTrail>,
        ai_metadata: Option<&crate::runtime::audit::AIMetadata>,
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

                // Remaining arguments are tool arguments
                // If a single Array is passed (e.g. tool_exec("system", ["-c", "cmd"])),
                // flatten it into individual string args
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

                // Check permission if checker is available
                if let Some(checker) = permission_checker {
                    checker.check_permission(
                        &PermissionResource::System("tool_exec".to_string()),
                        Some(&tool_name),
                    )?;
                }

                log_debug(&format!(
                    "Executing tool '{}' with args: {:?}",
                    tool_name, tool_args
                ));

                // Create tool executor
                let executor = ToolExecutor::new();
                let context = ToolContext::new();

                // Execute tool — pass permission_checker so the executor can
                // enforce the finer-grained Process permission as well.
                let result = executor.execute_tool(
                    &tool_name,
                    tool_args,
                    Some(context),
                    audit_trail,
                    ai_metadata,
                    permission_checker,
                )?;

                // Return result based on success
                if result.success {
                    Ok(Value::String(result.output.stdout))
                } else {
                    Err(RuntimeError::new(format!(
                        "Tool '{}' failed with exit code {}: {}",
                        tool_name, result.output.exit_code, result.output.stderr
                    )))
                }
            }
            "tool_list" => {
                // List available tools
                let executor = ToolExecutor::new();
                let tools = executor.registry().list();

                let tool_names: Vec<Value> = tools
                    .iter()
                    .map(|t| Value::String(t.name.clone()))
                    .collect();

                Ok(Value::Array(tool_names))
            }
            "tool_info" => {
                // Get tool information
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
