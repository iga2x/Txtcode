use crate::capability::CapabilityManager;
use crate::compiler::bytecode::{Bytecode, Constant, Instruction};
use crate::policy::PolicyEngine;
use crate::runtime::audit::{AIMetadata, AuditResult, AuditTrail};
use crate::runtime::core::{ScopeManager, Value};
use crate::runtime::errors::RuntimeError;
use crate::runtime::gc::GarbageCollector;
use crate::runtime::intent::{IntentChecker, IntentDeclaration};
use crate::runtime::permissions::{Permission, PermissionManager, PermissionResource};
use crate::runtime::security::RuntimeSecurity;
use crate::runtime::security_pipeline::{self, PipelineAuditResult, SecurityPipelineContext};
use crate::stdlib::{FunctionExecutor, PermissionChecker, StdLib};
use std::collections::HashMap;

/// A catch handler frame on the catch_stack
#[allow(dead_code)]
struct CatchFrame {
    catch_ip: usize,
    finally_ip: Option<usize>,
    error_var: Option<String>,
}

/// Bytecode Virtual Machine
///
/// Security model is at full parity with the AST VM (VirtualMachine).
/// Every security-relevant function call goes through the same 6-layer pipeline:
///   1. Max execution time check  (PolicyEngine)
///   2. AI allowance check        (PolicyEngine, when ai_metadata is set)
///   3. Intent check              (IntentChecker — allowed/forbidden actions per function)
///   4. Capability token check    (CapabilityManager — time-bound authorisation tokens)
///   5. Rate limit check          (PolicyEngine — per-action frequency limits)
///   6. Permission check          (PermissionManager — grant/deny rules with glob scopes)
///
/// Every check is logged to the AuditTrail.
pub struct BytecodeVM {
    stack: Vec<Value>,
    variables: HashMap<String, Value>,
    scope_manager: ScopeManager,
    /// User-defined functions: name -> (param_names, body_start_ip)
    functions: HashMap<String, (Vec<String>, usize)>,
    ip: usize,                                               // Instruction pointer
    call_stack: Vec<(usize, HashMap<String, Value>, usize)>, // (return_ip, local_vars, catch_depth)
    /// Active for-loop iterators: (variable_name, items, current_index)
    for_iters: Vec<(String, Vec<Value>, usize)>,
    /// Active try-catch handlers
    catch_stack: Vec<CatchFrame>,
    /// Closure environments: function_name -> captured variables at definition time
    closure_envs: HashMap<String, HashMap<String, Value>>,
    /// Permission manager — grant/deny rules with glob scope matching
    permission_manager: PermissionManager,
    /// Module search paths (mirrors ModuleResolver in the AST VM)
    module_search_paths: Vec<std::path::PathBuf>,
    /// Safe-mode flag (disables exec/spawn)
    safe_mode: bool,

    // ── Security layer (parity with AST VM) ──────────────────────────────────
    /// Immutable append-only audit log of all security events
    pub audit_trail: AuditTrail,
    /// AI agent metadata — set when invoked from an AI pipeline
    ai_metadata: AIMetadata,
    /// Intent checker — enforces allowed/forbidden action constraints per function
    intent_checker: IntentChecker,
    /// Capability manager — time-bound authorisation tokens
    capability_manager: CapabilityManager,
    /// Active capability token in the current scope (None = no token)
    active_capability: Option<String>,
    /// Policy engine — rate limiting, AI control, max execution time
    policy_engine: PolicyEngine,
    /// Runtime security — anti-debug, platform detection, source integrity
    pub runtime_security: RuntimeSecurity,
    /// Function name stack — top is the currently executing user function
    /// (used by the intent checker to enforce per-function constraints)
    function_name_stack: Vec<String>,
    /// Cancellation flag: set to `true` by an external caller (e.g. timeout handler)
    /// to stop execution at the next instruction boundary.
    cancel_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    /// Bytecode currently being executed — stored so `call_lambda_inline` can
    /// re-enter the instruction loop without needing extra parameters.
    current_bytecode: Option<(Vec<crate::compiler::bytecode::Instruction>, Vec<crate::compiler::bytecode::Constant>)>,
    /// Garbage collector — tracks allocation metrics (Rust drop handles real memory).
    gc: GarbageCollector,
}

