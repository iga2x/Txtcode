// Virtual Machine - modularized into focused components
// Submodules: core, helpers, permissions, intent, capabilities, audit, policy, modules

mod audit;
mod capabilities;
mod core;
mod helpers;
mod intent;
mod modules;
mod permissions;
mod policy;

use crate::capability::CapabilityManager;
use crate::parser::ast::*;
use crate::policy::PolicyEngine;
use crate::runtime::async_executor::AsyncExecutor;
use crate::runtime::audit::{AIMetadata, AuditTrail};
use crate::runtime::core::{CallFrame, CallStack, ScopeManager, Value};
use crate::runtime::errors::RuntimeError;
use crate::runtime::execution::{
    ControlFlowExecutor, ControlFlowVM, StatementExecutor, StatementVM,
};
use crate::runtime::gc::GarbageCollector;
use crate::runtime::intent::IntentChecker;
use crate::runtime::module::ModuleResolver;
use crate::runtime::permissions::{PermissionManager, PermissionResource};
use crate::stdlib::FunctionExecutor;
use crate::tools::logger::{log_debug, log_warn};
use crate::typecheck::types::Type;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Virtual Machine for executing Txt-code programs
#[allow(dead_code)]
pub struct VirtualMachine {
    stack: Vec<Value>,
    scope_manager: ScopeManager,
    enum_defs: HashMap<String, Vec<(String, Option<Expression>)>>,
    struct_defs: HashMap<String, Vec<(String, Type)>>,
    call_stack: CallStack,
    gc: GarbageCollector,
    module_resolver: ModuleResolver,
    async_executor: Option<AsyncExecutor>, // Async executor for async/await
    current_file: Option<PathBuf>,
    import_stack: Vec<PathBuf>, // Track imports to detect circular dependencies
    exported_symbols: HashSet<String>, // Track explicitly exported symbols for current module
    safe_mode: bool,
    debug: bool,
    verbose: bool,
    exec_allowed: bool, // Keep for backward compatibility, but deprecate
    permission_manager: PermissionManager,
    audit_trail: AuditTrail,               // NEW: Audit trail for all actions
    ai_metadata: AIMetadata,               // NEW: AI metadata tracking
    policy_engine: PolicyEngine, // NEW: Policy engine for rate limiting and execution control
    intent_checker: IntentChecker, // NEW: Intent enforcement system
    capability_manager: CapabilityManager, // NEW: Capability token system
    active_capability: Option<String>, // NEW: Active capability token in current scope
}

impl VirtualMachine {
    // Core methods are now in vm/core.rs and vm/helpers.rs
    // Permission methods are now in vm/permissions.rs
    // Intent methods are now in vm/intent.rs
    // Capability methods are now in vm/capabilities.rs
    // Audit methods are now in vm/audit.rs
    // Policy methods are now in vm/policy.rs
    // Module methods are now in vm/modules.rs

    // These methods are implemented in submodules but accessible here
    // Helper methods delegate to implementations in helpers.rs, core.rs, etc.
    // All methods are part of impl VirtualMachine blocks in their respective modules

    // The following methods remain here:
    // - interpret() - main entry point
    // - execute_statement() - statement dispatcher
    // - evaluate_expression() - expression evaluator (large, ~700 lines - can be moved to execution/expressions.rs later)

    // REMOVED: All duplicate methods (bind_pattern_old, extract_free_variables, capture_environment,
    // set_exec_allowed, grant_permission, deny_permission, check_permission, check_permission_with_audit,
    // register_function_intent, set_module_intent, check_intent, get_intent_checker,
    // grant_capability, use_capability, clear_capability, get_active_capability, revoke_capability,
    // get_capability_manager, capability_valid, get_action_from_resource, map_stdlib_to_action,
    // get_permission_manager, get_audit_trail, get_ai_metadata, set_ai_metadata, calculate_provenance_hash,
    // export_audit_trail_json, get_policy_engine, set_policy, check_rate_limit, check_ai_allowed,
    // check_max_execution_time, is_deterministic_mode, get_time, execute_import, execute_export)
    // These are now in their respective module files: vm/core.rs, vm/helpers.rs, vm/permissions.rs,
    // vm/intent.rs, vm/capabilities.rs, vm/audit.rs, vm/policy.rs, vm/modules.rs

