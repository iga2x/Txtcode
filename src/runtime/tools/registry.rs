// Tool registry - manages available pentest tools

use super::{Tool, ToolCategory};
use crate::runtime::errors::RuntimeError;
use std::collections::HashMap;

/// Shell interpreter binaries that must never be registered as a tool command.
/// Path-qualified variants (e.g. "/bin/sh") are caught by basename extraction.
const SHELL_BINARIES: &[&str] = &[
    "sh", "bash", "zsh", "fish", "ksh", "csh", "tcsh", "dash",
    "cmd", "powershell", "pwsh",
];

/// Registry of available pentest tools
pub struct ToolRegistry {
    tools: HashMap<String, Tool>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
        };

        // Register common pentest tools (all known-safe, unwrap is intentional).
        registry.register_default_tools();

        registry
    }

    /// Register a tool.
    ///
    /// Returns `Err` if the tool's `command` field is a shell interpreter binary,
    /// since that would give any caller with process permissions a shell escape vector.
    pub fn register(&mut self, tool: Tool) -> Result<(), RuntimeError> {
        let cmd = tool.command.trim();
        // Extract basename so "/bin/sh" and "sh" are treated identically.
        let basename = std::path::Path::new(cmd)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(cmd);
        if SHELL_BINARIES.contains(&basename) {
            return Err(RuntimeError::new(format!(
                "Tool '{}': registering a shell interpreter ('{}') as a tool command is not \
                 permitted — it creates a shell escape vector. Use sys.exec() with explicit \
                 Process permission for one-off commands.",
                tool.name, cmd
            )));
        }
        self.tools.insert(tool.name.clone(), tool);
        Ok(())
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<&Tool> {
        self.tools.get(name)
    }

    /// List all registered tools
    pub fn list(&self) -> Vec<&Tool> {
        self.tools.values().collect()
    }

    /// Check if a tool is registered
    pub fn is_registered(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Register default pentest tools.
    /// All built-in commands are known-safe binaries; `unwrap()` here is intentional.
    fn register_default_tools(&mut self) {
        // Network scanning tools
        self.register(Tool {
            name: "nmap".to_string(),
            command: "nmap".to_string(),
            description: "Network mapper - port scanner and service detector".to_string(),
            category: ToolCategory::NetworkScanning,
            requires_sudo: false, // Can run without sudo for basic scans
            default_timeout: 300, // 5 minutes
            allowed_actions: vec!["scan".to_string(), "enum".to_string()],
        })
        .unwrap();

        self.register(Tool {
            name: "masscan".to_string(),
            command: "masscan".to_string(),
            description: "Fast port scanner".to_string(),
            category: ToolCategory::NetworkScanning,
            requires_sudo: true,  // Requires sudo for raw sockets
            default_timeout: 600, // 10 minutes
            allowed_actions: vec!["scan".to_string()],
        })
        .unwrap();

        // Web testing tools
        self.register(Tool {
            name: "nikto".to_string(),
            command: "nikto".to_string(),
            description: "Web server scanner".to_string(),
            category: ToolCategory::WebTesting,
            requires_sudo: false,
            default_timeout: 900, // 15 minutes
            allowed_actions: vec!["scan".to_string(), "test".to_string()],
        })
        .unwrap();

        self.register(Tool {
            name: "sqlmap".to_string(),
            command: "sqlmap".to_string(),
            description: "SQL injection tool".to_string(),
            category: ToolCategory::WebTesting,
            requires_sudo: false,
            default_timeout: 1800, // 30 minutes
            allowed_actions: vec!["test".to_string(), "exploit".to_string()],
        })
        .unwrap();

        // Password cracking tools
        self.register(Tool {
            name: "hydra".to_string(),
            command: "hydra".to_string(),
            description: "Network login cracker".to_string(),
            category: ToolCategory::PasswordCracking,
            requires_sudo: false,
            default_timeout: 3600, // 1 hour
            allowed_actions: vec!["crack".to_string()],
        })
        .unwrap();

        // NOTE: A generic "system" → "sh" tool is intentionally NOT registered.
        // Mapping a tool to a shell interpreter gives any caller with net.connect
        // or process permissions a shell escape vector. Tools must map to specific
        // named binaries. Use `sys.exec()` (which requires explicit `sys.exec`
        // permission and direct-argv execution) for one-off commands.
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