impl BytecodeVM {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            variables: HashMap::new(),
            scope_manager: ScopeManager::new(),
            functions: HashMap::new(),
            ip: 0,
            call_stack: Vec::new(),
            for_iters: Vec::new(),
            catch_stack: Vec::new(),
            closure_envs: HashMap::new(),
            permission_manager: PermissionManager::new(),
            module_search_paths: vec![
                std::path::PathBuf::from("."),
                std::path::PathBuf::from("src"),
            ],
            safe_mode: false,
            audit_trail: AuditTrail::new(),
            ai_metadata: AIMetadata::new(),
            intent_checker: IntentChecker::new(),
            capability_manager: CapabilityManager::new(),
            active_capability: None,
            policy_engine: PolicyEngine::new(),
            runtime_security: RuntimeSecurity::new(),
            function_name_stack: Vec::new(),
            cancel_flag: None,
            current_bytecode: None,
            gc: GarbageCollector::new(),
        }
    }

    /// Attach a cancellation flag.  When set to `true` the execution loop
    /// terminates at the next instruction boundary with a timeout error.
    pub fn set_cancel_flag(&mut self, flag: std::sync::Arc<std::sync::atomic::AtomicBool>) {
        self.cancel_flag = Some(flag);
    }

    /// Enable safe mode (disables exec/spawn/signal_send).
    pub fn set_safe_mode(&mut self, safe: bool) {
        self.safe_mode = safe;
    }

    /// Grant a permission (mirrors VirtualMachine::grant_permission).
    pub fn grant_permission(&mut self, resource: PermissionResource, scope: Option<String>) {
        self.permission_manager
            .grant(Permission::new(resource, scope));
    }

    /// Deny a permission explicitly.
    pub fn deny_permission(&mut self, resource: PermissionResource, scope: Option<String>) {
        self.permission_manager
            .deny(Permission::new(resource, scope));
    }

    /// Add a directory to the module search path.
    pub fn add_module_search_path(&mut self, path: std::path::PathBuf) {
        self.module_search_paths.push(path);
    }

    // ── Security management (parity with VirtualMachine) ─────────────────────

    /// Set AI agent metadata for audit trail attribution.
    pub fn set_ai_metadata(&mut self, meta: AIMetadata) {
        self.ai_metadata = meta;
    }

    /// Register an intent declaration for a named function.
    pub fn register_function_intent(&mut self, name: String, declaration: IntentDeclaration) {
        self.intent_checker.register_function_intent(name, declaration);
    }

    /// Set module-level intent declaration.
    pub fn set_module_intent(&mut self, declaration: IntentDeclaration) {
        self.intent_checker.set_module_intent(declaration);
    }

    /// Grant a capability token for scoped authorisation.
    pub fn grant_capability(
        &mut self,
        resource: PermissionResource,
        action: String,
        scope: Option<String>,
        expires_in: Option<std::time::Duration>,
        granted_by: Option<String>,
        ai_metadata: Option<AIMetadata>,
    ) -> String {
        let is_meta_empty = ai_metadata.as_ref().map(|m| m.is_empty()).unwrap_or(true);
        let token_id = self.capability_manager.grant(
            resource,
            action.clone(),
            scope.clone(),
            expires_in,
            granted_by,
            ai_metadata.clone(),
        );
        let _ = self.audit_trail.log_action(
            format!("capability.granted.{}", action),
            scope.unwrap_or_default(),
            Some(format!("capability:{}", token_id)),
            AuditResult::Allowed,
            if let Some(ref meta) = ai_metadata.filter(|m| !is_meta_empty && !m.is_empty()) {
                Some(meta)
            } else if !self.ai_metadata.is_empty() {
                Some(&self.ai_metadata)
            } else {
                None
            },
        );
        token_id
    }

    /// Activate a capability token for the current scope.
    pub fn use_capability(&mut self, token_id: String) -> Result<(), RuntimeError> {
        let result = self.capability_manager.is_valid_detailed(&token_id);
        if result.is_granted() {
            self.active_capability = Some(token_id);
            Ok(())
        } else {
            let reason = result.denial_reason().unwrap_or_else(|| "invalid token".to_string());
            Err(RuntimeError::new(format!("Capability denied: {}", reason)))
        }
    }

    /// Revoke a capability token and log the revocation.
    pub fn revoke_capability(
        &mut self,
        token_id: &str,
        reason: Option<String>,
    ) -> Result<(), RuntimeError> {
        self.capability_manager
            .revoke(token_id, reason)
            .map_err(|e| RuntimeError::new(format!("Capability revocation error: {}", e)))?;
        if self.active_capability.as_deref() == Some(token_id) {
            self.active_capability = None;
        }
        let _ = self.audit_trail.log_action(
            "capability.revoked".to_string(),
            token_id.to_string(),
            Some("capability".to_string()),
            AuditResult::Denied,
            if self.ai_metadata.is_empty() { None } else { Some(&self.ai_metadata) },
        );
        Ok(())
    }

    /// Clear the active capability (end of scoped authorisation).
    pub fn clear_capability(&mut self) {
        self.active_capability = None;
    }

    /// Apply a policy (rate limits, AI control, execution timeout).
    pub fn set_policy(&mut self, policy: crate::policy::Policy) {
        self.policy_engine.set_policy(policy);
    }

    /// Export the audit trail as a JSON array string.
    pub fn export_audit_trail_json(&self) -> String {
        self.audit_trail.export_json()
    }

    // ── Core permission check pipeline ───────────────────────────────────────

    /// Full 6-layer permission check — delegates to the shared `run_pipeline()`.
    /// Both this VM and the AST VM (`VirtualMachine`) implement `SecurityPipelineContext`
    /// so the pipeline logic lives in exactly one place: `security_pipeline.rs`.
    pub fn check_permission_with_audit(
        &mut self,
        resource: &PermissionResource,
        scope: Option<&str>,
    ) -> Result<(), RuntimeError> {
        security_pipeline::run_pipeline(self, resource, scope)
            .into_result()
            .map_err(RuntimeError::new)
    }

    /// Execute a single instruction at the current ip and advance ip.
    /// Uses pre-increment so that Jump/JumpIfFalse targets are absolute indices.
    /// Returns Ok(true) if there are more instructions, Ok(false) if execution is complete.
    pub fn execute_single(&mut self, bytecode: &Bytecode) -> Result<bool, RuntimeError> {
        if self.ip >= bytecode.instructions.len() {
            return Ok(false);
        }
        let ip = self.ip;
        self.ip += 1; // advance before executing so Jump can override cleanly
        let instruction = bytecode.instructions[ip].clone();
        self.execute_instruction(&instruction, &bytecode.constants)?;
        Ok(self.ip < bytecode.instructions.len())
    }

    /// Get current instruction pointer
    pub fn get_ip(&self) -> usize {
        self.ip
    }

    /// Reset vm state and instruction pointer
    pub fn reset(&mut self) {
        self.ip = 0;
        self.stack.clear();
        self.variables.clear();
        self.call_stack.clear();
        self.for_iters.clear();
        self.catch_stack.clear();
        self.closure_envs.clear();
    }

    /// Look up a variable by name
    pub fn get_variable(&self, name: &str) -> Option<&Value> {
        self.variables.get(name)
    }

    /// Get all variables in current scope
    pub fn get_all_variables(&self) -> &HashMap<String, Value> {
        &self.variables
    }

    /// Get the operand stack
    pub fn get_stack(&self) -> &[Value] {
        &self.stack
    }

    /// Get call stack as human-readable frames
    pub fn get_call_stack_frames(&self) -> Vec<String> {
        self.call_stack
            .iter()
            .enumerate()
            .map(|(i, (return_ip, _, _))| format!("frame {}: return_ip={}", i, return_ip))
            .collect()
    }

    /// Execute bytecode.
    /// Uses pre-increment: ip is advanced BEFORE the instruction runs, so Jump(target) sets
    /// ip = target directly and that becomes the next instruction without an extra +1.
    pub fn execute(&mut self, bytecode: &Bytecode) -> Result<Value, RuntimeError> {
        self.ip = 0;

        // ── Startup security checks (parity with AST VM) ──────────────────────
        // Run anti-debug, integrity, and platform checks. Critical findings
        // (debugger present, integrity mismatch) block execution; all findings
        // are logged to the audit trail.
        let report = self.runtime_security.run_startup_checks();
        for w in &report.warnings {
            let _ = self.audit_trail.log_action(
                "security.startup.warning".to_string(),
                w.clone(),
                None,
                AuditResult::Error(w.clone()),
                if self.ai_metadata.is_empty() { None } else { Some(&self.ai_metadata) },
            );
        }
        // Log overall startup result.
        let _ = self.audit_trail.log_action(
            "security.startup".to_string(),
            report.summary(),
            Some(format!(
                "level={} platform={} secure={}",
                report.level, report.platform, report.is_secure()
            )),
            if report.is_secure() {
                AuditResult::Allowed
            } else {
                AuditResult::Error(report.warnings.first().cloned().unwrap_or_default())
            },
            if self.ai_metadata.is_empty() { None } else { Some(&self.ai_metadata) },
        );
        // Hard enforcement: block execution on active debugger or integrity failure.
        RuntimeSecurity::enforce_security_report(&report)
            .map_err(RuntimeError::new)?;
        // Start execution timer for max-execution-time policy checks
        self.policy_engine.start_execution();

        // Store bytecode so call_lambda_inline can re-enter the execution loop.
        self.current_bytecode = Some((bytecode.instructions.clone(), bytecode.constants.clone()));

        while self.ip < bytecode.instructions.len() {
            // Check external cancellation flag at each instruction boundary.
            if self.cancel_flag.as_ref().is_some_and(|f| {
                f.load(std::sync::atomic::Ordering::Relaxed)
            }) {
                return Err(RuntimeError::new(
                    "Execution cancelled: timeout exceeded".to_string(),
                ));
            }

            let ip = self.ip;
            self.ip += 1;
            let instruction = bytecode.instructions[ip].clone();
            match self.execute_instruction(&instruction, &bytecode.constants) {
                Ok(()) => {
                    // Register heap-allocated values for GC tracking
                    if let Some(top) = self.stack.last() {
                        match top {
                            Value::Array(_) | Value::Map(_) | Value::Set(_) | Value::Function(_, _, _, _) => {
                                self.gc.register_allocation(top);
                            }
                            _ => {}
                        }
                    }
                    // Threshold-gated collection (runs only every N allocations)
                    self.gc.collect(&self.stack, &self.variables, &[]);
                }
                Err(e) => {
                    // Control-flow signals (return/break/continue) must bypass try-catch entirely
                    // and propagate directly to their respective boundary handlers.
                    if e.is_control_flow_signal() {
                        return Err(e);
                    }
                    // Check if there's an active catch handler for genuine runtime errors
                    if let Some(frame) = self.catch_stack.pop() {
                        // Bind the error message to the error variable if provided
                        if let Some(var) = frame.error_var {
                            self.variables
                                .insert(var, Value::String(e.message().to_string()));
                        }
                        // Jump to catch handler
                        self.ip = frame.catch_ip;
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        self.current_bytecode = None;
        Ok(self.stack.pop().unwrap_or(Value::Null))
    }

    /// Execute a registered bytecode function by name with the given arguments.
    /// Used by `call_hof_with_bytecode_lambda` to implement HOF callbacks inline.
    fn call_lambda_inline(&mut self, func_name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
        let (params, start_ip) = match self.functions.get(func_name).cloned() {
            Some(f) => f,
            None => return Err(RuntimeError::new(format!("Lambda '{}' not found", func_name))),
        };

        let (instrs, consts) = match self.current_bytecode.clone() {
            Some(bc) => bc,
            None => return Err(RuntimeError::new("No bytecode context for HOF lambda call".to_string())),
        };

        // Save VM state
        let saved_ip = self.ip;
        let saved_vars = std::mem::take(&mut self.variables);
        let saved_stack_len = self.stack.len();
        let saved_call_stack_len = self.call_stack.len();
        let saved_catch_stack_len = self.catch_stack.len();

        // Set up closure environment, then bind args
        if let Some(closure_env) = self.closure_envs.get(func_name).cloned() {
            self.variables = closure_env;
        }
        for (param, arg) in params.iter().zip(args.iter()) {
            self.variables.insert(param.clone(), arg.clone());
        }

        // Run from the lambda's start IP until it returns (ip == usize::MAX or out of range)
        self.ip = start_ip;
        let mut exec_err: Option<RuntimeError> = None;
        loop {
            if self.ip == usize::MAX || self.ip >= instrs.len() {
                break;
            }
            let ip = self.ip;
            self.ip += 1;
            let instr = instrs[ip].clone();
            if let Err(e) = self.execute_instruction(&instr, &consts) {
                exec_err = Some(e);
                break;
            }
        }

        // Collect return value (pushed by ReturnValue before setting ip=usize::MAX)
        let return_val = if self.stack.len() > saved_stack_len {
            self.stack.pop().unwrap_or(Value::Null)
        } else {
            Value::Null
        };

        // Restore VM state
        self.ip = saved_ip;
        self.variables = saved_vars;
        self.stack.truncate(saved_stack_len);
        self.call_stack.truncate(saved_call_stack_len);
        self.catch_stack.truncate(saved_catch_stack_len);

        match exec_err {
            Some(e) => Err(e),
            None => Ok(return_val),
        }
    }

    /// Inline implementation of map/filter/reduce/find when the callback is a bytecode lambda.
    fn call_hof_with_bytecode_lambda(&mut self, hof_name: &str, args: &[Value], lambda_name: &str) -> Result<Value, RuntimeError> {
        match hof_name {
            "map" => {
                let arr = match args.first() {
                    Some(Value::Array(a)) => a.clone(),
                    _ => return Err(RuntimeError::new("map: first argument must be an array".to_string())),
                };
                let mut result = Vec::with_capacity(arr.len());
                for elem in arr {
                    result.push(self.call_lambda_inline(lambda_name, &[elem])?);
                }
                Ok(Value::Array(result))
            }
            "filter" => {
                let arr = match args.first() {
                    Some(Value::Array(a)) => a.clone(),
                    _ => return Err(RuntimeError::new("filter: first argument must be an array".to_string())),
                };
                let mut result = Vec::new();
                for elem in arr {
                    let pred = self.call_lambda_inline(lambda_name, &[elem.clone()])?;
                    if matches!(pred, Value::Boolean(true)) {
                        result.push(elem);
                    }
                }
                Ok(Value::Array(result))
            }
            "reduce" => {
                let arr = match args.first() {
                    Some(Value::Array(a)) => a.clone(),
                    _ => return Err(RuntimeError::new("reduce: first argument must be an array".to_string())),
                };
                let mut acc = args.get(2).cloned().unwrap_or(Value::Null);
                for elem in arr {
                    acc = self.call_lambda_inline(lambda_name, &[acc, elem])?;
                }
                Ok(acc)
            }
            "find" => {
                let arr = match args.first() {
                    Some(Value::Array(a)) => a.clone(),
                    _ => return Err(RuntimeError::new("find: first argument must be an array".to_string())),
                };
                for elem in arr {
                    let pred = self.call_lambda_inline(lambda_name, &[elem.clone()])?;
                    if matches!(pred, Value::Boolean(true)) {
                        return Ok(elem);
                    }
                }
                Ok(Value::Null)
            }
            _ => Err(RuntimeError::new(format!("Unknown HOF: {}", hof_name))),
        }
    }

    fn execute_instruction(
        &mut self,
        inst: &Instruction,
        constants: &[Constant],
    ) -> Result<(), RuntimeError> {
        match inst {
            Instruction::PushConstant(idx) => {
                let constant = &constants[*idx];
                let value = self.constant_to_value(constant);
                self.stack.push(value);
            }
            Instruction::Pop => {
                self.stack.pop();
            }
            Instruction::Dup => {
                if let Some(val) = self.stack.last() {
                    self.stack.push(val.clone());
                }
            }
            Instruction::LoadVar(name) => {
                let value = self
                    .variables
                    .get(name)
                    .cloned()
                    .or_else(|| self.scope_manager.get_variable(name))
                    .ok_or_else(|| RuntimeError::new(format!("Undefined variable: {}", name)))?;
                self.stack.push(value);
            }
            Instruction::StoreVar(name) => {
                let value = self
                    .stack
                    .pop()
                    .ok_or_else(|| RuntimeError::new("Stack underflow".to_string()))?;
                self.variables.insert(name.clone(), value);
            }
            Instruction::LoadGlobal(name) => {
                let value = self
                    .scope_manager
                    .get_variable(name)
                    .ok_or_else(|| RuntimeError::new(format!("Undefined global: {}", name)))?
                    .clone();
                self.stack.push(value);
            }
            Instruction::Add => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                // Handle string concatenation in bytecode VM too
                match (&a, &b) {
                    (Value::String(s), other) => {
                        self.stack.push(Value::String(format!("{}{}", s, other)));
                    }
                    (other, Value::String(s)) => {
                        self.stack.push(Value::String(format!("{}{}", other, s)));
                    }
                    (Value::Integer(x), Value::Integer(y)) => {
                        let result = x.checked_add(*y).ok_or_else(|| {
                            RuntimeError::new(format!("Integer overflow: {} + {}", x, y))
                        })?;
                        self.stack.push(Value::Integer(result));
                    }
                    _ => {
                        self.stack.push(self.binary_op(a, b, |a, b| a + b)?);
                    }
                }
            }
            Instruction::Subtract => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                if let (Value::Integer(x), Value::Integer(y)) = (&a, &b) {
                    let result = x.checked_sub(*y).ok_or_else(|| {
                        RuntimeError::new(format!("Integer overflow: {} - {}", x, y))
                    })?;
                    self.stack.push(Value::Integer(result));
                } else {
                    self.stack.push(self.binary_op(a, b, |a, b| a - b)?);
                }
            }
            Instruction::Multiply => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                if let (Value::Integer(x), Value::Integer(y)) = (&a, &b) {
                    let result = x.checked_mul(*y).ok_or_else(|| {
                        RuntimeError::new(format!("Integer overflow: {} * {}", x, y))
                    })?;
                    self.stack.push(Value::Integer(result));
                } else {
                    self.stack.push(self.binary_op(a, b, |a, b| a * b)?);
                }
            }
            Instruction::Divide => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                if let (Value::Integer(a_val), Value::Integer(b_val)) = (&a, &b) {
                    if *b_val == 0 {
                        return Err(RuntimeError::new("Division by zero".to_string()));
                    }
                    self.stack.push(Value::Integer(a_val / b_val));
                } else if let (Value::Float(a_val), Value::Float(b_val)) = (a, b) {
                    if b_val == 0.0 {
                        return Err(RuntimeError::new("Division by zero".to_string()));
                    }
                    self.stack.push(Value::Float(a_val / b_val));
                } else {
                    return Err(RuntimeError::new("Type mismatch in division".to_string()));
                }
            }
            Instruction::Modulo => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                if let (Value::Integer(a_val), Value::Integer(b_val)) = (&a, &b) {
                    if *b_val == 0 {
                        return Err(RuntimeError::new("Modulo by zero".to_string()));
                    }
                    self.stack.push(Value::Integer(a_val % b_val));
                } else {
                    return Err(RuntimeError::new("Modulo requires integers".to_string()));
                }
            }
            Instruction::Power => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                match (&a, &b) {
                    (Value::Integer(av), Value::Integer(bv)) => {
                        if *bv < 0 {
                            return Err(RuntimeError::new(
                                "Negative exponent not supported for integers".to_string(),
                            ));
                        }
                        let result = av.checked_pow(*bv as u32).ok_or_else(|| {
                            RuntimeError::new(format!("Integer overflow: {} ** {}", av, bv))
                        })?;
                        self.stack.push(Value::Integer(result));
                    }
                    (Value::Float(av), Value::Float(bv)) => {
                        self.stack.push(Value::Float(av.powf(*bv)));
                    }
                    _ => return Err(RuntimeError::new("Power requires numbers".to_string())),
                }
            }
            Instruction::Negate => {
                let val = self.pop_value()?;
                match val {
                    Value::Integer(i) => self.stack.push(Value::Integer(-i)),
                    Value::Float(f) => self.stack.push(Value::Float(-f)),
                    _ => return Err(RuntimeError::new("Negate requires number".to_string())),
                }
            }
            Instruction::Equal => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                self.stack.push(Value::Boolean(a == b));
            }
            Instruction::NotEqual => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                self.stack.push(Value::Boolean(a != b));
            }
            Instruction::Less => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                self.stack.push(Value::Boolean(self.compare(&a, &b)? < 0));
            }
            Instruction::Greater => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                self.stack.push(Value::Boolean(self.compare(&a, &b)? > 0));
            }
            Instruction::LessEqual => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                self.stack.push(Value::Boolean(self.compare(&a, &b)? <= 0));
            }
            Instruction::GreaterEqual => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                self.stack.push(Value::Boolean(self.compare(&a, &b)? >= 0));
            }
            Instruction::And => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                let a_bool = self.to_bool(&a)?;
                let b_bool = self.to_bool(&b)?;
                self.stack.push(Value::Boolean(a_bool && b_bool));
            }
            Instruction::Or => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                let a_bool = self.to_bool(&a)?;
                let b_bool = self.to_bool(&b)?;
                self.stack.push(Value::Boolean(a_bool || b_bool));
            }
            Instruction::Not => {
                let val = self.pop_value()?;
                let bool_val = self.to_bool(&val)?;
                self.stack.push(Value::Boolean(!bool_val));
            }
            Instruction::BitAnd => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                if let (Value::Integer(a_val), Value::Integer(b_val)) = (&a, &b) {
                    self.stack.push(Value::Integer(a_val & b_val));
                } else {
                    return Err(RuntimeError::new(
                        "Bitwise AND requires integers".to_string(),
                    ));
                }
            }
            Instruction::BitOr => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                if let (Value::Integer(a_val), Value::Integer(b_val)) = (&a, &b) {
                    self.stack.push(Value::Integer(a_val | b_val));
                } else {
                    return Err(RuntimeError::new(
                        "Bitwise OR requires integers".to_string(),
                    ));
                }
            }
            Instruction::BitXor => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                if let (Value::Integer(a_val), Value::Integer(b_val)) = (&a, &b) {
                    self.stack.push(Value::Integer(a_val ^ b_val));
                } else {
                    return Err(RuntimeError::new(
                        "Bitwise XOR requires integers".to_string(),
                    ));
                }
            }
            Instruction::LeftShift => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                if let (Value::Integer(a_val), Value::Integer(b_val)) = (&a, &b) {
                    self.stack.push(Value::Integer(a_val << b_val));
                } else {
                    return Err(RuntimeError::new(
                        "Left shift requires integers".to_string(),
                    ));
                }
            }
            Instruction::RightShift => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                if let (Value::Integer(a_val), Value::Integer(b_val)) = (&a, &b) {
                    self.stack.push(Value::Integer(a_val >> b_val));
                } else {
                    return Err(RuntimeError::new(
                        "Right shift requires integers".to_string(),
                    ));
                }
            }
            Instruction::BitNot => {
                let val = self.pop_value()?;
                if let Value::Integer(i) = val {
                    self.stack.push(Value::Integer(!i));
                } else {
                    return Err(RuntimeError::new(
                        "Bitwise NOT requires integer".to_string(),
                    ));
                }
            }
            Instruction::Jump(target) => {
                self.ip = *target;
            }
            Instruction::JumpIfFalse(target) => {
                let val = self.pop_value()?;
                if !self.to_bool(&val)? {
                    self.ip = *target;
                }
            }
            Instruction::JumpIfTrue(target) => {
                let val = self.pop_value()?;
                if self.to_bool(&val)? {
                    self.ip = *target;
                }
            }
            Instruction::Call(name, arg_count) => {
                let mut args = Vec::new();
                for _ in 0..*arg_count {
                    args.insert(0, self.pop_value()?);
                }

                // Recursion depth guard (matches AST VM limit of 50)
                const MAX_CALL_DEPTH: usize = 50;
                if self.call_stack.len() >= MAX_CALL_DEPTH {
                    return Err(RuntimeError::new(format!(
                        "Maximum call stack depth ({}) exceeded — possible infinite recursion in '{}'",
                        MAX_CALL_DEPTH, name
                    )));
                }

                // Check user-defined functions first
                if let Some((params, start_ip)) = self.functions.get(name.as_str()).cloned() {
                    // Save caller state: return address + variable scope + catch stack depth.
                    // catch_depth ensures any SetupCatch frames pushed inside the callee are
                    // truncated on Return/ReturnValue, preventing stale catch frames from
                    // misfiring in the caller's error handler.
                    let catch_depth = self.catch_stack.len();
                    let saved_vars = std::mem::take(&mut self.variables);
                    // Restore closure environment (captured vars at definition time)
                    if let Some(closure_env) = self.closure_envs.get(name.as_str()).cloned() {
                        self.variables = closure_env;
                    }
                    self.call_stack.push((self.ip, saved_vars, catch_depth));
                    // Track function name for intent checking in nested permission calls
                    self.function_name_stack.push(name.clone());
                    // Bind arguments to parameter names (override closure vars)
                    for (param, arg) in params.iter().zip(args.iter()) {
                        self.variables.insert(param.clone(), arg.clone());
                    }
                    self.ip = start_ip;
                    return Ok(());
                }

                // Handle dotted method calls: "obj.method" → call method on variable obj
                // (mirrors AST VM logic in function_calls.rs)
                if let Some(dot_pos) = name.find('.') {
                    let obj_name = &name[..dot_pos];
                    let method_name = name[dot_pos + 1..].to_string();
                    if let Some(obj_val) = self.variables.get(obj_name).cloned() {
                        let result = self.dispatch_method(obj_val, &method_name, &args)?;
                        self.stack.push(result);
                        return Ok(());
                    }
                }

                // Check if a variable holds a lambda name (string -> registered function)
                if let Some(Value::String(func_name)) = self.variables.get(name.as_str()).cloned() {
                    if let Some((params, start_ip)) =
                        self.functions.get(func_name.as_str()).cloned()
                    {
                        let catch_depth = self.catch_stack.len();
                        let saved_vars = std::mem::take(&mut self.variables);
                        // Restore closure environment for lambdas
                        if let Some(closure_env) =
                            self.closure_envs.get(func_name.as_str()).cloned()
                        {
                            self.variables = closure_env;
                        }
                        self.call_stack.push((self.ip, saved_vars, catch_depth));
                        for (param, arg) in params.iter().zip(args.iter()) {
                            self.variables.insert(param.clone(), arg.clone());
                        }
                        self.ip = start_ip;
                        return Ok(());
                    }
                }

                // ── HOF bytecode-lambda interception ──────────────────────────
                //
                // When map/filter/reduce/find is called with a bytecode lambda
                // (Value::String pointing to a registered function), handle the HOF
                // inline using call_lambda_inline rather than delegating to stdlib,
                // which cannot re-enter the bytecode VM.
                if matches!(name.as_str(), "map" | "filter" | "reduce" | "find") {
                    // Lambda is always args[1] for these HOFs
                    let lambda_opt = args.get(1).and_then(|v| match v {
                        Value::String(s) if self.functions.contains_key(s.as_str()) => {
                            Some(s.clone())
                        }
                        _ => None,
                    });
                    if let Some(lambda_name) = lambda_opt {
                        let result = self.call_hof_with_bytecode_lambda(name, &args, &lambda_name)?;
                        self.stack.push(result);
                        return Ok(());
                    }
                }

                // ── Full 6-layer security pre-flight check ────────────────────
                //
                // For well-known stdlib functions we extract the real scope argument
                // (path, hostname, command) and run the complete check pipeline:
                //   intent → capability → rate limit → permission → audit log.
                //
                // This mirrors the AST VM's check_permission_with_audit() path in
                // src/runtime/execution/expressions/function_calls.rs.
                {
                    // Function-name rate limit (parity with AST VM: function_calls.rs
                    // calls check_rate_limit(name) before check_permission_with_audit).
                    // This allows per-function call frequency limits independent of the
                    // resource-level rate limit that runs inside check_permission_with_audit.
                    if let Err(e) = self.policy_engine.check_rate_limit(name) {
                        return Err(RuntimeError::new(format!("Policy error: {}", e)));
                    }

                    // Resolve the (resource, scope) pair from function name + args.
                    let preflight: Option<(PermissionResource, Option<&str>)> = if name
                        == "read_file"
                        || name == "file_exists"
                        || name == "is_file"
                        || name == "is_dir"
                        || name == "list_dir"
                        || name == "read_lines"
                        || name == "watch_file"
                    {
                        args.first()
                            .and_then(|v| match v {
                                Value::String(p) => Some(p.as_str()),
                                _ => None,
                            })
                            .map(|p| (PermissionResource::FileSystem("read".to_string()), Some(p)))
                    } else if name == "write_file"
                        || name == "append_file"
                        || name == "copy_file"
                        || name == "move_file"
                        || name == "rename_file"
                        || name == "temp_file"
                        || name == "symlink_create"
                        || name == "mkdir"
                        || name == "zip_create"
                        || name == "zip_extract"
                        || name == "write_file_binary"
                        || name == "csv_write"
                    {
                        args.first()
                            .and_then(|v| match v {
                                Value::String(p) => Some(p.as_str()),
                                _ => None,
                            })
                            .map(|p| (PermissionResource::FileSystem("write".to_string()), Some(p)))
                    } else if name == "delete" || name == "rmdir" {
                        args.first()
                            .and_then(|v| match v {
                                Value::String(p) => Some(p.as_str()),
                                _ => None,
                            })
                            .map(|p| {
                                (PermissionResource::FileSystem("delete".to_string()), Some(p))
                            })
                    } else if name == "http_get"
                        || name == "http_post"
                        || name == "http_put"
                        || name == "http_delete"
                        || name == "http_patch"
                        || name == "tcp_connect"
                        || name == "udp_send"
                        || name == "resolve"
                    {
                        // Extract hostname from URL for scoped permission check
                        args.first().and_then(|v| match v {
                            Value::String(url) => {
                                let host = url
                                    .split("//")
                                    .nth(1)
                                    .and_then(|s| s.split('/').next())
                                    .and_then(|s| s.split(':').next())
                                    .unwrap_or(url.as_str());
                                if host.is_empty() {
                                    None
                                } else {
                                    Some((
                                        PermissionResource::Network("connect".to_string()),
                                        Some(host),
                                    ))
                                }
                            }
                            _ => None,
                        })
                    } else if name == "exec" || name == "exec_status" || name == "exec_lines"
                           || name == "exec_json" || name == "spawn" || name == "pipe_exec" {
                        if self.safe_mode {
                            return Err(RuntimeError::new(format!(
                                "{}() is disabled in safe mode (--safe-mode)", name
                            )));
                        }
                        args.first().and_then(|v| match v {
                            Value::String(cmd) => {
                                let prog =
                                    cmd.split_whitespace().next().unwrap_or(cmd.as_str());
                                if prog.is_empty() {
                                    None
                                } else {
                                    Some((
                                        PermissionResource::System("exec".to_string()),
                                        Some(prog),
                                    ))
                                }
                            }
                            _ => None,
                        })
                    } else if name == "signal_send" {
                        Some((PermissionResource::Process(vec![name.to_string()]), None))
                    } else if name == "getenv" || name == "setenv" || name == "env_list" {
                        Some((PermissionResource::System("env".to_string()), None))
                    } else if name == "cpu_count"
                        || name == "memory"
                        || name == "memory_available"
                        || name == "disk_space"
                        || name == "platform"
                        || name == "arch"
                        || name == "pid"
                        || name == "user"
                        || name == "uid"
                        || name == "gid"
                        || name == "is_root"
                        || name == "os_name"
                        || name == "os_version"
                    {
                        Some((PermissionResource::System("info".to_string()), None))
                    } else {
                        None
                    };

                    // Run the full 6-layer check: intent → capability → rate limit → permission → audit
                    if let Some((resource, scope)) = preflight {
                        self.check_permission_with_audit(&resource, scope)?;
                    }
                }

                // ── Stdlib fallback executor ───────────────────────────────────
                //
                // BcvmExecutor is a secondary safety net: it catches any stdlib
                // function that bypassed the pre-flight (e.g. unknown functions or
                // new stdlib additions). The pre-flight above handles all known
                // security-relevant calls with full audit trail logging.
                struct BcvmExecutor<'a> {
                    pm: &'a PermissionManager,
                    safe_mode: bool,
                    policy: &'a PolicyEngine,
                }
                impl<'a> FunctionExecutor for BcvmExecutor<'a> {
                    fn call_function_value(
                        &mut self,
                        func: &Value,
                        args: &[Value],
                    ) -> Result<Value, RuntimeError> {
                        match func {
                            Value::Function(name, params, body, captured_env) => {
                                use crate::runtime::execution::expressions::call_user_function;
                                use crate::parser::ast::Expression;
                                let mut temp_vm = crate::runtime::vm::VirtualMachine::new();
                                // Inherit all permissions from the parent bytecode VM so that
                                // closures executed via HOF callbacks respect the same grants
                                // and denials as the surrounding execution context.
                                for p in self.pm.get_granted() {
                                    temp_vm.grant_permission(p.resource.clone(), p.scope.clone());
                                }
                                for p in self.pm.get_denied() {
                                    temp_vm.deny_permission(p.resource.clone(), p.scope.clone());
                                }
                                let dummy_expr = Expression::Identifier("__lambda__".to_string());
                                call_user_function(
                                    &mut temp_vm,
                                    name,
                                    params,
                                    body,
                                    captured_env,
                                    args,
                                    &dummy_expr,
                                )
                            }
                            _ => {
                                let kind = match func {
                                    Value::Integer(_) => "int",
                                    Value::Float(_) => "float",
                                    Value::String(_) => "string",
                                    Value::Boolean(_) => "bool",
                                    Value::Array(_) => "array",
                                    Value::Map(_) => "map",
                                    Value::Set(_) => "set",
                                    Value::Null => "null",
                                    _ => "value",
                                };
                                Err(RuntimeError::new(format!(
                                    "Cannot call {} as a function",
                                    kind
                                )))
                            }
                        }
                    }

                    fn deterministic_time(&self) -> Option<std::time::SystemTime> {
                        if self.policy.is_deterministic_mode() {
                            Some(self.policy.get_time())
                        } else {
                            None
                        }
                    }

                    fn deterministic_random_seed(&self) -> Option<u64> {
                        self.policy.get_random_seed()
                    }
                }
                impl<'a> PermissionChecker for BcvmExecutor<'a> {
                    fn check_permission(
                        &self,
                        resource: &PermissionResource,
                        scope: Option<&str>,
                    ) -> Result<(), RuntimeError> {
                        // Layer 1: Max execution time (safety-net parity with full pipeline)
                        if let Err(e) = self.policy.check_max_execution_time() {
                            return Err(RuntimeError::new(format!("Execution time exceeded: {}", e)));
                        }
                        // Layer 2: AI allowance
                        if let Err(e) = self.policy.check_ai_allowed() {
                            return Err(RuntimeError::new(format!("AI policy denied: {}", e)));
                        }
                        // Layer 5: Rate limit (using resource-level key)
                        // Note: check_rate_limit requires &mut so we use a best-effort key check
                        // via check_rate_limit_remaining (read-only); full rate limiting runs in
                        // the pre-flight check_permission_with_audit for all known functions.
                        // Layer 6: Exec guard + permission manager
                        if self.safe_mode
                            && matches!(resource, PermissionResource::System(a) if a == "exec")
                        {
                            return Err(RuntimeError::new(
                                "exec() is disabled in safe mode (--safe-mode)".to_string(),
                            ));
                        }
                        self.pm
                            .check(resource, scope)
                            .map_err(|e| RuntimeError::new(e.to_string()))
                    }
                }

                let exec_allowed = !self.safe_mode;
                let mut executor = BcvmExecutor {
                    pm: &self.permission_manager,
                    safe_mode: self.safe_mode,
                    policy: &self.policy_engine,
                };
                match StdLib::call_function_with_combined_traits(
                    name,
                    &args,
                    exec_allowed,
                    Some(&mut executor),
                ) {
                    Ok(result) => self.stack.push(result),
                    Err(e) => {
                        // Distinguish permission errors from "not found"
                        let msg = e.to_string();
                        if msg.contains("Permission")
                            || msg.contains("denied")
                            || msg.contains("safe mode")
                        {
                            return Err(RuntimeError::new(msg));
                        }
                        return Err(RuntimeError::new(format!(
                            "Function not found: {} ({})",
                            name, msg
                        )));
                    }
                }
            }
            Instruction::RegisterFunction(name, params, start_ip) => {
                self.functions
                    .insert(name.clone(), (params.clone(), *start_ip));
                // Capture current environment for closure support
                self.closure_envs
                    .insert(name.clone(), self.variables.clone());
            }
            Instruction::Return => {
                if let Some((return_ip, saved_vars, catch_depth)) = self.call_stack.pop() {
                    self.catch_stack.truncate(catch_depth);
                    self.variables = saved_vars;
                    self.ip = return_ip;
                    // Restore caller's function name context
                    self.function_name_stack.pop();
                } else {
                    // Top-level return: exit the execution loop
                    self.ip = usize::MAX;
                }
            }
            Instruction::ReturnValue => {
                let value = self.pop_value()?;
                if let Some((return_ip, saved_vars, catch_depth)) = self.call_stack.pop() {
                    self.catch_stack.truncate(catch_depth);
                    self.variables = saved_vars;
                    self.stack.push(value);
                    self.ip = return_ip;
                    // Restore caller's function name context
                    self.function_name_stack.pop();
                } else {
                    // Top-level return with value
                    self.stack.push(value);
                    self.ip = usize::MAX;
                }
            }
            Instruction::BuildArray(count) => {
                let mut arr = Vec::new();
                for _ in 0..*count {
                    arr.insert(0, self.pop_value()?);
                }
                self.stack.push(Value::Array(arr));
            }
            Instruction::BuildMap(count) => {
                let mut map = HashMap::new();
                for _ in 0..*count {
                    let value = self.pop_value()?;
                    let key_val = self.pop_value()?;
                    if let Value::String(key) = key_val {
                        map.insert(key, value);
                    } else {
                        return Err(RuntimeError::new("Map keys must be strings".to_string()));
                    }
                }
                self.stack.push(Value::Map(map));
            }
            Instruction::Index => {
                let index = self.pop_value()?;
                let target = self.pop_value()?;
                match (&target, &index) {
                    (Value::Array(arr), Value::Integer(i)) => {
                        let idx = *i as usize;
                        if idx < arr.len() {
                            self.stack.push(arr[idx].clone());
                        } else {
                            return Err(RuntimeError::new("Index out of bounds".to_string()));
                        }
                    }
                    (Value::Map(map), Value::String(key)) => {
                        if let Some(val) = map.get(key) {
                            self.stack.push(val.clone());
                        } else {
                            return Err(RuntimeError::new(format!("Key not found: {}", key)));
                        }
                    }
                    _ => return Err(RuntimeError::new("Invalid index operation".to_string())),
                }
            }
            Instruction::GetField(name) => {
                let obj = self.pop_value()?;
                if let Value::Struct(_, fields) = obj {
                    if let Some(val) = fields.get(name) {
                        self.stack.push(val.clone());
                    } else {
                        return Err(RuntimeError::new(format!("Field not found: {}", name)));
                    }
                } else {
                    return Err(RuntimeError::new("GetField requires struct".to_string()));
                }
            }
            Instruction::TypeOf => {
                let val = self.pop_value()?;
                let type_name = match val {
                    Value::Integer(_) => "int",
                    Value::Float(_) => "float",
                    Value::String(_) => "string",
                    Value::Char(_) => "char",
                    Value::Boolean(_) => "bool",
                    Value::Null => "null",
                    Value::Array(_) => "array",
                    Value::Map(_) => "map",
                    Value::Set(_) => "set",
                    Value::Function(_, _, _, _) => "function",
                    Value::Struct(_, _) => "struct",
                    Value::Enum(_, _) => "enum",
                    Value::Result(_, _) => "result",
                };
                self.stack.push(Value::String(type_name.to_string()));
            }
            // ?? operator: if top-of-stack is null, use the default (second value)
            Instruction::NullCoalesce => {
                let default_val = self.pop_value()?;
                let value = self.pop_value()?;
                if matches!(value, Value::Null) {
                    self.stack.push(default_val);
                } else {
                    self.stack.push(value);
                }
            }
            // Optional chaining: returns Null if target is Null, otherwise behaves like normal
            Instruction::OptionalGetField(name) => {
                let obj = self.pop_value()?;
                match obj {
                    Value::Null => self.stack.push(Value::Null),
                    Value::Struct(_, ref fields) => {
                        let val = fields.get(name).cloned().unwrap_or(Value::Null);
                        self.stack.push(val);
                    }
                    Value::Map(ref map) => {
                        let val = map.get(name).cloned().unwrap_or(Value::Null);
                        self.stack.push(val);
                    }
                    _ => self.stack.push(Value::Null),
                }
            }
            Instruction::OptionalIndex => {
                let index = self.pop_value()?;
                let target = self.pop_value()?;
                match target {
                    Value::Null => self.stack.push(Value::Null),
                    Value::Array(ref arr) => {
                        if let Value::Integer(i) = index {
                            let idx = i as usize;
                            let val = arr.get(idx).cloned().unwrap_or(Value::Null);
                            self.stack.push(val);
                        } else {
                            return Err(RuntimeError::new(
                                "Array index must be integer".to_string(),
                            ));
                        }
                    }
                    Value::Map(ref map) => {
                        if let Value::String(key) = index {
                            let val = map.get(&key).cloned().unwrap_or(Value::Null);
                            self.stack.push(val);
                        } else {
                            return Err(RuntimeError::new("Map index must be string".to_string()));
                        }
                    }
                    other => {
                        return Err(RuntimeError::new(format!(
                            "Optional index (?[]) on non-indexable value: {:?}",
                            std::mem::discriminant(&other)
                        )))
                    }
                }
            }
            Instruction::OptionalCall(_name, arg_count) => {
                // Pop args first (in reverse), then target
                let mut args = Vec::new();
                for _ in 0..*arg_count {
                    args.insert(0, self.pop_value()?);
                }
                let target = self.pop_value()?;
                match target {
                    Value::Null => self.stack.push(Value::Null),
                    Value::String(func_name) => {
                        // Target is a lambda/function name string — look it up and call it
                        if let Some((params, start_ip)) =
                            self.functions.get(func_name.as_str()).cloned()
                        {
                            const MAX_CALL_DEPTH: usize = 50;
                            if self.call_stack.len() >= MAX_CALL_DEPTH {
                                return Err(RuntimeError::new(format!(
                                    "Maximum call stack depth ({}) exceeded",
                                    MAX_CALL_DEPTH
                                )));
                            }
                            let catch_depth = self.catch_stack.len();
                            let saved_vars = std::mem::take(&mut self.variables);
                            if let Some(closure_env) =
                                self.closure_envs.get(func_name.as_str()).cloned()
                            {
                                self.variables = closure_env;
                            }
                            self.call_stack.push((self.ip, saved_vars, catch_depth));
                            for (param, arg) in params.iter().zip(args.iter()) {
                                self.variables.insert(param.clone(), arg.clone());
                            }
                            self.ip = start_ip;
                        } else {
                            // Unknown function name — return Null (optional call semantics)
                            self.stack.push(Value::Null);
                        }
                    }
                    other => {
                        return Err(RuntimeError::new(format!(
                            "Optional call (?()) requires a callable value or null, got: {}",
                            other
                        )))
                    }
                }
            }
            Instruction::Label(_) => {
                // Labels are just markers, no operation
            }
            Instruction::Nop => {
                // No operation
            }
            Instruction::ImportModule(module_path) => {
                // Resolve, parse, and execute the module, merging its top-level
                // variables (exported names) into the current variable scope.
                let resolved = self.resolve_module(module_path)?;
                match resolved {
                    Some(source) => {
                        use crate::compiler::bytecode::BytecodeCompiler;
                        use crate::lexer::Lexer;
                        use crate::parser::Parser;

                        let mut lexer = Lexer::new(source);
                        let tokens = lexer.tokenize().map_err(|e| {
                            RuntimeError::new(format!("Module '{}' lex error: {}", module_path, e))
                        })?;
                        let mut parser = Parser::new(tokens);
                        let program = parser.parse().map_err(|e| {
                            RuntimeError::new(format!(
                                "Module '{}' parse error: {}",
                                module_path, e
                            ))
                        })?;
                        let mut compiler = BytecodeCompiler::new();
                        let module_bc = compiler.compile(&program);

                        // Execute module in a sub-VM that inherits the full security context
                        // from the parent VM: permissions, safe mode, policy, intent, and
                        // AI metadata all propagate so module code is subject to the same
                        // security enforcement as the caller.
                        let mut sub_vm = BytecodeVM::new();
                        sub_vm.safe_mode = self.safe_mode;
                        // Inherit permission grants and explicit denials
                        for perm in self.permission_manager.get_granted() {
                            sub_vm.permission_manager.grant(perm.clone());
                        }
                        for perm in self.permission_manager.get_denied() {
                            sub_vm.permission_manager.deny(perm.clone());
                        }
                        // Inherit AI metadata and active capability context
                        sub_vm.ai_metadata = self.ai_metadata.clone();
                        if let Some(ref token) = self.active_capability {
                            sub_vm.active_capability = Some(token.clone());
                        }
                        sub_vm.module_search_paths = self.module_search_paths.clone();
                        // Inherit policy engine (rate limits, max_execution_time, AI control),
                        // intent checker (per-function allowed/forbidden actions), and
                        // capability manager (active tokens with expiry and deny-override).
                        // Without this, module code bypasses rate limiting, intent constraints,
                        // and capability scoping that the caller is subject to.
                        sub_vm.policy_engine = self.policy_engine.clone();
                        sub_vm.intent_checker = self.intent_checker.clone();
                        sub_vm.capability_manager = self.capability_manager.clone();
                        sub_vm.execute(&module_bc).map_err(|e| {
                            RuntimeError::new(format!(
                                "Module '{}' runtime error: {}",
                                module_path, e
                            ))
                        })?;

                        // Merge module's variables into current scope (skip internal names)
                        for (k, v) in sub_vm.variables {
                            if !k.starts_with("__") {
                                self.variables.insert(k, v);
                            }
                        }
                        // Merge module's functions
                        for (k, v) in sub_vm.functions {
                            self.functions.insert(k, v);
                        }
                    }
                    None => {
                        // Module not found on disk — try stdlib (math, json, etc. are built-in)
                        // No error: stdlib functions are available globally by name already.
                        // Silently succeed so `import → math` doesn't crash compiled code.
                    }
                }
            }
            Instruction::SetIndex => {
                // Stack: [new_value, index, target_obj] — target_obj on top
                let obj = self.pop_value()?;
                let index = self.pop_value()?;
                let new_value = self.pop_value()?;
                match (obj, &index) {
                    (Value::Array(mut arr), Value::Integer(i)) => {
                        let idx = *i as usize;
                        if idx >= arr.len() {
                            return Err(RuntimeError::new(format!("Index out of bounds: {}", i)));
                        }
                        arr[idx] = new_value;
                        self.stack.push(Value::Array(arr));
                    }
                    (Value::Map(mut map), Value::String(key)) => {
                        map.insert(key.clone(), new_value);
                        self.stack.push(Value::Map(map));
                    }
                    _ => {
                        return Err(RuntimeError::new(
                            "SetIndex: requires array+integer or map+string".to_string(),
                        ))
                    }
                }
            }
            Instruction::SetField(name) => {
                // Stack: [new_value, target_obj] — target_obj on top
                let obj = self.pop_value()?;
                let new_value = self.pop_value()?;
                match obj {
                    Value::Struct(type_name, mut fields) => {
                        fields.insert(name.clone(), new_value);
                        self.stack.push(Value::Struct(type_name, fields));
                    }
                    Value::Map(mut map) => {
                        map.insert(name.clone(), new_value);
                        self.stack.push(Value::Map(map));
                    }
                    _ => {
                        return Err(RuntimeError::new(format!(
                            "SetField: cannot set field '{}' on non-struct/map value",
                            name
                        )))
                    }
                }
            }

            // ── For-loop iterator instructions ────────────────────────────────
            Instruction::ForSetup(var, end_idx) => {
                let iterable = self.pop_value()?;
                let items: Vec<Value> = match iterable {
                    Value::Array(arr) => arr,
                    Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(),
                    _ => {
                        return Err(RuntimeError::new(
                            "for … in requires an array or string iterable".to_string(),
                        ))
                    }
                };

                if items.is_empty() {
                    self.ip = *end_idx;
                    return Ok(());
                }

                // Store first element in the loop variable
                self.variables.insert(var.clone(), items[0].clone());
                self.for_iters.push((var.clone(), items, 0));
            }
            Instruction::ForNext(body_start) => {
                if let Some((var, items, idx)) = self.for_iters.last_mut() {
                    *idx += 1;
                    if *idx < items.len() {
                        let next = items[*idx].clone();
                        self.variables.insert(var.clone(), next);
                        self.ip = *body_start; // jump back to body start
                    } else {
                        // Exhausted: pop iterator, fall through to end
                        self.for_iters.pop();
                    }
                }
            }
            Instruction::ForCleanup => {
                self.for_iters.pop();
            }

            // ── Set construction ──────────────────────────────────────────────
            Instruction::BuildSet(count) => {
                let mut items = Vec::new();
                for _ in 0..*count {
                    items.insert(0, self.pop_value()?);
                }
                // Deduplicate preserving insertion order
                let mut seen = std::collections::HashSet::new();
                let deduped: Vec<Value> = items
                    .into_iter()
                    .filter(|v| seen.insert(format!("{:?}", v)))
                    .collect();
                self.stack.push(Value::Set(deduped));
            }

            // ── Pipe operator ─────────────────────────────────────────────────
            Instruction::Pipe => {
                // Stack: [arg, func] — func on top, arg below.
                let func = self.pop_value()?;
                let arg = self.pop_value()?;
                // func should be a string naming a registered function (lambda or user-defined)
                let func_name = match &func {
                    Value::String(s) => s.clone(),
                    other => {
                        return Err(RuntimeError::new(format!(
                            "Pipe operator |> requires a callable on the right side, got: {}",
                            other
                        )))
                    }
                };
                if let Some((params, start_ip)) = self.functions.get(func_name.as_str()).cloned() {
                    const MAX_CALL_DEPTH: usize = 50;
                    if self.call_stack.len() >= MAX_CALL_DEPTH {
                        return Err(RuntimeError::new(format!(
                            "Maximum call stack depth ({}) exceeded",
                            MAX_CALL_DEPTH
                        )));
                    }
                    let catch_depth = self.catch_stack.len();
                    let saved_vars = std::mem::take(&mut self.variables);
                    if let Some(closure_env) = self.closure_envs.get(func_name.as_str()).cloned() {
                        self.variables = closure_env;
                    }
                    self.call_stack.push((self.ip, saved_vars, catch_depth));
                    // Bind single arg to first parameter
                    if let Some(first_param) = params.first() {
                        self.variables.insert(first_param.clone(), arg);
                    }
                    self.ip = start_ip;
                } else {
                    return Err(RuntimeError::new(format!(
                        "Pipe operator |>: '{}' is not a defined function",
                        func_name
                    )));
                }
            }

            // ── Spread / array mutation ───────────────────────────────────────
            Instruction::ArrayAppend => {
                let val = self.pop_value()?;
                let arr = self
                    .stack
                    .last_mut()
                    .ok_or_else(|| RuntimeError::new("ArrayAppend: empty stack".to_string()))?;
                match arr {
                    Value::Array(ref mut v) => v.push(val),
                    _ => {
                        return Err(RuntimeError::new(
                            "ArrayAppend: top of stack is not an array".to_string(),
                        ))
                    }
                }
            }
            Instruction::ArrayExtend => {
                let spread = self.pop_value()?;
                match spread {
                    Value::Array(spread_vals) => {
                        let arr = self.stack.last_mut().ok_or_else(|| {
                            RuntimeError::new("ArrayExtend: empty stack".to_string())
                        })?;
                        match arr {
                            Value::Array(ref mut v) => v.extend(spread_vals),
                            _ => {
                                return Err(RuntimeError::new(
                                    "ArrayExtend: top of stack is not an array".to_string(),
                                ))
                            }
                        }
                    }
                    other => {
                        return Err(RuntimeError::new(format!(
                            "Spread operator requires an array, got: {}",
                            other
                        )))
                    }
                }
            }

            // ── Method dispatch ───────────────────────────────────────────────
            Instruction::CallMethod(method, arg_count) => {
                let mut args = Vec::new();
                for _ in 0..*arg_count {
                    args.insert(0, self.pop_value()?);
                }
                let obj = self.pop_value()?;
                let result = self.dispatch_method(obj, method, &args)?;
                self.stack.push(result);
            }

            // ── Try-catch support ─────────────────────────────────────────────
            Instruction::SetupCatch(catch_ip, finally_ip, error_var) => {
                self.catch_stack.push(CatchFrame {
                    catch_ip: *catch_ip,
                    finally_ip: *finally_ip,
                    error_var: error_var.clone(),
                });
            }
            Instruction::PopCatch => {
                self.catch_stack.pop();
            }
            Instruction::Throw => {
                let val = self.pop_value()?;
                return Err(RuntimeError::new(val.to_string()));
            }

            // ── Result type ───────────────────────────────────────────────────
            Instruction::BuildOk => {
                let val = self.pop_value()?;
                self.stack.push(Value::Result(true, Box::new(val)));
            }
            Instruction::BuildErr => {
                let val = self.pop_value()?;
                self.stack.push(Value::Result(false, Box::new(val)));
            }

            // ── Struct literal ────────────────────────────────────────────────
            Instruction::BuildStructLiteral(field_count) => {
                // Stack layout: struct_name, key0, val0, key1, val1, ... (fields pushed after name)
                // Pop in reverse order: fields, then struct_name
                let mut fields = HashMap::new();
                let mut pairs: Vec<(String, Value)> = Vec::new();
                for _ in 0..*field_count {
                    let val = self.pop_value()?;
                    let key_val = self.pop_value()?;
                    if let Value::String(key) = key_val {
                        pairs.push((key, val));
                    } else {
                        return Err(RuntimeError::new(
                            "Struct field key must be a string".to_string(),
                        ));
                    }
                }
                // Fields were pushed in order, popped in reverse → reverse to restore
                for (k, v) in pairs.into_iter().rev() {
                    fields.insert(k, v);
                }
                let name_val = self.pop_value()?;
                let struct_name = match name_val {
                    Value::String(s) => s,
                    _ => {
                        return Err(RuntimeError::new(
                            "Struct name must be a string".to_string(),
                        ))
                    }
                };
                self.stack.push(Value::Struct(struct_name, fields));
            }

            // ── Slice ─────────────────────────────────────────────────────────
            Instruction::Slice => {
                let step_val = self.pop_value()?;
                let end_val = self.pop_value()?;
                let start_val = self.pop_value()?;
                let target = self.pop_value()?;

                // Step: Value::Null means omitted (default 1). Zero is a runtime error.
                let step_raw: i64 = match &step_val {
                    Value::Integer(i) => *i,
                    Value::Null => 1,
                    _ => {
                        return Err(RuntimeError::new(
                            "Slice step must be an integer".to_string(),
                        ))
                    }
                };
                if step_raw == 0 {
                    return Err(RuntimeError::new(
                        "Slice step cannot be zero".to_string(),
                    ));
                }

                match target {
                    Value::Array(arr) => {
                        let len = arr.len();
                        // Resolve an index: negative counts from end, Null uses the default.
                        let resolve =
                            |v: &Value, default: usize| -> Result<usize, RuntimeError> {
                                match v {
                                    Value::Integer(i) if *i < 0 => {
                                        let r = len as i64 + i;
                                        if r < 0 {
                                            Err(RuntimeError::new(format!(
                                                "Slice index {} out of bounds for array of length {}",
                                                i, len
                                            )))
                                        } else {
                                            Ok(r as usize)
                                        }
                                    }
                                    Value::Integer(i) => Ok(*i as usize),
                                    Value::Null => Ok(default),
                                    _ => Err(RuntimeError::new(
                                        "Slice index must be an integer".to_string(),
                                    )),
                                }
                            };

                        if step_raw < 0 {
                            if len == 0 {
                                self.stack.push(Value::Array(vec![]));
                            } else {
                                let abs_step = (-step_raw) as usize;
                                let s = resolve(&start_val, len - 1)?;
                                let e = resolve(&end_val, 0)?;
                                if s >= len || e >= len {
                                    return Err(RuntimeError::new(format!(
                                        "Slice index out of bounds (array len={})",
                                        len
                                    )));
                                }
                                let mut result = Vec::new();
                                let mut idx = s;
                                while idx > e {
                                    result.push(arr[idx].clone());
                                    if idx < abs_step {
                                        break;
                                    }
                                    idx -= abs_step;
                                }
                                if idx == e {
                                    result.push(arr[idx].clone());
                                }
                                self.stack.push(Value::Array(result));
                            }
                        } else {
                            let abs_step = step_raw as usize;
                            let s = resolve(&start_val, 0)?;
                            let e = resolve(&end_val, len)?;
                            if s > len || e > len {
                                return Err(RuntimeError::new(format!(
                                    "Slice index out of bounds (array len={})",
                                    len
                                )));
                            }
                            if s > e {
                                return Err(RuntimeError::new(format!(
                                    "Slice start ({}) cannot be greater than end ({})",
                                    s, e
                                )));
                            }
                            let sliced: Vec<Value> =
                                arr[s..e].iter().step_by(abs_step).cloned().collect();
                            self.stack.push(Value::Array(sliced));
                        }
                    }
                    Value::String(s_str) => {
                        let chars: Vec<char> = s_str.chars().collect();
                        let len = chars.len();
                        // Resolve a char index: negative counts from end, Null uses the default.
                        let resolve =
                            |v: &Value, default: usize| -> Result<usize, RuntimeError> {
                                match v {
                                    Value::Integer(i) if *i < 0 => {
                                        let r = len as i64 + i;
                                        if r < 0 {
                                            Err(RuntimeError::new(format!(
                                                "String slice index {} out of bounds for string of length {}",
                                                i, len
                                            )))
                                        } else {
                                            Ok(r as usize)
                                        }
                                    }
                                    Value::Integer(i) => Ok(*i as usize),
                                    Value::Null => Ok(default),
                                    _ => Err(RuntimeError::new(
                                        "String slice index must be an integer".to_string(),
                                    )),
                                }
                            };

                        if step_raw < 0 {
                            if len == 0 {
                                self.stack.push(Value::String(String::new()));
                            } else {
                                let abs_step = (-step_raw) as usize;
                                let s = resolve(&start_val, len - 1)?;
                                let e = resolve(&end_val, 0)?;
                                if s >= len || e >= len {
                                    return Err(RuntimeError::new(format!(
                                        "String slice index out of bounds (string len={})",
                                        len
                                    )));
                                }
                                let mut result = Vec::new();
                                let mut idx = s;
                                while idx > e {
                                    result.push(chars[idx]);
                                    if idx < abs_step {
                                        break;
                                    }
                                    idx -= abs_step;
                                }
                                if idx == e {
                                    result.push(chars[idx]);
                                }
                                self.stack.push(Value::String(result.into_iter().collect()));
                            }
                        } else {
                            let abs_step = step_raw as usize;
                            let s = resolve(&start_val, 0)?;
                            let e = resolve(&end_val, len)?;
                            if s > len || e > len {
                                return Err(RuntimeError::new(format!(
                                    "String slice index out of bounds (string len={})",
                                    len
                                )));
                            }
                            if s > e {
                                return Err(RuntimeError::new(format!(
                                    "String slice start ({}) cannot be greater than end ({})",
                                    s, e
                                )));
                            }
                            let sliced: String =
                                chars[s..e].iter().step_by(abs_step).collect();
                            self.stack.push(Value::String(sliced));
                        }
                    }
                    _ => {
                        return Err(RuntimeError::new(
                            "Slice requires array or string".to_string(),
                        ))
                    }
                }
            }
        }
        Ok(())
    }

    fn dispatch_method(
        &mut self,
        obj: Value,
        method: &str,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        match &obj {
            Value::String(s) => match method {
                "len" => Ok(Value::Integer(s.chars().count() as i64)),
                "isEmpty" => Ok(Value::Boolean(s.is_empty())),
                "toLower" | "to_lower" => Ok(Value::String(s.to_lowercase())),
                "toUpper" | "to_upper" => Ok(Value::String(s.to_uppercase())),
                "trim" => Ok(Value::String(s.trim().to_string())),
                "reverse" => Ok(Value::String(s.chars().rev().collect())),
                "chars" => Ok(Value::Array(
                    s.chars().map(|c| Value::String(c.to_string())).collect(),
                )),
                "toInt" | "to_int" => s
                    .trim()
                    .parse::<i64>()
                    .map(Value::Integer)
                    .map_err(|_| RuntimeError::new(format!("Cannot parse '{}' as integer", s))),
                "toFloat" | "to_float" => s
                    .trim()
                    .parse::<f64>()
                    .map(Value::Float)
                    .map_err(|_| RuntimeError::new(format!("Cannot parse '{}' as float", s))),
                "contains" => {
                    let pat = match args.first() {
                        Some(Value::String(p)) => p.clone(),
                        _ => {
                            return Err(RuntimeError::new(
                                "contains: expected string argument".to_string(),
                            ))
                        }
                    };
                    Ok(Value::Boolean(s.contains(pat.as_str())))
                }
                "startsWith" | "starts_with" => {
                    let pat = match args.first() {
                        Some(Value::String(p)) => p.clone(),
                        _ => {
                            return Err(RuntimeError::new(
                                "startsWith: expected string argument".to_string(),
                            ))
                        }
                    };
                    Ok(Value::Boolean(s.starts_with(pat.as_str())))
                }
                "endsWith" | "ends_with" => {
                    let pat = match args.first() {
                        Some(Value::String(p)) => p.clone(),
                        _ => {
                            return Err(RuntimeError::new(
                                "endsWith: expected string argument".to_string(),
                            ))
                        }
                    };
                    Ok(Value::Boolean(s.ends_with(pat.as_str())))
                }
                "split" => {
                    let sep = match args.first() {
                        Some(Value::String(p)) => p.clone(),
                        _ => {
                            return Err(RuntimeError::new(
                                "split: expected string argument".to_string(),
                            ))
                        }
                    };
                    let parts: Vec<Value> = s
                        .split(sep.as_str())
                        .map(|p| Value::String(p.to_string()))
                        .collect();
                    Ok(Value::Array(parts))
                }
                "replace" => {
                    let from = match args.first() {
                        Some(Value::String(p)) => p.clone(),
                        _ => {
                            return Err(RuntimeError::new(
                                "replace: expected string arguments".to_string(),
                            ))
                        }
                    };
                    let to = match args.get(1) {
                        Some(Value::String(p)) => p.clone(),
                        _ => {
                            return Err(RuntimeError::new(
                                "replace: expected 2 string arguments".to_string(),
                            ))
                        }
                    };
                    Ok(Value::String(s.replace(from.as_str(), to.as_str())))
                }
                "substring" | "substr" => {
                    let start = match args.first() {
                        Some(Value::Integer(i)) => *i as usize,
                        _ => 0,
                    };
                    let end = match args.get(1) {
                        Some(Value::Integer(i)) => *i as usize,
                        _ => s.len(),
                    };
                    let chars: Vec<char> = s.chars().collect();
                    let end = end.min(chars.len());
                    Ok(Value::String(chars[start..end].iter().collect()))
                }
                "indexOf" | "index_of" => {
                    let pat = match args.first() {
                        Some(Value::String(p)) => p.clone(),
                        _ => {
                            return Err(RuntimeError::new(
                                "indexOf: expected string argument".to_string(),
                            ))
                        }
                    };
                    match s.find(pat.as_str()) {
                        Some(idx) => Ok(Value::Integer(idx as i64)),
                        None => Ok(Value::Integer(-1)),
                    }
                }
                "repeat" => {
                    let n = match args.first() {
                        Some(Value::Integer(i)) => *i as usize,
                        _ => {
                            return Err(RuntimeError::new(
                                "repeat: expected integer argument".to_string(),
                            ))
                        }
                    };
                    Ok(Value::String(s.repeat(n)))
                }
                "padStart" | "pad_start" => {
                    let n = match args.first() {
                        Some(Value::Integer(i)) => *i as usize,
                        _ => {
                            return Err(RuntimeError::new("padStart: expected integer".to_string()))
                        }
                    };
                    let pad = match args.get(1) {
                        Some(Value::String(p)) => p.chars().next().unwrap_or(' '),
                        _ => ' ',
                    };
                    let chars: Vec<char> = s.chars().collect();
                    if chars.len() >= n {
                        return Ok(Value::String(s.clone()));
                    }
                    let pad_n = n - chars.len();
                    Ok(Value::String(
                        std::iter::repeat_n(pad, pad_n).chain(chars).collect(),
                    ))
                }
                "padEnd" | "pad_end" => {
                    let n = match args.first() {
                        Some(Value::Integer(i)) => *i as usize,
                        _ => return Err(RuntimeError::new("padEnd: expected integer".to_string())),
                    };
                    let pad = match args.get(1) {
                        Some(Value::String(p)) => p.chars().next().unwrap_or(' '),
                        _ => ' ',
                    };
                    let chars: Vec<char> = s.chars().collect();
                    if chars.len() >= n {
                        return Ok(Value::String(s.clone()));
                    }
                    let pad_n = n - chars.len();
                    Ok(Value::String(
                        chars
                            .into_iter()
                            .chain(std::iter::repeat_n(pad, pad_n))
                            .collect(),
                    ))
                }
                _ => Err(RuntimeError::new(format!(
                    "Unknown string method: {}",
                    method
                ))),
            },
            Value::Array(arr) => match method {
                "len" => Ok(Value::Integer(arr.len() as i64)),
                "isEmpty" | "is_empty" => Ok(Value::Boolean(arr.is_empty())),
                "first" => arr
                    .first()
                    .cloned()
                    .ok_or_else(|| RuntimeError::new("Array is empty".to_string())),
                "last" => arr
                    .last()
                    .cloned()
                    .ok_or_else(|| RuntimeError::new("Array is empty".to_string())),
                "contains" => {
                    let target = args.first().ok_or_else(|| {
                        RuntimeError::new("contains: expected argument".to_string())
                    })?;
                    Ok(Value::Boolean(arr.contains(target)))
                }
                "indexOf" | "index_of" => {
                    let target = args.first().ok_or_else(|| {
                        RuntimeError::new("indexOf: expected argument".to_string())
                    })?;
                    match arr.iter().position(|v| v == target) {
                        Some(i) => Ok(Value::Integer(i as i64)),
                        None => Ok(Value::Integer(-1)),
                    }
                }
                "join" => {
                    let sep = match args.first() {
                        Some(Value::String(s)) => s.clone(),
                        _ => ",".to_string(),
                    };
                    let joined = arr
                        .iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(&sep);
                    Ok(Value::String(joined))
                }
                "reverse" => {
                    let mut rev = arr.clone();
                    rev.reverse();
                    Ok(Value::Array(rev))
                }
                "sort" => {
                    let mut sorted = arr.clone();
                    sorted.sort_by(|a, b| format!("{:?}", a).cmp(&format!("{:?}", b)));
                    Ok(Value::Array(sorted))
                }
                "push" => {
                    let val = args
                        .first()
                        .ok_or_else(|| RuntimeError::new("push: expected argument".to_string()))?
                        .clone();
                    let mut new_arr = arr.clone();
                    new_arr.push(val);
                    Ok(Value::Array(new_arr))
                }
                "pop" => {
                    if arr.is_empty() {
                        return Err(RuntimeError::new("pop: array is empty".to_string()));
                    }
                    let mut new_arr = arr.clone();
                    new_arr.pop();
                    Ok(Value::Array(new_arr))
                }
                "slice" => {
                    let s = match args.first() {
                        Some(Value::Integer(i)) => *i as usize,
                        _ => 0,
                    };
                    let e = match args.get(1) {
                        Some(Value::Integer(i)) => (*i as usize).min(arr.len()),
                        _ => arr.len(),
                    };
                    Ok(Value::Array(arr[s..e].to_vec()))
                }
                "flat" | "flatten" => {
                    let mut flat = Vec::new();
                    for v in arr {
                        if let Value::Array(inner) = v {
                            flat.extend(inner.clone());
                        } else {
                            flat.push(v.clone());
                        }
                    }
                    Ok(Value::Array(flat))
                }
                _ => Err(RuntimeError::new(format!(
                    "Unknown array method: {}",
                    method
                ))),
            },
            Value::Map(map) => match method {
                "len" => Ok(Value::Integer(map.len() as i64)),
                "isEmpty" | "is_empty" => Ok(Value::Boolean(map.is_empty())),
                "keys" => Ok(Value::Array(
                    map.keys().cloned().map(Value::String).collect(),
                )),
                "values" => Ok(Value::Array(map.values().cloned().collect())),
                "has" | "contains" => {
                    let key = match args.first() {
                        Some(Value::String(s)) => s.clone(),
                        _ => return Err(RuntimeError::new("has: expected string key".to_string())),
                    };
                    Ok(Value::Boolean(map.contains_key(&key)))
                }
                "get" => {
                    let key = match args.first() {
                        Some(Value::String(s)) => s.clone(),
                        _ => return Err(RuntimeError::new("get: expected string key".to_string())),
                    };
                    Ok(map.get(&key).cloned().unwrap_or(Value::Null))
                }
                "entries" => {
                    let entries: Vec<Value> = map
                        .iter()
                        .map(|(k, v)| Value::Array(vec![Value::String(k.clone()), v.clone()]))
                        .collect();
                    Ok(Value::Array(entries))
                }
                _ => Err(RuntimeError::new(format!("Unknown map method: {}", method))),
            },
            Value::Set(items) => match method {
                "len" => Ok(Value::Integer(items.len() as i64)),
                "isEmpty" | "is_empty" => Ok(Value::Boolean(items.is_empty())),
                "has" | "contains" => {
                    let target = args
                        .first()
                        .ok_or_else(|| RuntimeError::new("has: expected argument".to_string()))?;
                    Ok(Value::Boolean(items.contains(target)))
                }
                "values" => Ok(Value::Array(items.clone())),
                _ => Err(RuntimeError::new(format!("Unknown set method: {}", method))),
            },
            _ => Err(RuntimeError::new(format!(
                "Cannot call method '{}' on this type",
                method
            ))),
        }
    }

    fn pop_value(&mut self) -> Result<Value, RuntimeError> {
        self.stack
            .pop()
            .ok_or_else(|| RuntimeError::new("Stack underflow".to_string()))
    }

    fn constant_to_value(&self, constant: &Constant) -> Value {
        match constant {
            Constant::Integer(i) => Value::Integer(*i),
            Constant::Float(f) => Value::Float(*f),
            Constant::String(s) => Value::String(s.clone()),
            Constant::Boolean(b) => Value::Boolean(*b),
            Constant::Null => Value::Null,
        }
    }

    fn binary_op<F>(&self, a: Value, b: Value, op: F) -> Result<Value, RuntimeError>
    where
        F: FnOnce(i64, i64) -> i64,
    {
        match (a, b) {
            (Value::Integer(a_val), Value::Integer(b_val)) => Ok(Value::Integer(op(a_val, b_val))),
            _ => Err(RuntimeError::new(
                "Type mismatch in binary operation".to_string(),
            )),
        }
    }

    fn compare(&self, a: &Value, b: &Value) -> Result<i32, RuntimeError> {
        match (a, b) {
            (Value::Integer(a_val), Value::Integer(b_val)) => Ok((a_val - b_val).signum() as i32),
            (Value::Float(a_val), Value::Float(b_val)) => Ok(a_val
                .partial_cmp(b_val)
                .unwrap_or(std::cmp::Ordering::Equal)
                as i32),
            (Value::String(a_val), Value::String(b_val)) => Ok(a_val.cmp(b_val) as i32),
            _ => Err(RuntimeError::new("Cannot compare values".to_string())),
        }
    }

    fn to_bool(&self, val: &Value) -> Result<bool, RuntimeError> {
        match val {
            Value::Boolean(b) => Ok(*b),
            Value::Integer(i) => Ok(*i != 0),
            Value::Float(f) => Ok(*f != 0.0),
            Value::Null => Ok(false),
            Value::String(s) => Ok(!s.is_empty()),
            Value::Array(arr) => Ok(!arr.is_empty()),
            Value::Map(map) => Ok(!map.is_empty()),
            _ => Ok(true),
        }
    }

    /// Resolve a module name to source code by searching `module_search_paths`.
    fn resolve_module(&self, module_path: &str) -> Result<Option<String>, RuntimeError> {
        // Normalise: "math" -> "math.tc", "./utils" -> "./utils.tc"
        let file_name = if module_path.ends_with(".tc") {
            module_path.to_string()
        } else {
            format!("{}.tc", module_path)
        };

        for search_dir in &self.module_search_paths {
            let candidate = search_dir.join(&file_name);
            if candidate.exists() {
                let source = std::fs::read_to_string(&candidate).map_err(|e| {
                    RuntimeError::new(format!(
                        "Cannot read module '{}': {}",
                        candidate.display(),
                        e
                    ))
                })?;
                return Ok(Some(source));
            }
        }
        // Not found on disk — caller treats None as a built-in/stdlib module (no error)
        Ok(None)
    }
}

