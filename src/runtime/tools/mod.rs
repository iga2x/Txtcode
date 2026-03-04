// Tool orchestration interface - safe wrapper for external pentest tools
// Replaces raw exec() with permission-checked, auditable tool execution

pub mod registry;
pub mod executor;
pub mod result;

pub use registry::ToolRegistry;
pub use executor::ToolExecutor;
pub use result::{ToolResult, ToolOutput};

use crate::runtime::errors::RuntimeError;
use crate::runtime::permissions::PermissionResource;

/// Tool definition - describes a pentest tool that can be executed
#[derive(Debug, Clone, PartialEq)]
pub struct Tool {
    pub name: String,              // Tool name (e.g., "nmap", "nikto", "hydra")
    pub command: String,            // Base command (e.g., "nmap")
    pub description: String,        // Human-readable description
    pub category: ToolCategory,     // Tool category
    pub requires_sudo: bool,        // Whether tool requires sudo
    pub default_timeout: u64,       // Default timeout in seconds
    pub allowed_actions: Vec<String>, // Allowed actions for this tool (e.g., ["scan", "enum"])
}

/// Tool category for grouping and policy enforcement
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolCategory {
    NetworkScanning,      // nmap, masscan, etc.
    WebTesting,          // nikto, sqlmap, etc.
    PasswordCracking,    // hydra, john, etc.
    Exploitation,        // metasploit, etc.
    SystemInfo,          // system commands (ls, ps, etc.)
    Wireless,            // aircrack-ng, etc.
    Other,               // Other tools
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

/// Tool permission check - determines if a tool can be executed
pub fn check_tool_permission(
    tool: &Tool,
    _resource: &PermissionResource,
    _scope: Option<&str>,
) -> Result<(), RuntimeError> {
    // Tool permission checking logic
    // For now, basic validation
    // Phase 4 will add AST-based capability checking
    
    // Check if tool requires sudo and if permission is granted
    if tool.requires_sudo {
        // Additional sudo permission check would go here
    }
    
    Ok(())
}

