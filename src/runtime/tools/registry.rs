// Tool registry - manages available pentest tools

use super::{Tool, ToolCategory};
use std::collections::HashMap;

/// Registry of available pentest tools
pub struct ToolRegistry {
    tools: HashMap<String, Tool>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
        };

        // Register common pentest tools
        registry.register_default_tools();

        registry
    }

    /// Register a tool
    pub fn register(&mut self, tool: Tool) {
        self.tools.insert(tool.name.clone(), tool);
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

    /// Register default pentest tools
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
        });

        self.register(Tool {
            name: "masscan".to_string(),
            command: "masscan".to_string(),
            description: "Fast port scanner".to_string(),
            category: ToolCategory::NetworkScanning,
            requires_sudo: true,  // Requires sudo for raw sockets
            default_timeout: 600, // 10 minutes
            allowed_actions: vec!["scan".to_string()],
        });

        // Web testing tools
        self.register(Tool {
            name: "nikto".to_string(),
            command: "nikto".to_string(),
            description: "Web server scanner".to_string(),
            category: ToolCategory::WebTesting,
            requires_sudo: false,
            default_timeout: 900, // 15 minutes
            allowed_actions: vec!["scan".to_string(), "test".to_string()],
        });

        self.register(Tool {
            name: "sqlmap".to_string(),
            command: "sqlmap".to_string(),
            description: "SQL injection tool".to_string(),
            category: ToolCategory::WebTesting,
            requires_sudo: false,
            default_timeout: 1800, // 30 minutes
            allowed_actions: vec!["test".to_string(), "exploit".to_string()],
        });

        // Password cracking tools
        self.register(Tool {
            name: "hydra".to_string(),
            command: "hydra".to_string(),
            description: "Network login cracker".to_string(),
            category: ToolCategory::PasswordCracking,
            requires_sudo: false,
            default_timeout: 3600, // 1 hour
            allowed_actions: vec!["crack".to_string()],
        });

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
