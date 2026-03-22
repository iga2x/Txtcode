use super::VirtualMachine;
use std::sync::Arc;
use crate::runtime::core::Value;
use crate::runtime::errors::RuntimeError;
use crate::runtime::intent::{IntentChecker, IntentDeclaration};

/// Intent management methods for VirtualMachine
impl VirtualMachine {
    /// Register intent declaration for a function
    pub fn register_function_intent(&mut self, name: String, declaration: IntentDeclaration) {
        self.intent_checker
            .register_function_intent(name, declaration);
    }

    /// Set module-level intent
    pub fn set_module_intent(&mut self, declaration: IntentDeclaration) {
        self.intent_checker.set_module_intent(declaration);
    }

    /// Check if an action violates declared intent (returns error if violated)
    pub fn check_intent(
        &self,
        function_name: &str,
        action: &str,
        resource: &str,
    ) -> Result<(), RuntimeError> {
        self.intent_checker
            .check_action(function_name, action, resource)
            .map_err(|e| self.create_error(format!("Intent violation: {}", e)))
    }

    /// Get intent checker reference
    pub fn get_intent_checker(&self) -> &IntentChecker {
        &self.intent_checker
    }

    /// Map stdlib function name to action and resource for intent checking
    /// Returns (action, resource_str) if function performs an action, None otherwise
    pub(super) fn map_stdlib_to_action(
        &self,
        function_name: &str,
        args: &[Value],
    ) -> Option<(String, String)> {
        // Map I/O functions
        if function_name == "read_file"
            || function_name == "file_exists"
            || function_name == "is_file"
            || function_name == "is_dir"
            || function_name == "list_dir"
        {
            if let Some(Value::String(path)) = args.first() {
                return Some(("fs.read".to_string(), path.to_string()));
            }
        }
        if function_name == "write_file"
            || function_name == "append_file"
            || function_name == "delete"
            || function_name == "mkdir"
            || function_name == "rmdir"
        {
            if let Some(Value::String(path)) = args.first() {
                return Some(("fs.write".to_string(), path.to_string()));
            }
        }

        // Map network functions
        if function_name == "http_get"
            || function_name == "http_post"
            || function_name == "tcp_connect"
        {
            if let Some(Value::String(url)) = args.first() {
                return Some(("net.connect".to_string(), url.to_string()));
            }
        }

        // Map system functions
        if function_name == "exec" {
            if let Some(Value::String(cmd)) = args.first() {
                return Some(("sys.exec".to_string(), cmd.to_string()));
            }
        }

        // Functions that don't perform actions (no intent check needed)
        // e.g., len, type, max, min, math functions, string functions, etc.
        None
    }
}