    pub fn interpret(&mut self, program: &Program) -> Result<Value, RuntimeError> {
        // Start execution timer for max execution time checking
        self.policy_engine.start_execution();

        // Check AI allowance before execution if AI metadata is present
        if !self.ai_metadata.is_empty() {
            self.check_ai_allowed()?;
        }

        for statement in &program.statements {
            // Check max execution time periodically
            self.check_max_execution_time()?;

            self.execute_statement(statement)?;
            // Periodic garbage collection
            // Note: scopes() returns a slice, but collect expects a Vec reference
            // We need to pass the scopes as a reference to a Vec
            let scopes_vec: Vec<_> = self.scope_manager.scopes().to_vec();
            self.gc
                .collect(&self.stack, self.scope_manager.globals(), &scopes_vec);
        }
        Ok(Value::Null)
    }

    /// Like interpret, but returns the last expression's value (for REPL display).
    pub fn interpret_repl(&mut self, program: &Program) -> Result<Value, RuntimeError> {
        self.policy_engine.start_execution();
        if !self.ai_metadata.is_empty() {
            self.check_ai_allowed()?;
        }
        let mut last = Value::Null;
        for statement in &program.statements {
            self.check_max_execution_time()?;
            let val = self.execute_statement(statement)?;
            if !matches!(val, Value::Null) {
                last = val;
            }
            let scopes_vec: Vec<_> = self.scope_manager.scopes().to_vec();
            self.gc
                .collect(&self.stack, self.scope_manager.globals(), &scopes_vec);
        }
        Ok(last)
    }

    fn execute_statement(&mut self, stmt: &Statement) -> Result<Value, RuntimeError> {
        // Route control flow statements to ControlFlowExecutor
        match stmt {
            Statement::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => ControlFlowExecutor::execute_if(self, condition, then_branch, else_branch),
            Statement::While {
                condition, body, ..
            } => ControlFlowExecutor::execute_while(self, condition, body),
            Statement::DoWhile {
                body, condition, ..
            } => ControlFlowExecutor::execute_do_while(self, body, condition),
            Statement::For {
                variable,
                iterable,
                body,
                ..
            } => ControlFlowExecutor::execute_for(self, variable, iterable, body),
            Statement::Match {
                value,
                cases,
                default,
                ..
            } => ControlFlowExecutor::execute_match(self, value, cases, default),
            Statement::Try {
                body,
                catch,
                finally,
                ..
            } => ControlFlowExecutor::execute_try(self, body, catch, finally),
            Statement::Repeat { count, body, .. } => {
                ControlFlowExecutor::execute_repeat(self, count, body)
            }
            // All other statements go to StatementExecutor
            _ => StatementExecutor::execute(self, stmt),
        }
    }

    fn evaluate_expression(&mut self, expr: &Expression) -> Result<Value, RuntimeError> {
        // Delegate to ExpressionEvaluator
        use crate::runtime::execution::expressions::ExpressionEvaluator;
        ExpressionEvaluator::evaluate(self, expr)
    }
}

impl Default for VirtualMachine {
    fn default() -> Self {
        Self::new()
    }
}

// Implement traits for execution modules
impl StatementVM for VirtualMachine {
    fn evaluate_expression(&mut self, expr: &Expression) -> Result<Value, RuntimeError> {
        // Delegate to ExpressionEvaluator to avoid recursion
        use crate::runtime::execution::expressions::ExpressionEvaluator;
        ExpressionEvaluator::evaluate(self, expr)
    }

    fn get_variable(&self, name: &str) -> Option<Value> {
        // Methods from impl VirtualMachine blocks in submodules are merged
        VirtualMachine::get_variable(self, name)
    }

    fn set_variable(&mut self, name: String, value: Value) -> Result<(), RuntimeError> {
        // Methods from impl VirtualMachine blocks in submodules are merged
        VirtualMachine::set_variable(self, name, value)
    }

    fn set_global(&mut self, name: String, value: Value) -> Result<(), RuntimeError> {
        // Check if it's a const variable
        if self.scope_manager.is_const(&name) {
            return Err(self.create_error(format!("Cannot reassign const variable '{}'", name)));
        }
        self.scope_manager.globals_mut().insert(name, value);
        Ok(())
    }

