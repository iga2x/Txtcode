use super::VirtualMachine;
use crate::runtime::core::{ScopeManager, Value};
use crate::runtime::errors::RuntimeError;
use crate::runtime::permissions::PermissionResource;
use std::path::PathBuf;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

/// Core VirtualMachine functionality (constructors, variables, scopes)
impl VirtualMachine {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            scope_manager: ScopeManager::new(),
            enum_defs: std::collections::HashMap::new(),
            struct_defs: std::collections::HashMap::new(),
            call_stack: crate::runtime::core::CallStack::new(),
            gc: crate::runtime::gc::GarbageCollector::new(),
            module_resolver: crate::runtime::module::ModuleResolver::new(),
            _async_executor: None,
            current_file: None,
            import_stack: Vec::new(),
            exported_symbols: std::collections::HashSet::new(),
            safe_mode: false,
            debug: false,
            verbose: false,
            exec_allowed: true,
            permission_manager: crate::runtime::permissions::PermissionManager::new(),
            audit_trail: crate::runtime::audit::AuditTrail::new(),
            ai_metadata: crate::runtime::audit::AIMetadata::new(),
            policy_engine: crate::policy::PolicyEngine::new(),
            intent_checker: crate::runtime::intent::IntentChecker::new(),
            capability_manager: crate::capability::CapabilityManager::new(),
            active_capability: None,
            runtime_security: crate::runtime::security::RuntimeSecurity::new(),
            cancel_flag: None,
        }
    }

    pub fn with_current_file(mut self, file: PathBuf) -> Self {
        self.current_file = Some(file);
        self
    }

    pub fn with_all_options(safe_mode: bool, debug: bool, verbose: bool) -> Self {
        let mut vm = Self {
            stack: Vec::new(),
            scope_manager: ScopeManager::new(),
            enum_defs: std::collections::HashMap::new(),
            struct_defs: std::collections::HashMap::new(),
            call_stack: crate::runtime::core::CallStack::new(),
            gc: crate::runtime::gc::GarbageCollector::new(),
            module_resolver: crate::runtime::module::ModuleResolver::new(),
            _async_executor: None,
            current_file: None,
            import_stack: Vec::new(),
            exported_symbols: std::collections::HashSet::new(),
            safe_mode,
            debug,
            verbose,
            exec_allowed: !safe_mode,
            permission_manager: crate::runtime::permissions::PermissionManager::new(),
            audit_trail: crate::runtime::audit::AuditTrail::new(),
            ai_metadata: crate::runtime::audit::AIMetadata::new(),
            policy_engine: crate::policy::PolicyEngine::new(),
            intent_checker: crate::runtime::intent::IntentChecker::new(),
            capability_manager: crate::capability::CapabilityManager::new(),
            active_capability: None,
            runtime_security: crate::runtime::security::RuntimeSecurity::new(),
            cancel_flag: None,
        };

        // If safe_mode is enabled, deny exec by default
        if safe_mode {
            vm.deny_permission(PermissionResource::System("exec".to_string()), None);
        }

        vm
    }

    pub(super) fn create_error(&self, message: String) -> RuntimeError {
        RuntimeError::new(message).with_stack_trace(self.call_stack.clone_frames())
    }

    /// Get a variable from current scope or globals
    pub(super) fn get_variable(&self, name: &str) -> Option<Value> {
        self.scope_manager.get_variable(name)
    }

    /// Set a variable in the current scope (or create new scope if needed)
    /// If variable exists in an outer scope, update it there instead
    pub(super) fn set_variable(&mut self, name: String, value: Value) -> Result<(), RuntimeError> {
        self.scope_manager
            .set_variable(name, value)
            .map_err(|e| self.create_error(e))
    }

    /// Public: define a global variable (used by REPL to inject `_` last result)
    pub fn define_global(&mut self, name: String, value: Value) {
        let _ = self.scope_manager.set_variable(name, value);
    }

    /// Set a const variable (immutable)
    pub(super) fn set_const(&mut self, name: String, value: Value) {
        self.scope_manager.set_const(name, value);
    }

    /// Push a new scope
    pub(super) fn push_scope(&mut self) {
        self.scope_manager.push_scope();
    }

    /// Pop the current scope
    pub(super) fn pop_scope(&mut self) {
        self.scope_manager.pop_scope();
    }

    /// Attach a cancellation flag.  When the flag is set to `true` the execution
    /// loop will terminate at the next statement boundary with a timeout error.
    /// Used by `run_file_with_timeout` to stop the worker thread after deadline.
    pub fn set_cancel_flag(&mut self, flag: Arc<AtomicBool>) {
        self.cancel_flag = Some(flag);
    }

    /// Returns true if the external cancel flag has been raised.
    pub(super) fn is_cancelled(&self) -> bool {
        self.cancel_flag
            .as_ref()
            .is_some_and(|f| f.load(Ordering::Relaxed))
    }

    pub fn set_exec_allowed(&mut self, allowed: bool) {
        self.exec_allowed = allowed;

        // Map to permissions for backward compatibility
        if !allowed {
            self.deny_permission(PermissionResource::System("exec".to_string()), None);
        }
    }
}