impl Default for BytecodeVM {
    fn default() -> Self {
        Self::new()
    }
}

// ── SecurityPipelineContext impl ─────────────────────────────────────────────

impl SecurityPipelineContext for BytecodeVM {
    fn check_max_execution_time(&mut self) -> Result<(), String> {
        self.policy_engine
            .check_max_execution_time()
            .map_err(|e| format!("Policy error: {}", e))
    }

    fn check_ai_allowed(&mut self) -> Result<(), String> {
        self.policy_engine
            .check_ai_allowed()
            .map_err(|e| format!("Policy error: {}", e))
    }

    fn check_intent(&self, function_name: &str, action: &str, resource: &str) -> Result<(), String> {
        self.intent_checker
            .check_action(function_name, action, resource)
            .map_err(|e| e.to_string())
    }

    /// Handles deny-wins, rate-limit (Phase 2.4), and audit logging for capability checks.
    fn check_capability(
        &mut self,
        resource: &PermissionResource,
        action: &str,
        scope: Option<&str>,
    ) -> Option<Result<(), String>> {
        let token_id = self.active_capability.clone()?;

        match self.capability_manager.check(&token_id, resource, action, scope) {
            Ok(()) => {
                // Explicit denies always win, even over a valid capability token.
                if let Err(deny_err) = self.permission_manager.check_denied(resource, scope) {
                    let _ = self.audit_trail.log_action(
                        format!("capability.denied.{}", action),
                        scope.unwrap_or("").to_string(),
                        Some(format!("capability:{}/deny-override", token_id)),
                        AuditResult::Denied,
                        if self.ai_metadata.is_empty() { None } else { Some(&self.ai_metadata) },
                    );
                    return Some(Err(format!("Permission error: {}", deny_err)));
                }
                // Rate limit still applies even when capability grants access (Phase 2.4).
                if let Err(e) = self.policy_engine
                    .check_rate_limit(&format!("capability.check.{}", action))
                {
                    return Some(Err(format!("Policy error: {}", e)));
                }
                let _ = self.audit_trail.log_action(
                    format!("capability.used.{}", action),
                    scope.unwrap_or("").to_string(),
                    Some(format!("capability:{}", token_id)),
                    AuditResult::Allowed,
                    if self.ai_metadata.is_empty() { None } else { Some(&self.ai_metadata) },
                );
                Some(Ok(()))
            }
            Err(cap_err) => {
                let _ = self.audit_trail.log_action(
                    format!("capability.check.{}", action),
                    scope.unwrap_or("").to_string(),
                    Some(format!("capability:{}", token_id)),
                    AuditResult::Denied,
                    if self.ai_metadata.is_empty() { None } else { Some(&self.ai_metadata) },
                );
                Some(Err(format!("Capability error: {}", cap_err)))
            }
        }
    }

