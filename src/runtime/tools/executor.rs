// Tool executor - executes tools with permission checking and audit logging

use super::result::{ToolOutput, ToolResult};
use super::{check_tool_permission, Tool, ToolContext, ToolRegistry};
use crate::policy::PolicyEngine;
use crate::runtime::audit::{AIMetadata, AuditResult, AuditTrail};
use crate::runtime::errors::RuntimeError;
use crate::runtime::permissions::PermissionResource;
use crate::stdlib::PermissionChecker;
use crate::tools::logger::log_debug;
use std::process::Command;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::time::{Duration, SystemTime};

/// Maximum bytes captured per stream (stdout or stderr) from a single tool execution.
/// Output exceeding this limit is truncated to prevent OOM on verbose tools.
const MAX_TOOL_OUTPUT_BYTES: usize = 10 * 1024 * 1024; // 10 MiB

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

    /// Execute a tool by name with arguments.
    ///
    /// `permission_checker` and `audit_trail` are required.
    /// `policy` enforces rate limits, max execution time, and AI allowance before
    /// the tool runs. When `None`, policy checks are skipped (use only in tests).
    ///
    /// The audit trail entry is written **after** execution completes, recording the
    /// actual outcome (`Allowed` on exit-code 0, `Denied` on nonzero exit or error).
    pub fn execute_tool(
        &self,
        tool_name: &str,
        args: Vec<String>,
        context: Option<ToolContext>,
        audit_trail: &mut AuditTrail,
        ai_metadata: Option<&AIMetadata>,
        permission_checker: &dyn PermissionChecker,
        policy: Option<&mut PolicyEngine>,
    ) -> Result<ToolResult, RuntimeError> {
        // Get tool from registry
        let tool = self.registry.get(tool_name).ok_or_else(|| {
            RuntimeError::new(format!("Tool '{}' not found in registry", tool_name))
        })?;

        // Create execution context
        let ctx = context.unwrap_or_else(|| ToolContext::new().with_timeout(tool.default_timeout));

        // ── PolicyEngine checks (rate limit, max exec time, AI allowance) ──────
        if let Some(eng) = policy {
            eng.check_max_execution_time()
                .map_err(|e| RuntimeError::new(format!("Policy error: {}", e)))?;
            eng.check_rate_limit(&format!("tool.exec.{}", tool_name))
                .map_err(|e| RuntimeError::new(format!("Policy error: {}", e)))?;
            if ai_metadata.is_some() {
                eng.check_ai_allowed()
                    .map_err(|e| RuntimeError::new(format!("Policy error: {}", e)))?;
            }
        }

        // Tool-level invariant check (requires_sudo, allowed_actions).
        // No action context at this call-site — action filtering is the caller's responsibility.
        check_tool_permission(tool, None)?;

        // VM permission check — enforces the grant/deny policy set on the VM.
        let resource = PermissionResource::Process(vec![tool.command.clone()]);
        permission_checker.check_permission(&resource, Some(tool_name))?;

        log_debug(&format!(
            "Executing tool '{}' with args: {:?}",
            tool_name, args
        ));

        // Execute tool and record the actual outcome in the audit trail.
        let start_time = SystemTime::now();
        let cmd_result = self.execute_command(tool, args.clone(), &ctx);
        let duration = start_time.elapsed().unwrap_or(Duration::ZERO);

        // Audit entry written after execution — reflects the real outcome.
        {
            let action = format!("tool.exec.{}", tool_name);
            let resource_str = format!("tool:{}", tool_name);
            let context_str = Some(format!("args:{}", args.join(" ")));
            let outcome = match &cmd_result {
                Ok(out) if out.exit_code == 0 => AuditResult::Allowed,
                _ => AuditResult::Denied,
            };
            let _ = audit_trail.log_action(
                action,
                resource_str,
                context_str,
                outcome,
                ai_metadata,
            );
        }

        let result = cmd_result?;

        Ok(ToolResult {
            tool_name: tool_name.to_string(),
            success: result.exit_code == 0,
            output: result,
            duration: duration.as_secs(),
            timestamp: SystemTime::now(),
        })
    }

    /// Execute a command with context, enforcing the configured timeout.
    ///
    /// Timeout is enforced via a background watcher thread + `AtomicBool` cancel flag.
    /// When the deadline expires the child process is killed (SIGKILL on Unix) and an
    /// error is returned. When `timeout == 0` execution is unbounded.
    ///
    /// Output is capped at [`MAX_TOOL_OUTPUT_BYTES`] per stream; excess is truncated.
    fn execute_command(
        &self,
        tool: &Tool,
        args: Vec<String>,
        context: &ToolContext,
    ) -> Result<ToolOutput, RuntimeError> {
        let timeout_secs = context.timeout.unwrap_or(tool.default_timeout);

        // Build the child process (not yet spawned).
        let mut command = Command::new(&tool.command);
        for arg in &args {
            command.arg(arg);
        }
        if let Some(ref cwd) = context.working_directory {
            command.current_dir(cwd);
        }
        for (key, value) in &context.environment_vars {
            command.env(key, value);
        }

        if timeout_secs == 0 {
            // No timeout — execute directly.
            let output = command
                .output()
                .map_err(|e| RuntimeError::new(format!("Tool execution failed: {}", e)))?;
            return Ok(ToolOutput {
                stdout: truncate_output(&output.stdout),
                stderr: truncate_output(&output.stderr),
                exit_code: output.status.code().unwrap_or(-1),
            });
        }

        // Spawn child and enforce timeout via a watcher thread.
        let child = command
            .spawn()
            .map_err(|e| RuntimeError::new(format!("Tool execution failed: {}", e)))?;

        let child_id = child.id();
        let deadline = Duration::from_secs(timeout_secs);
        let timed_out = Arc::new(AtomicBool::new(false));
        let timed_out_watcher = Arc::clone(&timed_out);

        // Watcher: sleep for the deadline, then kill the child if still running.
        // Uses Release ordering on store so the load below observes the flag correctly.
        std::thread::spawn(move || {
            std::thread::sleep(deadline);
            timed_out_watcher.store(true, Ordering::Release);
            // Best-effort kill via platform signal.
            #[cfg(unix)]
            unsafe {
                libc::kill(child_id as libc::pid_t, libc::SIGKILL);
            }
            #[cfg(not(unix))]
            {
                // On non-Unix platforms kill-by-pid is not straightforward from a
                // background thread. The process will be reaped when wait_with_output
                // returns. The timeout flag is still set so the error path fires.
                let _ = child_id;
            }
        });

        let output = child
            .wait_with_output()
            .map_err(|e| RuntimeError::new(format!("Tool execution failed: {}", e)))?;

        // Acquire ordering ensures we see the watcher's Release store.
        if timed_out.load(Ordering::Acquire) {
            return Err(RuntimeError::new(format!(
                "Tool '{}' timed out after {} seconds",
                tool.name, timeout_secs
            )));
        }

        Ok(ToolOutput {
            stdout: truncate_output(&output.stdout),
            stderr: truncate_output(&output.stderr),
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

/// Converts raw output bytes to a UTF-8 string, capping at [`MAX_TOOL_OUTPUT_BYTES`].
/// Bytes beyond the cap are dropped and a notification suffix is appended.
fn truncate_output(bytes: &[u8]) -> String {
    if bytes.len() > MAX_TOOL_OUTPUT_BYTES {
        let mut s = String::from_utf8_lossy(&bytes[..MAX_TOOL_OUTPUT_BYTES]).to_string();
        s.push_str("\n[output truncated: exceeded 10 MiB limit]");
        s
    } else {
        String::from_utf8_lossy(bytes).to_string()
    }
}

impl Default for ToolExecutor {
    fn default() -> Self {
        Self::new()
    }
}
