// Tool orchestration interface - safe wrapper for external pentest tools
// Replaces raw exec() with permission-checked, auditable tool execution

pub mod executor;
pub mod registry;
pub mod result;

pub use executor::ToolExecutor;
pub use registry::ToolRegistry;
pub use result::{ToolOutput, ToolResult};

use crate::runtime::errors::RuntimeError;

/// Tool definition - describes a pentest tool that can be executed
#[derive(Debug, Clone, PartialEq)]
pub struct Tool {
    pub name: String,                 // Tool name (e.g., "nmap", "nikto", "hydra")
    pub command: String,              // Base command (e.g., "nmap")
    pub description: String,          // Human-readable description
    pub category: ToolCategory,       // Tool category
    pub requires_sudo: bool,          // Whether tool requires sudo
    pub default_timeout: u64,         // Default timeout in seconds
    pub allowed_actions: Vec<String>, // Allowed actions for this tool (e.g., ["scan", "enum"])
}

/// Tool category for grouping and policy enforcement
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolCategory {
    NetworkScanning,  // nmap, masscan, etc.
    WebTesting,       // nikto, sqlmap, etc.
    PasswordCracking, // hydra, john, etc.
    Exploitation,     // metasploit, etc.
    SystemInfo,       // system commands (ls, ps, etc.)
    Wireless,         // aircrack-ng, etc.
    Other,            // Other tools
}

/// Tool execution context - provides environment for tool execution
pub struct ToolContext {
    pub working_directory: Option<String>,
    pub environment_vars: Vec<(String, String)>,
    pub timeout: Option<u64>,
}

impl ToolContext {
    pub fn new() -> Self {
        Self {
            working_directory: None,
            environment_vars: Vec::new(),
            timeout: None,
        }
    }

    pub fn with_timeout(mut self, timeout: u64) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn with_cwd(mut self, cwd: String) -> Self {
        self.working_directory = Some(cwd);
        self
    }

    pub fn with_env(mut self, key: String, value: String) -> Self {
        self.environment_vars.push((key, value));
        self
    }
}

impl Default for ToolContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Tool permission check - determines if a tool can be executed.
///
/// Enforces tool-level invariants that do not require VM state:
/// - `requires_sudo` blocks privilege escalation unconditionally.
/// - `action` (if provided) is validated against `tool.allowed_actions`.
///   Pass `None` to skip action validation (e.g. when the caller has no
///   action context). An empty `allowed_actions` list means "all actions allowed".
///
/// The VM permission gate (PermissionChecker) is enforced separately in
/// ToolExecutor::execute_tool.
pub fn check_tool_permission(tool: &Tool, action: Option<&str>) -> Result<(), RuntimeError> {
    // Tools that require sudo cannot be safely executed inside the runtime
    // sandbox — privilege escalation is never permitted implicitly.
    if tool.requires_sudo {
        return Err(RuntimeError::new(format!(
            "Tool '{}' requires elevated privileges (sudo), which is not permitted \
             in the runtime sandbox. Grant explicit process permissions or run outside \
             the sandbox.",
            tool.name
        )));
    }

    // Validate requested action against the tool's allow-list.
    // An empty allowed_actions means no restriction on action.
    if let Some(act) = action {
        if !tool.allowed_actions.is_empty()
            && !tool.allowed_actions.iter().any(|a| a == act)
        {
            return Err(RuntimeError::new(format!(
                "Tool '{}': action '{}' is not permitted. Allowed actions: {:?}",
                tool.name, act, tool.allowed_actions
            )));
        }
    }

    Ok(())
}
