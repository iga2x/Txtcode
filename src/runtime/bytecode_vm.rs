use crate::compiler::bytecode::{Bytecode, Constant, Instruction};
use crate::runtime::core::{ScopeManager, Value};
use crate::runtime::errors::RuntimeError;
use crate::runtime::permissions::{Permission, PermissionManager, PermissionResource};
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
/// Executes compiled bytecode instructions
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
    /// Permission manager — enforces the same security model as the AST VM
    permission_manager: PermissionManager,
    /// Module search paths (mirrors ModuleResolver in the AST VM)
    module_search_paths: Vec<std::path::PathBuf>,
    /// Safe-mode flag (disables exec/spawn)
    safe_mode: bool,
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
        }
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

        while self.ip < bytecode.instructions.len() {
            let ip = self.ip;
            self.ip += 1;
            let instruction = bytecode.instructions[ip].clone();
            match self.execute_instruction(&instruction, &bytecode.constants) {
                Ok(()) => {}
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

        Ok(self.stack.pop().unwrap_or(Value::Null))
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

                // Scope-aware pre-flight permission check.
                //
                // The BcvmExecutor below calls check_permission with scope=None, which can
                // fail to match scoped grants (e.g. fs.read:/tmp/*) and cannot pass the real
                // path/hostname to the permission manager. We do the authoritative check here
                // where the actual argument values are available.
                //
                // PARITY NOTE — the bytecode VM currently lacks vs the AST VM:
                //   • intent checking (IntentChecker)
                //   • capability token checking (CapabilityManager / active_capability)
                //   • per-action rate limiting (PolicyEngine)
                //   • audit trail logging (AuditTrail)
                // These are tracked for closure before the bytecode VM graduates to production.
                {
                    let preflight: Option<(PermissionResource, Option<&str>)> = if name
                        == "read_file"
                        || name == "file_exists"
                        || name == "is_file"
                        || name == "is_dir"
                        || name == "list_dir"
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
                        || name == "temp_file"
                        || name == "watch_file"
                        || name == "symlink_create"
                        || name == "mkdir"
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
                        || name == "tcp_connect"
                        || name == "udp_send"
                        || name == "resolve"
                    {
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
                    } else if name == "exec" || name == "spawn" || name == "pipe_exec" {
                        if self.safe_mode {
                            return Err(RuntimeError::new(
                                "exec() is disabled in safe mode (--safe-mode)".to_string(),
                            ));
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
                    } else if name == "getenv" || name == "setenv" {
                        Some((PermissionResource::System("env".to_string()), None))
                    } else {
                        None
                    };

                    if let Some((resource, scope)) = preflight {
                        self.permission_manager
                            .check(&resource, scope)
                            .map_err(|e| RuntimeError::new(format!("Permission error: {}", e)))?;
                    }
                }

                // Fall back to stdlib — BcvmExecutor provides a secondary scopeless check
                // for any stdlib function not covered by the pre-flight above.
                struct BcvmExecutor<'a> {
                    pm: &'a PermissionManager,
                    safe_mode: bool,
                }
                impl<'a> FunctionExecutor for BcvmExecutor<'a> {
                    fn call_function_value(
                        &mut self,
                        _func: &Value,
                        _args: &[Value],
                    ) -> Result<Value, RuntimeError> {
                        Err(RuntimeError::new(
                            "Higher-order stdlib calls not supported in bytecode VM".to_string(),
                        ))
                    }
                }
                impl<'a> PermissionChecker for BcvmExecutor<'a> {
                    fn check_permission(
                        &self,
                        resource: &PermissionResource,
                        scope: Option<&str>,
                    ) -> Result<(), RuntimeError> {
                        // Safe mode: exec functions are already blocked in the pre-flight above.
                        // Guard here catches any exec call that bypasses the pre-flight.
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

                        // Execute module in a sub-VM with inherited permissions
                        let mut sub_vm = BytecodeVM::new();
                        sub_vm.safe_mode = self.safe_mode;
                        // Inherit granted permissions
                        for perm in self.permission_manager.get_granted() {
                            sub_vm.permission_manager.grant(perm.clone());
                        }
                        sub_vm.module_search_paths = self.module_search_paths.clone();
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
                let to_opt = |v: &Value| -> Option<usize> {
                    match v {
                        Value::Integer(i) => Some(*i as usize),
                        _ => None,
                    }
                };
                let step = to_opt(&step_val).unwrap_or(1).max(1);
                match target {
                    Value::Array(arr) => {
                        let s = to_opt(&start_val).unwrap_or(0);
                        let e = to_opt(&end_val).unwrap_or(arr.len()).min(arr.len());
                        let sliced: Vec<Value> = arr[s..e].iter().step_by(step).cloned().collect();
                        self.stack.push(Value::Array(sliced));
                    }
                    Value::String(s_str) => {
                        let chars: Vec<char> = s_str.chars().collect();
                        let s = to_opt(&start_val).unwrap_or(0);
                        let e = to_opt(&end_val).unwrap_or(chars.len()).min(chars.len());
                        let sliced: String = chars[s..e].iter().step_by(step).collect();
                        self.stack.push(Value::String(sliced));
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
