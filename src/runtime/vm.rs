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
use crate::runtime::audit::AuditTrail;
use crate::runtime::core::{CallFrame, CallStack, ScopeManager, Value};
use crate::runtime::errors::RuntimeError;
use crate::runtime::execution::{
    ControlFlowExecutor, ControlFlowVM, StatementExecutor, StatementVM,
};
use crate::runtime::gc::MemoryTracker;
use crate::runtime::intent::IntentChecker;
use crate::runtime::module::ModuleResolver;
use crate::runtime::permissions::{PermissionManager, PermissionResource};
use crate::runtime::security::RuntimeSecurity;
use crate::stdlib::FunctionExecutor;
use crate::tools::logger::{log_debug, log_warn};
use crate::typecheck::types::Type;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, atomic::AtomicBool};

/// Virtual Machine for executing Txt-code programs
#[allow(dead_code)]
pub struct VirtualMachine {
    stack: Vec<Value>,
    scope_manager: ScopeManager,
    enum_defs: HashMap<String, Vec<(String, Option<Expression>)>>,
    struct_defs: HashMap<String, Vec<(String, Type)>>,
    /// Struct method registry: struct_name → method_name → Function value
    struct_methods: HashMap<String, HashMap<String, Value>>,
    call_stack: CallStack,
    memory: MemoryTracker,
    module_resolver: ModuleResolver,
    current_file: Option<PathBuf>,
    import_stack: Vec<PathBuf>, // Track imports to detect circular dependencies
    exported_symbols: HashSet<String>, // Track explicitly exported symbols for current module
    safe_mode: bool,
    debug: bool,
    verbose: bool,
    strict_types: bool,
    exec_allowed: bool, // Keep for backward compatibility, but deprecate
    permission_manager: PermissionManager,
    audit_trail: AuditTrail,  // Audit trail for all actions
    // B.1: ai_metadata removed — was always empty; AIMetadata struct kept in audit.rs for future use
    policy_engine: PolicyEngine, // Policy engine for rate limiting and execution control
    intent_checker: IntentChecker, // NEW: Intent enforcement system
    capability_manager: CapabilityManager, // NEW: Capability token system
    active_capability: Option<String>, // NEW: Active capability token in current scope
    pub runtime_security: RuntimeSecurity, // Capability-adaptive security (anti-debug + integrity)
    /// Cancellation flag: when set to `true` by an external caller (e.g. timeout handler),
    /// the VM terminates its execution loop at the next statement boundary.
    cancel_flag: Option<Arc<AtomicBool>>,
    /// Names of functions declared with the `async` keyword.
    /// When one of these is called (without `await`), a thread is spawned and
    /// a `Value::Future` is returned to the caller instead of blocking.
    async_functions: HashSet<String>,
    /// Coverage: set of source line numbers executed when coverage_enabled = true.
    pub covered_lines: HashSet<u32>,
    /// Whether line coverage tracking is active.
    coverage_enabled: bool,
    /// O.3: Current statement's source location (line, column) for error reporting.
    /// Updated at the start of each `execute_statement` call.
    pub(crate) current_span: Option<(usize, usize)>,
    /// Per-engine native function registry (populated by TxtcodeEngine::register_fn).
    /// Stored on the VM so multiple engines don't share a global registry.
    pub(crate) native_registry: HashMap<String, Box<dyn Fn(&[Value]) -> Value + Send + Sync>>,
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
        // ── Security startup checks ────────────────────────────────────────
        // Run anti-debug + integrity at the highest level available on this platform.
        // Results are logged to the audit trail; execution is NOT blocked.
        // Warnings surface in the audit trail for operator review.
        {
            let report = self.runtime_security.run_startup_checks();
            let summary = report.summary();
            for w in &report.warnings {
                log_warn(&format!("Security warning: {}", w));
            }
            let _ = self.audit_trail.log_action(
                "security.startup".to_string(),
                summary,
                Some(format!(
                    "level={} platform={} secure={}",
                    report.level,
                    report.platform,
                    report.is_secure()
                )),
                if report.is_secure() {
                    crate::runtime::audit::AuditResult::Allowed
                } else {
                    crate::runtime::audit::AuditResult::Error(
                        report.warnings.first().cloned().unwrap_or_default(),
                    )
                },
                None,
            );
            // Hard enforcement: block on debugger presence or integrity mismatch.
            RuntimeSecurity::enforce_security_report(&report)
                .map_err(RuntimeError::new)?;
        }