    fn bind_pattern(&mut self, pattern: &Pattern, value: &Value) -> Result<(), RuntimeError> {
        // Methods from impl VirtualMachine blocks in submodules are merged
        VirtualMachine::bind_pattern(self, pattern, value)
    }

    fn register_enum(&mut self, name: String, variants: Vec<(String, Option<Expression>)>) {
        self.enum_defs.insert(name, variants);
    }

    fn register_struct(&mut self, name: String, fields: Vec<(String, Type)>) {
        self.struct_defs.insert(name, fields);
    }

    fn execute_import(
        &mut self,
        modules: &[String],
        from: &Option<String>,
        alias: &Option<String>,
    ) -> Result<Value, RuntimeError> {
        // Methods from impl VirtualMachine blocks in submodules are merged
        VirtualMachine::execute_import(self, modules, from, alias)
    }

    fn execute_export(&mut self, names: &[String]) -> Result<Value, RuntimeError> {
        // Methods from impl VirtualMachine blocks in submodules are merged
        VirtualMachine::execute_export(self, names)
    }

    fn set_const(&mut self, name: String, value: Value) {
        // Methods from impl VirtualMachine blocks in submodules are merged
        VirtualMachine::set_const(self, name, value)
    }

    fn grant_permission(
        &mut self,
        resource: crate::runtime::permissions::PermissionResource,
        scope: Option<String>,
    ) {
        // Methods from impl VirtualMachine blocks in submodules are merged
        VirtualMachine::grant_permission(self, resource, scope)
    }

    fn register_function_intent(
        &mut self,
        name: String,
        declaration: crate::runtime::intent::IntentDeclaration,
    ) {
        // Methods from impl VirtualMachine blocks in submodules are merged
        VirtualMachine::register_function_intent(self, name, declaration)
    }
}

impl ControlFlowVM for VirtualMachine {
    fn evaluate_expression(&mut self, expr: &Expression) -> Result<Value, RuntimeError> {
        // Methods from impl VirtualMachine blocks in submodules are merged
        VirtualMachine::evaluate_expression(self, expr)
    }

    fn execute_statement(&mut self, stmt: &Statement) -> Result<Value, RuntimeError> {
        // Methods from impl VirtualMachine blocks in submodules are merged
        VirtualMachine::execute_statement(self, stmt)
    }

    fn push_scope(&mut self) {
        // Methods from impl VirtualMachine blocks in submodules are merged
        VirtualMachine::push_scope(self);
    }

    fn pop_scope(&mut self) {
        // Methods from impl VirtualMachine blocks in submodules are merged
        VirtualMachine::pop_scope(self);
    }

    fn set_variable(&mut self, name: String, value: Value) -> Result<(), RuntimeError> {
        // Methods from impl VirtualMachine blocks in submodules are merged
        VirtualMachine::set_variable(self, name, value)
    }

    fn bind_pattern(&mut self, pattern: &Pattern, value: &Value) -> Result<(), RuntimeError> {
        // Methods from impl VirtualMachine blocks in submodules are merged
        VirtualMachine::bind_pattern(self, pattern, value)
    }
}

impl crate::stdlib::capabilities::CapabilityExecutor for VirtualMachine {
    fn grant_capability(
        &mut self,
        resource: PermissionResource,
        action: String,
        scope: Option<String>,
        expires_in: Option<std::time::Duration>,
        granted_by: Option<String>,
        ai_metadata: Option<AIMetadata>,
    ) -> String {
        VirtualMachine::grant_capability(
            self,
            resource,
            action,
            scope,
            expires_in,
            granted_by,
            ai_metadata,
        )
    }

    fn use_capability(&mut self, token_id: String) -> Result<(), RuntimeError> {
        VirtualMachine::use_capability(self, token_id)
    }

    fn revoke_capability(
        &mut self,
        token_id: &str,
        reason: Option<String>,
    ) -> Result<(), RuntimeError> {
        VirtualMachine::revoke_capability(self, token_id, reason)
    }