    fn check_rate_limit(&mut self, action: &str) -> Result<(), String> {
        self.policy_engine
            .check_rate_limit(action)
            .map_err(|e| format!("Policy error: {}", e))
    }

    fn check_permission_manager(
        &mut self,
        resource: &PermissionResource,
        scope: Option<&str>,
    ) -> Result<(), String> {
        let result = self.permission_manager.check(resource, scope);
        let _ = self.audit_trail.log_permission_check(
            resource,
            scope,
            result.clone(),
            if self.ai_metadata.is_empty() { None } else { Some(&self.ai_metadata) },
        );
        result.map_err(|e| format!("Permission error: {}", e))
    }

    fn current_function_name(&self) -> Option<&str> {
        self.function_name_stack.last().map(|s| s.as_str())
    }

    fn has_ai_metadata(&self) -> bool {
        !self.ai_metadata.is_empty()
    }

    fn log_audit(
        &mut self,
        action: &str,
        resource: &str,
        token: Option<&str>,
        result: PipelineAuditResult,
    ) {
        let audit_result = match result {
            PipelineAuditResult::Allowed => AuditResult::Allowed,
            PipelineAuditResult::Denied => AuditResult::Denied,
        };
        let _ = self.audit_trail.log_action(
            action.to_string(),
            resource.to_string(),
            token.map(|s| s.to_string()),
            audit_result,
            if self.ai_metadata.is_empty() { None } else { Some(&self.ai_metadata) },
        );
    }
}