        // Start execution timer for max execution time checking
        self.policy_engine.start_execution();

        for statement in &program.statements {
            // Check max execution time and external cancellation flag periodically.
            self.check_max_execution_time()?;
            if self.is_cancelled() {
                return Err(RuntimeError::new("Execution cancelled: timeout exceeded".to_string()));
            }

            // Execute the statement; attach its source location to any error
            // that doesn't already carry one (innermost span wins).
            self.execute_statement(statement).map_err(|e| {
                if let Some((line, col)) = statement.source_location() {
                    e.with_span(line, col)
                } else {
                    e
                }
            })?;
            // Periodic garbage collection
            // Note: scopes() returns a slice, but collect expects a Vec reference
            // We need to pass the scopes as a reference to a Vec
            let scopes_vec: Vec<_> = self.scope_manager.scopes().to_vec();
            self.memory
                .collect_checked(&self.stack, self.scope_manager.globals(), &scopes_vec)
                .map_err(|e| RuntimeError::new(e).with_code(crate::runtime::errors::ErrorCode::E0021))?;
        }
        Ok(Value::Null)
    }

    /// Like interpret, but returns the last expression's value (for REPL display).
    pub fn interpret_repl(&mut self, program: &Program) -> Result<Value, RuntimeError> {
        // ── Security startup checks (per REPL input) ──────────────────────────
        // Run anti-debug + integrity at the best available level. Note: no source
        // hash is set for REPL input, so SecurityLevel stays at Standard (Full
        // requires a hash). Integrity field will be None, not a failure.
        {
            let report = self.runtime_security.run_startup_checks();
            for w in &report.warnings {
                log_warn(&format!("Security warning: {}", w));
            }
            let _ = self.audit_trail.log_action(
                "security.startup".to_string(),
                report.summary(),
                Some(format!(
                    "level={} platform={} secure={}",
                    report.level,
                    report.platform,
                    report.is_secure()
                )),
                if report.is_secure() {
                    crate::runtime::audit::AuditResult::Allowed
                } else {
                    crate::runtime::audit::AuditResult::Error(
                        report.warnings.first().cloned().unwrap_or_default(),
                    )
                },
                None,
            );
            RuntimeSecurity::enforce_security_report(&report)
                .map_err(RuntimeError::new)?;
        }
        self.policy_engine.start_execution();
        let mut last = Value::Null;
        for statement in &program.statements {
            self.check_max_execution_time()?;
            if self.is_cancelled() {
                return Err(RuntimeError::new("Execution cancelled: timeout exceeded".to_string()));
            }
            let val = self.execute_statement(statement).map_err(|e| {
                if let Some((line, col)) = statement.source_location() {
                    e.with_span(line, col)
                } else {
                    e
                }
            })?;
            if !matches!(val, Value::Null) {
                last = val;
            }
            let scopes_vec: Vec<_> = self.scope_manager.scopes().to_vec();
            self.memory
                .collect_checked(&self.stack, self.scope_manager.globals(), &scopes_vec)
                .map_err(|e| RuntimeError::new(e).with_code(crate::runtime::errors::ErrorCode::E0021))?;
        }
        Ok(last)
    }

    fn execute_statement(&mut self, stmt: &Statement) -> Result<Value, RuntimeError> {
        // O.3: Track current statement span for runtime error location reporting.
        // Coverage tracking: record the source line before executing each statement.
        if let Some(loc) = stmt.source_location() {
            self.current_span = Some(loc);
            self.record_line(loc.0);
        }
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
            // Task 15.1: Structured concurrency nursery block
            Statement::Nursery { body, .. } => {
                use crate::runtime::execution::statements::NURSERY_HANDLES;
                // Activate the nursery handle collector
                NURSERY_HANDLES.with(|h| *h.borrow_mut() = Some(Vec::new()));
                self.push_scope();
                let body_result = (|| -> Result<Value, RuntimeError> {
                    for stmt in body {
                        self.execute_statement(stmt)?;
                    }
                    Ok(Value::Null)
                })();
                self.pop_scope();
                // Collect all spawned task handles
                let handles = NURSERY_HANDLES.with(|h| h.borrow_mut().take().unwrap_or_default());
                // Await all child tasks; track first child error
                let mut first_child_err: Option<RuntimeError> = None;
                for handle in handles {
                    match handle.resolve() {
                        Ok(_) => {}
                        Err(e) => {
                            if first_child_err.is_none() {
                                first_child_err = Some(RuntimeError::new(e));
                            }
                        }
                    }
                }
                // Body error takes priority (e.g. break/return signals), then child errors
                body_result?;
                if let Some(e) = first_child_err {
                    return Err(e);
                }
                Ok(Value::Null)
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

    fn register_struct_method(&mut self, struct_name: &str, method_name: String, func: Value) {
        self.struct_methods
            .entry(struct_name.to_string())
            .or_default()
            .insert(method_name, func);
    }

    fn execute_nested_statement(&mut self, stmt: &Statement) -> Result<Value, RuntimeError> {
        self.execute_statement(stmt)
    }

    fn is_in_local_scope(&self) -> bool {
        VirtualMachine::is_in_local_scope(self)
    }

    fn snapshot_local_vars(&self) -> std::collections::HashMap<String, Value> {
        VirtualMachine::snapshot_local_vars(self)
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

    fn struct_defs(&self) -> &HashMap<String, Vec<(String, Type)>> {
        &self.struct_defs
    }

    fn strict_types(&self) -> bool {
        self.strict_types
    }

    fn register_async_function(&mut self, name: &str) {
        self.async_functions.insert(name.to_string());
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

    fn call_struct_method(&mut self, obj: Value, method: &str) -> Option<Result<Value, RuntimeError>> {
        use crate::runtime::execution::expressions::ExpressionVM;
        if let Value::Struct(ref type_name, _) = obj {
            if let Some(func_val) = ExpressionVM::lookup_struct_method(self, type_name, method) {
                use crate::runtime::execution::expressions::call_user_function;
                if let Value::Function(_, params, body, captured_env) = func_val {
                    let self_arg = vec![obj.clone()];
                    let dummy = crate::parser::ast::Expression::Identifier(method.to_string());
                    return Some(call_user_function(self, method, &params, &body, &captured_env, &self_arg, &dummy));
                }
            }
        }
        None
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
    ) -> String {
        VirtualMachine::grant_capability(
            self,
            resource,
            action,
            scope,
            expires_in,
            granted_by,
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
                const MAX_CALL_DEPTH: usize = crate::runtime::errors::MAX_CALL_DEPTH;
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

                // Push captured environment as a scope (for closures) - BEFORE parameters.
                // Use define_local so values land in the new scope rather than updating
                // any outer-scope variable with the same name (mutation isolation).
                if !captured_env.is_empty() {
                    self.push_scope();
                    for (var_name, var_value) in &captured_env {
                        self.scope_manager.define_local(var_name.clone(), var_value.clone());
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

    fn deterministic_time(&self) -> Option<std::time::SystemTime> {
        if self.policy_engine.is_deterministic_mode() {
            Some(self.policy_engine.get_time())
        } else {
            None
        }
    }

    fn deterministic_random_seed(&self) -> Option<u64> {
        self.policy_engine.get_random_seed()
    }
}

// Implement ExpressionVM trait for VirtualMachine
impl crate::runtime::execution::expressions::ExpressionVM for VirtualMachine {
    /// Check the per-VM native function registry.
    fn call_native_fn(&mut self, fn_name: &str, args: &[Value]) -> Option<Value> {
        if let Some(f) = self.native_registry.get(fn_name) {
            let result = f(args);
            return Some(result);
        }
        None
    }

    fn get_variable(&self, name: &str) -> Option<Value> {
        VirtualMachine::get_variable(self, name)
    }

    fn list_variables(&self) -> Vec<String> {
        let mut names: std::collections::HashSet<String> = std::collections::HashSet::new();
        for (k, _) in self.scope_manager.globals() {
            names.insert(k.clone());
        }
        for scope in self.scope_manager.scopes() {
            for k in scope.keys() {
                names.insert(k.clone());
            }
        }
        names.into_iter().collect()
    }

    fn set_variable(&mut self, name: String, value: Value) -> Result<(), RuntimeError> {
        VirtualMachine::set_variable(self, name, value)
    }

    fn define_local_variable(&mut self, name: String, value: Value) -> Result<(), RuntimeError> {
        self.scope_manager.define_local(name, value);
        Ok(())
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
    ) {
        let _ = self.audit_trail.log_action(
            action,
            resource,
            context.as_deref().map(|s| s.to_string()),
            result,
            None, // B.1: ai_metadata removed
        );
    }

    fn struct_defs(&self) -> &HashMap<String, Vec<(String, Type)>> {
        &self.struct_defs
    }

    fn enum_defs(&self) -> &HashMap<String, Vec<(String, Option<Expression>)>> {
        &self.enum_defs
    }

    fn lookup_struct_method(&self, struct_name: &str, method_name: &str) -> Option<Value> {
        self.struct_methods
            .get(struct_name)
            .and_then(|methods| methods.get(method_name))
            .cloned()
    }

    fn gc_register_allocation(&mut self, value: &Value) {
        self.memory.register_allocation(value)
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

    fn strict_types(&self) -> bool {
        self.strict_types
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
        // Tool functions need audit_trail and policy_engine from VM state, which cannot
        // be passed through the generic StdLib::call_function path without borrow conflicts.
        // Intercept them here and route to a dedicated helper on impl VirtualMachine.
        if name == "tool_exec" || name == "tool_list" || name == "tool_info" {
            return self.dispatch_tool_function(name, args);
        }
        StdLib::call_function(name, args, self.exec_allowed, Some(self))
    }

    fn is_async_function(&self, name: &str) -> bool {
        self.async_functions.contains(name)
    }

    fn globals_snapshot(&self) -> HashMap<String, Value> {
        self.scope_manager.globals().clone()
    }

    fn exec_allowed_bool(&self) -> bool {
        self.exec_allowed
    }

    /// Task 15.3: Run `func` in a new thread, wait up to `ms` milliseconds.
    fn with_timeout_function(&mut self, ms: u64, func: Value) -> Result<Value, RuntimeError> {
        let (name, params, body, captured_env) = match func {
            Value::Function(n, p, b, e) => (n, p, b, e),
            _ => return Err(RuntimeError::new(
                "with_timeout: second argument must be a function".to_string(),
            )),
        };

        let globals = self.scope_manager.globals().clone();
        let exec_allowed = self.exec_allowed;

        let (handle, sender) = crate::runtime::core::value::FutureHandle::pending();

        std::thread::spawn(move || {
            use crate::runtime::execution::expressions::call_user_function;
            use crate::parser::ast::{Expression, Literal};

            let mut child_vm = VirtualMachine::new();
            child_vm.set_exec_allowed(exec_allowed);
            for (k, v) in globals {
                child_vm.define_global(k, v);
            }

            let dummy_expr = Expression::Literal(Literal::Null);
            let result = call_user_function(
                &mut child_vm,
                &name,
                &params,
                &body,
                &captured_env,
                &[],
                &dummy_expr,
            )
            .map_err(|e: RuntimeError| e.to_string());

            sender.send(result);
        });

        let timeout = std::time::Duration::from_millis(ms);
        match handle.resolve_with_timeout(timeout) {
            None => Ok(Value::Result(
                false,
                Box::new(Value::String(Arc::from("timeout".to_string()))),
            )),
            Some(Ok(v)) => Ok(Value::Result(true, Box::new(v))),
            Some(Err(e)) => Ok(Value::Result(false, Box::new(Value::String(Arc::from(e))))),
        }
    }

    /// Task 20.2: Free-standing async_run(closure) — spawns an OS thread and returns
    /// a `Value::Future` that can be passed to `await_all([...])`.
    /// Does NOT require a nursery block.
    fn async_run(&mut self, func: Value) -> Result<Value, RuntimeError> {
        let (name, params, body, captured_env) = match func {
            Value::Function(n, p, b, e) => (n, p, b, e),
            _ => return Err(RuntimeError::new(
                "async_run expects a function (zero-argument closure) as argument".to_string(),
            )),
        };

        let globals = self.scope_manager.globals().clone();
        let exec_allowed = self.exec_allowed;
        // D.2: Snapshot permission state at submission time so post-submission
        // deny() calls on the parent VM do not affect already-queued tasks.
        let permission_snapshot = self.snapshot_permissions();

        let (handle, sender) = crate::runtime::core::value::FutureHandle::pending();

        let task = move || {
            use crate::runtime::execution::expressions::call_user_function;
            use crate::parser::ast::{Expression, Literal};

            let mut child_vm = VirtualMachine::new();
            child_vm.set_exec_allowed(exec_allowed);
            // Restore the permission snapshot captured at task submission time.
            child_vm.set_permission_manager(permission_snapshot);
            for (k, v) in globals {
                child_vm.define_global(k, v);
            }
            let dummy_expr = Expression::Literal(Literal::Null);
            let result = call_user_function(
                &mut child_vm,
                &name,
                &params,
                &body,
                &captured_env,
                &[],
                &dummy_expr,
            )
            .map_err(|e: RuntimeError| e.to_string());
            sender.send(result);
        };

        // Task 26.1: use the event loop worker thread when enabled, otherwise fall back
        // to spawning a dedicated OS thread (original behavior).
        if crate::runtime::event_loop::is_enabled() {
            if !crate::runtime::event_loop::submit(Box::new(task)) {
                let cap = crate::runtime::event_loop::max_concurrent_tasks();
                return Err(RuntimeError::new(format!(
                    "async task queue full (max {} concurrent tasks)", cap
                ))
                .with_code(crate::runtime::errors::ErrorCode::E0053));
            }
        } else {
            std::thread::spawn(task);
        }

        Ok(Value::Future(handle))
    }

    /// D.2: async_run_scoped — like async_run but with a restricted permission set.
    ///
    /// `allowed` is a `Value::Array` of permission name strings (e.g. `["fs.read", "net.connect"]`).
    /// The child VM only gets the intersection of the parent's grants and this list.
    fn async_run_scoped(&mut self, func: Value, allowed: Value) -> Result<Value, RuntimeError> {
        let (name, params, body, captured_env) = match func {
            Value::Function(n, p, b, e) => (n, p, b, e),
            _ => return Err(RuntimeError::new(
                "async_run_scoped expects a function as first argument".to_string(),
            )),
        };

        let allowed_names: Vec<String> = match allowed {
            Value::Array(items) => items.into_iter().filter_map(|v| {
                if let Value::String(s) = v { Some(s.to_string()) } else { None }
            }).collect(),
            _ => vec![],
        };

        let globals = self.scope_manager.globals().clone();
        let exec_allowed = self.exec_allowed;
        let permission_snapshot = self.snapshot_permissions();

        let (handle, sender) = crate::runtime::core::value::FutureHandle::pending();

        let task = move || {
            use crate::runtime::execution::expressions::call_user_function;
            use crate::parser::ast::{Expression, Literal};
            use crate::runtime::permissions::{Permission, PermissionResource};

            let mut child_vm = VirtualMachine::new();
            child_vm.set_exec_allowed(exec_allowed);

            // Build a scoped permission manager from the snapshot filtered by `allowed_names`.
            let mut scoped_pm = crate::runtime::permissions::PermissionManager::new();
            for perm_name in &allowed_names {
                // Map string names to PermissionResource for grant check.
                let resource = match perm_name.as_str() {
                    "fs.read" | "fs.write" | "fs" =>
                        PermissionResource::FileSystem("*".to_string()),
                    "net.connect" | "net" =>
                        PermissionResource::Network("*".to_string()),
                    "process.exec" | "exec" =>
                        PermissionResource::Process(vec![]),
                    s if s.starts_with("sys.") =>
                        PermissionResource::System(s["sys.".len()..].to_string()),
                    s =>
                        PermissionResource::System(s.to_string()),
                };
                // Only grant if parent had it
                if permission_snapshot.check(&resource, None).is_ok() {
                    scoped_pm.grant(Permission::new(resource, None));
                }
            }
            child_vm.set_permission_manager(scoped_pm);

            for (k, v) in globals {
                child_vm.define_global(k, v);
            }
            let dummy_expr = Expression::Literal(Literal::Null);
            let result = call_user_function(
                &mut child_vm,
                &name,
                &params,
                &body,
                &captured_env,
                &[],
                &dummy_expr,
            )
            .map_err(|e: RuntimeError| e.to_string());
            sender.send(result);
        };

        if crate::runtime::event_loop::is_enabled() {
            if !crate::runtime::event_loop::submit(Box::new(task)) {
                let cap = crate::runtime::event_loop::max_concurrent_tasks();
                return Err(RuntimeError::new(format!(
                    "async task queue full (max {} concurrent tasks)", cap
                ))
                .with_code(crate::runtime::errors::ErrorCode::E0053));
            }
        } else {
            std::thread::spawn(task);
        }

        Ok(Value::Future(handle))
    }

    /// O.4: async_run_timeout(closure, timeout_ms) — like async_run but cancels
    /// the task after `timeout_ms` milliseconds if it hasn't completed.
    /// Cancelled tasks return E0020 (timeout) via the future.
    fn async_run_timeout(&mut self, func: Value, timeout_ms: i64) -> Result<Value, RuntimeError> {
        let (name, params, body, captured_env) = match func {
            Value::Function(n, p, b, e) => (n, p, b, e),
            _ => return Err(RuntimeError::new(
                "async_run_timeout expects a function (closure) as first argument".to_string(),
            )),
        };

        if timeout_ms <= 0 {
            return Err(RuntimeError::new(
                "async_run_timeout: timeout_ms must be a positive integer".to_string(),
            ));
        }

        let globals = self.scope_manager.globals().clone();
        let exec_allowed = self.exec_allowed;
        let permission_snapshot = self.snapshot_permissions();

        let (handle, sender) = crate::runtime::core::value::FutureHandle::pending();

        // Spawn a timeout thread that cancels the task after `timeout_ms`.
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let cancel_clone = std::sync::Arc::clone(&cancel);
        let duration_ms = timeout_ms as u64;
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(duration_ms));
            cancel_clone.store(true, std::sync::atomic::Ordering::Relaxed);
        });

        let task = move || {
            use crate::runtime::execution::expressions::call_user_function;
            use crate::parser::ast::{Expression, Literal};

            let mut child_vm = VirtualMachine::new();
            child_vm.set_exec_allowed(exec_allowed);
            child_vm.set_permission_manager(permission_snapshot);
            child_vm.set_cancel_flag(cancel);
            for (k, v) in globals {
                child_vm.define_global(k, v);
            }
            let dummy_expr = Expression::Literal(Literal::Null);
            let result = call_user_function(
                &mut child_vm,
                &name,
                &params,
                &body,
                &captured_env,
                &[],
                &dummy_expr,
            )
            .map_err(|e: RuntimeError| e.to_string());
            sender.send(result);
        };

        if crate::runtime::event_loop::is_enabled() {
            if !crate::runtime::event_loop::submit(Box::new(task)) {
                let cap = crate::runtime::event_loop::max_concurrent_tasks();
                return Err(RuntimeError::new(format!(
                    "async task queue full (max {} concurrent tasks)", cap
                ))
                .with_code(crate::runtime::errors::ErrorCode::E0053));
            }
        } else {
            std::thread::spawn(task);
        }

        Ok(Value::Future(handle))
    }

    /// Task 15.1: Spawn a function as an async nursery task.
    /// Pushes the resulting FutureHandle into the active NURSERY_HANDLES collector.
    fn spawn_for_nursery(&mut self, func: Value) -> Result<(), RuntimeError> {
        use crate::runtime::execution::statements::NURSERY_HANDLES;
        // Verify we are inside a nursery block
        if NURSERY_HANDLES.with(|h| h.borrow().is_none()) {
            return Err(RuntimeError::new(
                "nursery_spawn called outside an `async → nursery` block".to_string(),
            ));
        }
        let (name, params, body, captured_env) = match func {
            Value::Function(n, p, b, e) => (n, p, b, e),
            _ => return Err(RuntimeError::new(
                "nursery_spawn expects a function argument".to_string(),
            )),
        };

        let globals = self.scope_manager.globals().clone();
        let exec_allowed = self.exec_allowed;

        let (handle, sender) = crate::runtime::core::value::FutureHandle::pending();

        std::thread::spawn(move || {
            use crate::runtime::execution::expressions::call_user_function;
            use crate::parser::ast::{Expression, Literal};

            let mut child_vm = VirtualMachine::new();
            child_vm.set_exec_allowed(exec_allowed);
            for (k, v) in globals {
                child_vm.define_global(k, v);
            }

            let dummy_expr = Expression::Literal(Literal::Null);
            let result = call_user_function(
                &mut child_vm,
                &name,
                &params,
                &body,
                &captured_env,
                &[],
                &dummy_expr,
            )
            .map_err(|e: RuntimeError| e.to_string());

            sender.send(result);
        });

        NURSERY_HANDLES.with(|h| {
            if let Some(ref mut handles) = *h.borrow_mut() {
                handles.push(handle);
            }
        });

        Ok(())
    }

    /// Spawn an async user function in a new OS thread and return a `Value::Future`.
    ///
    /// The spawned thread gets its own fresh `VirtualMachine` pre-loaded with a
    /// snapshot of the current global scope (so it can see other functions and
    /// global variables defined before the call).
    fn maybe_spawn_async(
        &mut self,
        name: &str,
        params: Vec<crate::parser::ast::Parameter>,
        body: Vec<crate::parser::ast::Statement>,
        captured_env: HashMap<String, Value>,
        args: Vec<Value>,
    ) -> Option<Result<Value, RuntimeError>> {
        if !self.async_functions.contains(name) {
            return None;
        }

        let globals = self.scope_manager.globals().clone();
        let exec_allowed = self.exec_allowed;
        let name_owned = name.to_string();

        let (handle, sender) = crate::runtime::core::value::FutureHandle::pending();

        std::thread::spawn(move || {
            use crate::runtime::execution::expressions::call_user_function;
            use crate::parser::ast::{Expression, Literal};

            let mut child_vm = VirtualMachine::new();
            child_vm.set_exec_allowed(exec_allowed);
            // Populate the child VM with all globals (user functions, constants, etc.)
            for (k, v) in globals {
                child_vm.define_global(k, v);
            }

            let dummy_expr = Expression::Literal(Literal::Null);
            let result = call_user_function(
                &mut child_vm,
                &name_owned,
                &params,
                &body,
                &captured_env,
                &args,
                &dummy_expr,
            )
            .map_err(|e: crate::runtime::errors::RuntimeError| e.to_string());

            sender.send(result);
        });

        Some(Ok(Value::Future(handle)))
    }
}

impl VirtualMachine {
    /// Route tool_ stdlib functions with full audit trail and policy wiring.
    ///
    /// Borrow split strategy: permission check (immutable) is performed first via
    /// `self.check_permission`, then mutable borrows of `audit_trail` and `policy_engine`
    /// are taken sequentially for the ToolLib call.
    fn dispatch_tool_function(
        &mut self,
        name: &str,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        use crate::runtime::permissions::PermissionResource;
        use crate::stdlib::ToolLib;

        // Upfront permission gate for tool_exec.
        // tool_list and tool_info are read-only metadata — no process permission needed.
        if name == "tool_exec" {
            self.check_permission(&PermissionResource::System("exec".to_string()), None)?;
        }

        // Pass None for permission_checker: the check was already enforced above.
        // ToolExecutor's permission_checker path is bypassed intentionally — the upfront
        // check_permission call above covers the exec gate.
        ToolLib::call_function(
            name,
            args,
            None,
            Some(&mut self.audit_trail),
            None, // B.1: ai_metadata removed
            Some(&mut self.policy_engine),
        )
    }
}