    fn capability_valid(&self, token_id: &str) -> bool {
        VirtualMachine::capability_valid(self, token_id)
    }
}

impl crate::stdlib::permission_checker::PermissionChecker for VirtualMachine {
    fn check_permission(
        &self,
        resource: &PermissionResource,
        scope: Option<&str>,
    ) -> Result<(), RuntimeError> {
        VirtualMachine::check_permission(self, resource, scope)
    }
}

impl FunctionExecutor for VirtualMachine {
    fn call_function_value(&mut self, func: &Value, args: &[Value]) -> Result<Value, RuntimeError> {
        match func {
            Value::Function(name, params, body, captured_env) => {
                let params = params.clone();
                let body = body.clone();
                let captured_env = captured_env.clone();

                log_debug(&format!(
                    "Calling function value '{}' with {} arguments",
                    name,
                    args.len()
                ));

                // Guard against unbounded recursion.
                // Kept at 50 in debug mode — larger enums use more Rust stack per frame.
                const MAX_CALL_DEPTH: usize = 50;
                if self.call_stack.frames().len() >= MAX_CALL_DEPTH {
                    return Err(RuntimeError::new(format!(
                        "Maximum call stack depth ({}) exceeded — possible infinite recursion in '{}'",
                        MAX_CALL_DEPTH, name
                    )));
                }

                // Push call frame
                self.call_stack.push(CallFrame {
                    function_name: name.clone(),
                    line: 0,
                    column: 0,
                });

                // Push captured environment as a scope (for closures) - BEFORE parameters
                if !captured_env.is_empty() {
                    self.push_scope();
                    for (var_name, var_value) in &captured_env {
                        self.set_variable(var_name.clone(), var_value.clone())?;
                    }
                }

                // Push new scope for function parameters
                self.push_scope();

                // Use a closure to ensure cleanup on early return
                let result = (|| -> Result<Value, RuntimeError> {
                    // Bind arguments with variadic support
                    let mut arg_index = 0;
                    let args_len = args.len();

                    for param in &params {
                        if param.is_variadic {
                            // Collect all remaining arguments into an array for variadic parameter
                            let remaining_args: Vec<Value> = args[arg_index..].to_vec();
                            self.set_variable(param.name.clone(), Value::Array(remaining_args))?;
                            arg_index = args_len; // Mark all args as consumed
                        } else {
                            // Regular parameter binding
                            if arg_index < args_len {
                                let arg = &args[arg_index];
                                self.set_variable(param.name.clone(), arg.clone())?;
                                arg_index += 1;
                            } else if let Some(default_expr) = &param.default_value {
                                // Evaluate default value expression
                                let default_val = self.evaluate_expression(default_expr)?;
                                self.set_variable(param.name.clone(), default_val)?;
                                arg_index += 1;
                            } else {
                                return Err(self.create_error(format!(
                                    "Missing required parameter: {}",
                                    param.name
                                )));
                            }
                        }
                    }

                    // Check if there are extra arguments after all parameters are bound
                    if arg_index < args_len {
                        log_warn(&format!(
                            "Extra arguments provided to function '{}' ({} unused)",
                            name,
                            args_len - arg_index
                        ));
                    }

                    let mut result = Value::Null;
                    for stmt in &body {
                        if let Statement::Return { value, .. } = stmt {
                            result = if let Some(expr) = value {
                                self.evaluate_expression(expr)?
                            } else {
                                Value::Null
                            };
                            break;
                        }
                        self.execute_statement(stmt)?;
                    }

                    Ok(result)
                })();

                // Always clean up scope and call frame, even on error
                self.pop_scope();
                if !captured_env.is_empty() {
                    self.pop_scope(); // Pop captured environment scope too
                }
                self.call_stack.pop();

                result
            }
            _ => Err(RuntimeError::new("Expected function value".to_string())),
        }
    }
}

// Implement ExpressionVM trait for VirtualMachine
impl crate::runtime::execution::expressions::ExpressionVM for VirtualMachine {
    fn get_variable(&self, name: &str) -> Option<Value> {
        VirtualMachine::get_variable(self, name)
    }

    fn set_variable(&mut self, name: String, value: Value) -> Result<(), RuntimeError> {
        VirtualMachine::set_variable(self, name, value)
    }

