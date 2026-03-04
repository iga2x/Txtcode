// Tool executor - executes tools with permission checking and audit logging

use super::{Tool, ToolContext, ToolRegistry, check_tool_permission};
use super::result::{ToolResult, ToolOutput};
use crate::runtime::errors::RuntimeError;
use crate::runtime::permissions::PermissionResource;
use crate::runtime::audit::{AuditTrail, AuditResult, AIMetadata};
use crate::tools::logger::log_debug;
use std::process::Command;
use std::time::{SystemTime, Duration};

/// Tool executor - executes pentest tools safely
pub struct ToolExecutor {
    registry: ToolRegistry,
}

impl ToolExecutor {
    pub fn new() -> Self {
        Self {
            registry: ToolRegistry::new(),
        }
    }

    pub fn with_registry(registry: ToolRegistry) -> Self {
        Self { registry }
    }

    /// Execute a tool by name with arguments
    pub fn execute_tool(
        &self,
        tool_name: &str,
        args: Vec<String>,
        context: Option<ToolContext>,
        audit_trail: Option<&mut AuditTrail>,
        ai_metadata: Option<&AIMetadata>,
    ) -> Result<ToolResult, RuntimeError> {
        // Get tool from registry
        let tool = self.registry.get(tool_name)
            .ok_or_else(|| RuntimeError::new(format!("Tool '{}' not found in registry", tool_name)))?;

        // Create execution context
        let ctx = context.unwrap_or_else(|| ToolContext::new().with_timeout(tool.default_timeout));

        // Check permissions
        let resource = PermissionResource::System("tool_exec".to_string());
        check_tool_permission(tool, &resource, None)?;

        // Log to audit trail if provided
        if let Some(audit) = audit_trail {
            let action = format!("tool.exec.{}", tool_name);
            let resource_str = format!("tool:{}", tool_name);
            let context_str = Some(format!("args:{}", args.join(" ")));
            let _ = audit.log_action(
                action,
                resource_str,
                context_str,
                AuditResult::Allowed,
                ai_metadata,
            );
        }

        log_debug(&format!("Executing tool '{}' with args: {:?}", tool_name, args));

        // Execute tool
        let start_time = SystemTime::now();
        let result = self.execute_command(tool, args, &ctx)?;
        let duration = start_time.elapsed().unwrap_or(Duration::ZERO);

        // Build tool result
        Ok(ToolResult {
            tool_name: tool_name.to_string(),
            success: result.exit_code == 0,
            output: result,
            duration: duration.as_secs(),
            timestamp: SystemTime::now(),
        })
    }

    /// Execute a command with context
    fn execute_command(
        &self,
        tool: &Tool,
        args: Vec<String>,
        context: &ToolContext,
    ) -> Result<ToolOutput, RuntimeError> {
        let mut command = Command::new(&tool.command);

        // Add arguments
        for arg in args {
            command.arg(arg);
        }

        // Set working directory if provided
        if let Some(ref cwd) = context.working_directory {
            command.current_dir(cwd);
        }

        // Set environment variables
        for (key, value) in &context.environment_vars {
            command.env(key, value);
        }

        // Set timeout if provided (using timeout command as fallback)
        let timeout = context.timeout.unwrap_or(tool.default_timeout);

        // Execute command
        let output = if timeout > 0 {
            // For now, just execute - timeout handling would use tokio::process in async context
            command.output()
                .map_err(|e| RuntimeError::new(format!("Tool execution failed: {}", e)))?
        } else {
            // No timeout - direct execution
            command.output()
                .map_err(|e| RuntimeError::new(format!("Tool execution failed: {}", e)))?
        };

        Ok(ToolOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }

    /// Get tool registry reference
    pub fn registry(&self) -> &ToolRegistry {
        &self.registry
    }

    /// Get tool registry mutable reference
    pub fn registry_mut(&mut self) -> &mut ToolRegistry {
        &mut self.registry
    }
}

impl Default for ToolExecutor {
    fn default() -> Self {
        Self::new()
    }
}