    fn push_scope(&mut self) {
        VirtualMachine::push_scope(self)
    }

    fn pop_scope(&mut self) {
        VirtualMachine::pop_scope(self)
    }

    fn create_error(&self, message: String) -> RuntimeError {
        VirtualMachine::create_error(self, message)
    }

    fn check_permission_with_audit(
        &mut self,
        resource: &crate::runtime::permissions::PermissionResource,
        scope: Option<&str>,
    ) -> Result<(), RuntimeError> {
        VirtualMachine::check_permission_with_audit(self, resource, scope)
    }

    fn check_rate_limit(&mut self, action: &str) -> Result<(), RuntimeError> {
        VirtualMachine::check_rate_limit(self, action)
    }

    fn map_stdlib_to_action(&self, name: &str, args: &[Value]) -> Option<(String, String)> {
        VirtualMachine::map_stdlib_to_action(self, name, args)
    }

    fn check_intent(
        &self,
        function_name: &str,
        action: &str,
        resource: &str,
    ) -> Result<(), RuntimeError> {
        VirtualMachine::check_intent(self, function_name, action, resource)
    }

    fn call_stack_current_frame(&self) -> Option<&crate::runtime::core::CallFrame> {
        self.call_stack.current_frame()
    }

    fn call_stack_depth(&self) -> usize {
        self.call_stack.frames().len()
    }

    fn call_stack_push(&mut self, frame: crate::runtime::core::CallFrame) {
        self.call_stack.push(frame)
    }

    fn call_stack_pop(&mut self) {
        let _ = self.call_stack.pop();
    }

    fn audit_trail_log_action(
        &mut self,
        action: String,
        resource: String,
        context: Option<String>,
        result: crate::runtime::audit::AuditResult,
        ai_metadata: Option<&crate::runtime::audit::AIMetadata>,
    ) {
        let _ = self.audit_trail.log_action(
            action,
            resource,
            context.as_deref().map(|s| s.to_string()),
            result,
            ai_metadata,
        );
    }

    fn ai_metadata(&self) -> &crate::runtime::audit::AIMetadata {
        &self.ai_metadata
    }

    fn struct_defs(&self) -> &HashMap<String, Vec<(String, Type)>> {
        &self.struct_defs
    }

    fn enum_defs(&self) -> &HashMap<String, Vec<(String, Option<Expression>)>> {
        &self.enum_defs
    }

    fn gc_register_allocation(&mut self, value: &Value) {
        self.gc.register_allocation(value)
    }

    fn debug(&self) -> bool {
        self.debug
    }

    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec_allowed(&self) -> Option<bool> {
        Some(self.exec_allowed)
    }

    fn execute_statement(&mut self, stmt: &Statement) -> Result<Value, RuntimeError> {
        VirtualMachine::execute_statement(self, stmt)
    }

    fn extract_free_variables(body: &Expression, param_names: &HashSet<String>) -> HashSet<String> {
        // Call the static method from helpers module
        // Since it's in an impl block, we can call it directly via Self
        Self::extract_free_variables(body, param_names)
    }

    fn capture_environment(&self, var_names: &HashSet<String>) -> HashMap<String, Value> {
        // Call the instance method from helpers module
        // Since it's in an impl block, we can call it directly
        self.capture_environment(var_names)
    }

    fn handle_capability_function(
        &mut self,
        name: &str,
        args: &[Value],
    ) -> Result<Option<Value>, RuntimeError> {
        use crate::stdlib::CapabilityLib;
        match CapabilityLib::call_function(name, args, Some(self)) {
            Ok(result) => Ok(Some(result)),
            Err(e) => Err(e),
        }
    }

    fn call_stdlib_function(&mut self, name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
        use crate::stdlib::StdLib;
        // Extract permission checker from self first (immutable borrow)
        // Then use it in the call with mutable borrow of self as executor
        // We need to split the borrow: check permissions first, then call
        // For now, use call_function - it will route correctly, permissions are checked inside NetLib/IOLib
        // The stdlib functions receive executor but check permissions themselves if needed
        StdLib::call_function(name, args, self.exec_allowed, Some(self))
    }
}
