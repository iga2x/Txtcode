// Function call evaluation (stdlib, user functions, struct instantiation)

use super::ExpressionVM;
use std::sync::Arc;
use crate::parser::ast::{Expression, Span, Statement};
use crate::runtime::core::{CallFrame, Value};
use crate::runtime::errors::RuntimeError;
use crate::runtime::permission_map;
use crate::tools::logger::{log_debug, log_warn};
use std::collections::HashMap;
use std::cell::RefCell;

// P.4: Thread-local pool of pre-allocated argument vectors.
// Reusing cleared Vecs avoids per-call heap allocation for the args slice.
// Re-entrancy is safe: each call level pops its own Vec from the pool
// (or allocates a fresh one if the pool is empty); nested calls do the same.
// The pool depth grows to match maximum observed call depth, capped
// by the natural call-stack limit.
thread_local! {
    static ARG_POOL: RefCell<Vec<Vec<Value>>> = RefCell::new(Vec::with_capacity(16));
}

/// RAII guard: returns the Vec to the pool when dropped, even on error paths.
struct PooledArgs(Vec<Value>);

impl PooledArgs {
    fn acquire(capacity: usize) -> Self {
        let vec = ARG_POOL.with(|pool| pool.borrow_mut().pop())
            .unwrap_or_else(|| Vec::with_capacity(capacity.max(4)));
        PooledArgs(vec)
    }
}

impl Drop for PooledArgs {
    fn drop(&mut self) {
        self.0.clear();
        let vec = std::mem::take(&mut self.0);
        ARG_POOL.with(|pool| pool.borrow_mut().push(vec));
    }
}

pub fn evaluate_function_call<VM: ExpressionVM>(
    vm: &mut VM,
    name: &str,
    arguments: &[Expression],
    expr: &Expression,
) -> Result<Value, RuntimeError> {
    // P.4: acquire a pooled Vec to avoid per-call allocation.
    let mut pooled = PooledArgs::acquire(arguments.len());
    for arg in arguments {
        pooled.0.push(super::ExpressionEvaluator::evaluate(vm, arg)?);
    }
    let args: &[Value] = &pooled.0;

    // Single canonical permission gate: intent → capability → rate-limit → permission → audit.
    // The resource type and scope are determined by the central permission map so that adding a
    // new privileged function requires a change in exactly one place.
    // When scope cannot be extracted (missing or non-string first argument) we pass None —
    // the permission manager will only allow the call if a scope-less grant exists.
    // We do NOT skip the check: an unresolvable scope should produce a denial, not a bypass.
    if let Some(resource) = permission_map::map_function_to_permission(name) {
        let scope = permission_map::extract_permission_scope(&resource, args);
        vm.check_rate_limit(name)?;
        vm.check_permission_with_audit(&resource, scope.as_deref())?;
    }

    // Task 15.3: Handle with_timeout before stdlib — needs VM access to spawn threads
    if name == "with_timeout" {
        if args.len() < 2 {
            return Err(vm.create_error(
                "with_timeout requires 2 arguments (duration_ms, fn)".to_string(),
            ));
        }
        let ms = match &args[0] {
            Value::Integer(n) => *n as u64,
            Value::Float(f) => *f as u64,
            _ => return Err(vm.create_error(
                "with_timeout: first argument must be a number (milliseconds)".to_string(),
            )),
        };
        let func = args[1].clone();
        return vm.with_timeout_function(ms, func);
    }

    // Task 20.2: Handle async_run before stdlib — needs VM access to spawn threads
    if name == "async_run" {
        let func = args.first().cloned().ok_or_else(|| {
            vm.create_error("async_run requires 1 argument (a zero-arg closure)".to_string())
        })?;
        return vm.async_run(func);
    }

    // D.2: async_run_scoped(fn, [allowed_permissions]) — runs closure with a
    // permission-restricted subset of the parent's grants.
    if name == "async_run_scoped" {
        let func = args.first().cloned().ok_or_else(|| {
            vm.create_error(
                "async_run_scoped requires 2 arguments: closure and allowed-permissions array".to_string()
            )
        })?;
        let allowed = args.get(1).cloned().unwrap_or(Value::Array(vec![]));
        return vm.async_run_scoped(func, allowed);
    }

    // O.4: async_run_timeout(fn, timeout_ms) — like async_run but with a timeout.
    if name == "async_run_timeout" {
        let func = args.first().cloned().ok_or_else(|| {
            vm.create_error(
                "async_run_timeout requires 2 arguments: closure and timeout_ms (integer)".to_string()
            )
        })?;
        let timeout_val = args.get(1).cloned().ok_or_else(|| {
            vm.create_error(
                "async_run_timeout requires 2 arguments: closure and timeout_ms (integer)".to_string()
            )
        })?;
        let timeout_ms = match timeout_val {
            Value::Integer(n) => n,
            _ => return Err(vm.create_error(
                "async_run_timeout: timeout_ms must be an integer".to_string()
            )),
        };
        return vm.async_run_timeout(func, timeout_ms);
    }

    // Task 20.2: Handle await_future (single future resolve) before stdlib
    if name == "await_future" {
        let fut = args.first().cloned().ok_or_else(|| {
            vm.create_error("await_future requires 1 argument (a Future)".to_string())
        })?;
        return match fut {
            Value::Future(handle) => handle.resolve().map_err(RuntimeError::new),
            other => Ok(other), // pass-through for non-futures
        };
    }

    // Task 15.1: Handle nursery_spawn before stdlib — needs VM access to spawn threads
    if name == "nursery_spawn" {
        let func = args.first().cloned().ok_or_else(|| {
            vm.create_error("nursery_spawn requires 1 argument (a function)".to_string())
        })?;
        vm.spawn_for_nursery(func)?;
        return Ok(Value::Null);
    }

    // Handle capability functions directly (before stdlib)
    if name == "grant_capability"
        || name == "use_capability"
        || name == "revoke_capability"
        || name == "capability_valid"
    {
        match vm.handle_capability_function(name, args)? {
            Some(result) => return Ok(result),
            None => {
                // If VM doesn't handle it, try CapabilityLib directly (will fail but consistent)
                return Err(
                    vm.create_error(format!("Capability function '{}' not available", name))
                );
            }
        }
    }

    // Try to call standard library function
    match vm.call_stdlib_function(name, args) {
        Ok(result) => return Ok(result),
        Err(e) => {
            // Check if it's an "Unknown function" error - if so, continue to check structs/user functions
            // Otherwise, return the error immediately (e.g., permission errors, argument errors, etc.)
            let error_msg = e.to_string();
            if error_msg.contains("Unknown standard library function")
                || error_msg.contains("Unknown function")
                || error_msg.contains("Unknown test function")
                || error_msg.contains("Unknown networking function")
                || error_msg.contains("Unknown I/O function")
                || error_msg.contains("Unknown crypto function")
            {
                // Function not found in stdlib - continue to check structs and user functions
                if vm.debug() || vm.verbose() {
                    eprintln!("[DEBUG] StdLib::call_function('{}') - function not found in stdlib, checking structs/user functions", name);
                }
            } else {
                // Other error (permission, argument, DNS, etc.) - return it immediately
                if vm.debug() || vm.verbose() {
                    eprintln!("[DEBUG] StdLib::call_function('{}') failed: {}", name, e);
                }
                return Err(e);
            }
        }
    }

    // Check if it's a struct instantiation
    if let Some(fields) = vm.struct_defs().get(name) {
        if args.len() != fields.len() {
            return Err(vm.create_error(format!(
                "Struct '{}' requires {} arguments, got {}",
                name,
                fields.len(),
                args.len()
            )));
        }

        let mut struct_fields = HashMap::new();
        for (i, (field_name, _field_type)) in fields.iter().enumerate() {
            struct_fields.insert(field_name.clone(), args[i].clone());
        }

        log_debug(&format!(
            "Instantiating struct '{}' with {} fields",
            name,
            struct_fields.len()
        ));
        let struct_val = Value::Struct(name.to_string(), struct_fields);
        vm.gc_register_allocation(&struct_val);
        return Ok(struct_val);
    }

    // Try user-defined function
    if let Some(Value::Function(_, params, body, captured_env)) = vm.get_variable(name) {
        // If the function was declared `async`, spawn a thread and return a Future.
        if let Some(future_result) = vm.maybe_spawn_async(name, params.clone(), body.clone(), captured_env.clone(), args.to_vec()) {
            return future_result;
        }
        return call_user_function(vm, name, &params, &body, &captured_env, args, expr);
    }

    // Check per-VM native registry: variable holds "__native_fn::<name>" sentinel
    if let Some(Value::String(sentinel)) = vm.get_variable(name) {
        if let Some(fn_name) = sentinel.strip_prefix("__native_fn::") {
            if let Some(result) = vm.call_native_fn(fn_name, args) {
                return Ok(result);
            }
            return Err(vm.create_error(format!(
                "Native function '{}' registered but call failed",
                name
            )));
        }
    }

    // Try method call: "obj.method" pattern
    if let Some(dot_pos) = name.find('.') {
        let obj_name = &name[..dot_pos];
        let method_name = &name[dot_pos + 1..];
        if let Some(obj_val) = vm.get_variable(obj_name) {
            // Enum constructor: Color.Red(value) — build enum with payload
            if let Value::Enum(_, _, _) = &obj_val {
                // obj_name is the enum type; look for variant = method_name in enum_defs
                if vm.enum_defs().get(obj_name).and_then(|variants| {
                    variants.iter().find(|(v, _)| v == method_name)
                }).is_some() {
                    let payload = args.first().cloned();
                    let result = Value::Enum(
                        obj_name.to_string(),
                        method_name.to_string(),
                        payload.map(Box::new),
                    );
                    vm.gc_register_allocation(&result);
                    return Ok(result);
                }
            }
            return call_method(vm, obj_val, method_name, args, obj_name);
        }
        // Enum constructor via enum type name (not a variable holding an enum value)
        // e.g. Shape.Circle(5.0) where Shape is an enum type registered in enum_defs
        if let Some(variants) = vm.enum_defs().get(obj_name) {
            let variant_exists = variants.iter().any(|(v, _)| v == method_name);
            if variant_exists {
                let payload = args.first().cloned();
                let result = Value::Enum(
                    obj_name.to_string(),
                    method_name.to_string(),
                    payload.map(Box::new),
                );
                vm.gc_register_allocation(&result);
                return Ok(result);
            }
        }
    }

    Err(vm.create_error(format!("Undefined function: {}", name)))
}

/// Public entry point for MethodCall AST nodes (object can be any evaluated Value)
pub fn call_method_on_value<VM: ExpressionVM>(
    vm: &mut VM,
    obj: Value,
    method: &str,
    args: &[Value],
) -> Result<Value, RuntimeError> {
    call_method(vm, obj, method, args, "<expr>")
}

fn call_method<VM: ExpressionVM>(
    vm: &mut VM,
    obj: Value,
    method: &str,
    args: &[Value],
    obj_name: &str,
) -> Result<Value, RuntimeError> {
    // Struct method dispatch: look up impl methods registered for this struct type
    if let Value::Struct(ref type_name, _) = obj {
        if let Some(func_val) = vm.lookup_struct_method(type_name, method) {
            // Call the method with `self` (the struct) prepended to args
            let mut full_args = vec![obj.clone()];
            full_args.extend_from_slice(args);
            if let Value::Function(_, params, body, captured_env) = func_val {
                return call_user_function(vm, method, &params, &body, &captured_env, &full_args, &Expression::Identifier(method.to_string()));
            }
        }
    }
    match &obj {
        Value::String(s) => call_string_method(s, method, args, obj_name),
        Value::Array(arr) => call_array_method(arr, method, args, obj_name),
        Value::Map(map) => call_map_method(map, method, args, obj_name),
        Value::Set(set) => call_set_method(set, method, args, obj_name),
        // Q.3: For structs, check if the missing method is required by a declared protocol.
        Value::Struct(type_name, _) => {
            let implements_key = format!("__implements_{}", type_name);
            if let Some(Value::Array(protocols)) = vm.get_variable(&implements_key) {
                for proto_val in &protocols {
                    if let Value::String(proto_name) = proto_val {
                        let proto_key = format!("__protocol_{}", proto_name);
                        if let Some(Value::Array(proto_methods)) = vm.get_variable(&proto_key) {
                            // Protocol methods are stored as Value::Map { "name": ..., ... }
                            let has_method = proto_methods.iter().any(|m| {
                                match m {
                                    Value::String(mn) => mn.as_ref() == method,
                                    Value::Map(map) => map.get("name")
                                        .and_then(|v| if let Value::String(mn) = v { Some(mn) } else { None })
                                        .map(|mn| mn.as_ref() == method)
                                        .unwrap_or(false),
                                    _ => false,
                                }
                            });
                            if has_method {
                                return Err(RuntimeError::new(format!(
                                    "struct '{}' declares 'implements {}' but is missing required method '{}'",
                                    type_name, proto_name, method
                                )).with_code(crate::runtime::errors::ErrorCode::E0029));
                            }
                        }
                    }
                }
            }
            Err(RuntimeError::new(format!(
                "Type '{}' has no method '{}'",
                type_name, method
            )))
        }
        _ => Err(RuntimeError::new(format!(
            "Type '{}' has no method '{}'",
            obj.type_name(), method
        ))),
    }
}

fn call_string_method(
    s: &str,
    method: &str,
    args: &[Value],
    _obj_name: &str,
) -> Result<Value, RuntimeError> {
    match method {
        "toLower" | "toLowerCase" => Ok(Value::String(Arc::from(s.to_lowercase()))),
        "toUpper" | "toUpperCase" => Ok(Value::String(Arc::from(s.to_uppercase()))),
        "trim" => Ok(Value::String(Arc::from(s.trim().to_string()))),
        "trimStart" | "trimLeft" => Ok(Value::String(Arc::from(s.trim_start().to_string()))),
        "trimEnd" | "trimRight" => Ok(Value::String(Arc::from(s.trim_end().to_string()))),
        "len" | "length" => Ok(Value::Integer(s.chars().count() as i64)),
        "reverse" => Ok(Value::String(Arc::from(s.chars().rev().collect::<String>()))),
        "chars" | "toChars" => Ok(Value::Array(
            s.chars().map(|c| Value::String(Arc::from(c.to_string()))).collect(),
        )),
        "toInt" | "parseInt" => s
            .trim()
            .parse::<i64>()
            .map(Value::Integer)
            .map_err(|_| RuntimeError::new(format!("Cannot convert '{}' to integer", s))),
        "toFloat" | "parseFloat" => s
            .trim()
            .parse::<f64>()
            .map(Value::Float)
            .map_err(|_| RuntimeError::new(format!("Cannot convert '{}' to float", s))),
        "isEmpty" => Ok(Value::Boolean(s.is_empty())),
        "split" => {
            let sep = args
                .first()
                .and_then(|v| match v {
                    Value::String(sep) => Some(sep.as_ref()),
                    _ => None,
                })
                .unwrap_or(" ");
            let parts: Vec<Value> = s.split(sep).map(|p| Value::String(Arc::from(p.to_string()))).collect();
            Ok(Value::Array(parts))
        }
        "startsWith" => {
            let prefix = args
                .first()
                .and_then(|v| match v {
                    Value::String(p) => Some(p.as_ref()),
                    _ => None,
                })
                .unwrap_or("");
            Ok(Value::Boolean(s.starts_with(prefix)))
        }
        "endsWith" => {
            let suffix = args
                .first()
                .and_then(|v| match v {
                    Value::String(p) => Some(p.as_ref()),
                    _ => None,
                })
                .unwrap_or("");
            Ok(Value::Boolean(s.ends_with(suffix)))
        }
        "contains" | "includes" => {
            let needle = args
                .first()
                .and_then(|v| match v {
                    Value::String(n) => Some(n.as_ref()),
                    _ => None,
                })
                .unwrap_or("");
            Ok(Value::Boolean(s.contains(needle)))
        }
        "indexOf" => {
            let needle = args
                .first()
                .and_then(|v| match v {
                    Value::String(n) => Some(n.clone()),
                    _ => None,
                })
                .unwrap_or_default();
            Ok(Value::Integer(
                s.find(needle.as_ref()).map(|i| i as i64).unwrap_or(-1),
            ))
        }
        "replace" => {
            let from = args
                .first()
                .and_then(|v| match v {
                    Value::String(f) => Some(f.clone()),
                    _ => None,
                })
                .unwrap_or_default();
            let to = args
                .get(1)
                .and_then(|v| match v {
                    Value::String(t) => Some(t.clone()),
                    _ => None,
                })
                .unwrap_or_default();
            Ok(Value::String(Arc::from(s.replace(from.as_ref(), to.as_ref()))))
        }
        "substring" | "slice" => {
            let start = args
                .first()
                .and_then(|v| match v {
                    Value::Integer(i) => Some(*i as usize),
                    _ => None,
                })
                .unwrap_or(0);
            let end = args
                .get(1)
                .and_then(|v| match v {
                    Value::Integer(i) => Some(*i as usize),
                    _ => None,
                })
                .unwrap_or(s.len());
            let chars: Vec<char> = s.chars().collect();
            let end = end.min(chars.len());
            Ok(Value::String(Arc::from(chars[start..end].iter().collect::<String>())))
        }
        "repeat" => {
            let n = args
                .first()
                .and_then(|v| match v {
                    Value::Integer(i) => Some(*i as usize),
                    _ => None,
                })
                .unwrap_or(1);
            Ok(Value::String(Arc::from(s.repeat(n))))
        }
        "padStart" => {
            let len = args
                .first()
                .and_then(|v| match v {
                    Value::Integer(i) => Some(*i as usize),
                    _ => None,
                })
                .unwrap_or(0);
            let pad = args
                .get(1)
                .and_then(|v| match v {
                    Value::String(p) => Some(p.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| Arc::from(" "));
            if s.len() >= len {
                Ok(Value::String(Arc::from(s.to_string())))
            } else {
                let needed = len - s.len();
                let pad_str: String = pad.chars().cycle().take(needed).collect();
                Ok(Value::String(Arc::from(format!("{}{}", pad_str, s))))
            }
        }
        "padEnd" => {
            let len = args
                .first()
                .and_then(|v| match v {
                    Value::Integer(i) => Some(*i as usize),
                    _ => None,
                })
                .unwrap_or(0);
            let pad = args
                .get(1)
                .and_then(|v| match v {
                    Value::String(p) => Some(p.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| Arc::from(" "));
            if s.len() >= len {
                Ok(Value::String(Arc::from(s.to_string())))
            } else {
                let needed = len - s.len();
                let pad_str: String = pad.chars().cycle().take(needed).collect();
                Ok(Value::String(Arc::from(format!("{}{}", s, pad_str))))
            }
        }
        "center" => {
            let width = args
                .first()
                .and_then(|v| match v {
                    Value::Integer(i) => Some(*i as usize),
                    _ => None,
                })
                .unwrap_or(0);
            let pad_ch = args
                .get(1)
                .and_then(|v| match v {
                    Value::String(p) => p.chars().next(),
                    _ => None,
                })
                .unwrap_or(' ');
            let current_len = s.chars().count();
            if current_len >= width {
                Ok(Value::String(Arc::from(s.to_string())))
            } else {
                let total_pad = width - current_len;
                let left_pad = total_pad / 2;
                let right_pad = total_pad - left_pad;
                Ok(Value::String(Arc::from(format!(
                    "{}{}{}",
                    std::iter::repeat_n(pad_ch, left_pad).collect::<String>(),
                    s,
                    std::iter::repeat_n(pad_ch, right_pad).collect::<String>()
                ))))
            }
        }
        _ => Err(RuntimeError::new(format!(
            "String has no method '{}'",
            method
        ))),
    }
}

fn call_array_method(
    arr: &[Value],
    method: &str,
    args: &[Value],
    _obj_name: &str,
) -> Result<Value, RuntimeError> {
    match method {
        "len" | "length" => Ok(Value::Integer(arr.len() as i64)),
        "isEmpty" => Ok(Value::Boolean(arr.is_empty())),
        "first" => arr
            .first()
            .cloned()
            .ok_or_else(|| RuntimeError::new("Array is empty".to_string())),
        "last" => arr
            .last()
            .cloned()
            .ok_or_else(|| RuntimeError::new("Array is empty".to_string())),
        "reverse" => {
            let mut v = arr.to_vec();
            v.reverse();
            Ok(Value::Array(v))
        }
        "sort" => {
            let mut v = arr.to_vec();
            v.sort_by(|a, b| match (a, b) {
                (Value::Integer(x), Value::Integer(y)) => x.cmp(y),
                (Value::Float(x), Value::Float(y)) => {
                    x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal)
                }
                (Value::String(x), Value::String(y)) => x.cmp(y),
                _ => std::cmp::Ordering::Equal,
            });
            Ok(Value::Array(v))
        }
        "contains" | "includes" => {
            let needle = args.first().cloned().unwrap_or(Value::Null);
            Ok(Value::Boolean(arr.contains(&needle)))
        }
        "indexOf" => {
            let needle = args.first().cloned().unwrap_or(Value::Null);
            Ok(Value::Integer(
                arr.iter()
                    .position(|v| v == &needle)
                    .map(|i| i as i64)
                    .unwrap_or(-1),
            ))
        }
        "join" => {
            let sep = args
                .first()
                .and_then(|v| match v {
                    Value::String(s) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_default();
            let joined: Vec<String> = arr.iter().map(|v| v.to_string()).collect();
            Ok(Value::String(Arc::from(joined.join(&sep))))
        }
        "slice" => {
            let start = args
                .first()
                .and_then(|v| match v {
                    Value::Integer(i) => Some(*i as usize),
                    _ => None,
                })
                .unwrap_or(0);
            let end = args
                .get(1)
                .and_then(|v| match v {
                    Value::Integer(i) => Some(*i as usize),
                    _ => None,
                })
                .unwrap_or(arr.len());
            let end = end.min(arr.len());
            Ok(Value::Array(arr[start..end].to_vec()))
        }
        "push" => {
            let mut v = arr.to_vec();
            if let Some(val) = args.first() {
                v.push(val.clone());
            }
            Ok(Value::Array(v))
        }
        "pop" => {
            let mut v = arr.to_vec();
            v.pop();
            Ok(Value::Array(v))
        }
        "flat" | "flatten" => {
            let mut result = Vec::new();
            for item in arr {
                match item {
                    Value::Array(inner) => result.extend(inner.iter().cloned()),
                    other => result.push(other.clone()),
                }
            }
            Ok(Value::Array(result))
        }
        "sum" => {
            let mut int_sum: i64 = 0;
            let mut has_float = false;
            let mut float_sum: f64 = 0.0;
            for item in arr {
                match item {
                    Value::Integer(n) => {
                        int_sum = int_sum.checked_add(*n).unwrap_or(i64::MAX);
                        float_sum += *n as f64;
                    }
                    Value::Float(f) => {
                        has_float = true;
                        float_sum += f;
                    }
                    _ => {}
                }
            }
            if has_float {
                Ok(Value::Float(float_sum))
            } else {
                Ok(Value::Integer(int_sum))
            }
        }
        "enumerate" => {
            let result: Vec<Value> = arr
                .iter()
                .enumerate()
                .map(|(i, v)| Value::Array(vec![Value::Integer(i as i64), v.clone()]))
                .collect();
            Ok(Value::Array(result))
        }
        "zip" => {
            let other = match args.first() {
                Some(Value::Array(a)) => a.clone(),
                _ => return Err(RuntimeError::new("zip requires an array argument".to_string())),
            };
            let result: Vec<Value> = arr
                .iter()
                .zip(other.iter())
                .map(|(a, b)| Value::Array(vec![a.clone(), b.clone()]))
                .collect();
            Ok(Value::Array(result))
        }
        "head" => Ok(arr.first().cloned().unwrap_or(Value::Null)),
        "tail" => Ok(Value::Array(arr.get(1..).unwrap_or(&[]).to_vec())),
        _ => Err(RuntimeError::new(format!(
            "Array has no method '{}'",
            method
        ))),
    }
}

fn call_map_method(
    map: &indexmap::IndexMap<String, Value>,
    method: &str,
    args: &[Value],
    _obj_name: &str,
) -> Result<Value, RuntimeError> {
    match method {
        "len" | "length" | "size" => Ok(Value::Integer(map.len() as i64)),
        "isEmpty" => Ok(Value::Boolean(map.is_empty())),
        "keys" => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            Ok(Value::Array(keys.into_iter().map(|k| Value::String(Arc::from(k.clone()))).collect()))
        }
        "values" => {
            let mut pairs: Vec<(&String, &Value)> = map.iter().collect();
            pairs.sort_by_key(|(k, _)| k.as_str());
            Ok(Value::Array(pairs.into_iter().map(|(_, v)| v.clone()).collect()))
        }
        "entries" | "items" => {
            let mut pairs: Vec<(&String, &Value)> = map.iter().collect();
            pairs.sort_by_key(|(k, _)| k.as_str());
            let entries: Vec<Value> = pairs
                .into_iter()
                .map(|(k, v)| Value::Array(vec![Value::String(Arc::from(k.clone())), v.clone()]))
                .collect();
            Ok(Value::Array(entries))
        }
        "has" | "contains" | "containsKey" => {
            let key = args
                .first()
                .and_then(|v| match v {
                    Value::String(k) => Some(k.as_ref()),
                    _ => None,
                })
                .unwrap_or("");
            Ok(Value::Boolean(map.contains_key(key)))
        }
        "get" => {
            let key = args
                .first()
                .and_then(|v| match v {
                    Value::String(k) => Some(k.clone()),
                    _ => None,
                })
                .unwrap_or_default();
            Ok(map.get(key.as_ref()).cloned().unwrap_or(Value::Null))
        }
        _ => Err(RuntimeError::new(format!("Map has no method '{}'", method))),
    }
}

fn call_set_method(
    set: &[Value],
    method: &str,
    args: &[Value],
    _obj_name: &str,
) -> Result<Value, RuntimeError> {
    match method {
        "len" | "length" | "size" => Ok(Value::Integer(set.len() as i64)),
        "isEmpty" => Ok(Value::Boolean(set.is_empty())),
        "values" | "toArray" => Ok(Value::Array(set.to_vec())),
        "has" | "contains" => {
            let needle = args.first().cloned().unwrap_or(Value::Null);
            Ok(Value::Boolean(set.contains(&needle)))
        }
        _ => Err(RuntimeError::new(format!("Set has no method '{}'", method))),
    }
}

/// Task E.5 / N.4 — Detect if a function body ends with a self-tail-recursive call.
/// Handles:
/// - `return → fn_name(...)` as the last statement
/// - bare `fn_name(...)` as the last statement (N.4)
/// - if/else where every branch ends with a tail call to fn_name (N.4)
fn detect_self_tail_recursive(fn_name: &str, body: &[Statement]) -> bool {
    if body.is_empty() { return false; }
    stmt_is_tail_call(fn_name, body.last().unwrap())
}

fn stmt_is_tail_call(fn_name: &str, stmt: &Statement) -> bool {
    match stmt {
        // return → fn_name(...)
        Statement::Return { value: Some(expr), .. } => matches!(
            expr,
            Expression::FunctionCall { name, .. } if name == fn_name
        ),
        // bare fn_name(...)  (N.4)
        Statement::Expression(Expression::FunctionCall { name, .. }) if name == fn_name => true,
        // if/else: all branches must end with a tail call  (N.4)
        Statement::If { then_branch, else_if_branches, else_branch, .. } => {
            let then_ok = then_branch.last().is_some_and(|s| stmt_is_tail_call(fn_name, s));
            let else_ok = else_branch.as_ref()
                .and_then(|b| b.last())
                .is_some_and(|s| stmt_is_tail_call(fn_name, s));
            let elif_ok = else_if_branches.iter().all(|(_, b)| {
                b.last().is_some_and(|s| stmt_is_tail_call(fn_name, s))
            });
            // All branches must be present and end with a tail call.
            // An if without an else is NOT guaranteed to tail-call.
            then_ok && else_ok && (else_if_branches.is_empty() || elif_ok)
        }
        _ => false,
    }
}

/// Task E.5 — Bind a list of evaluated args to function params in the current scope.
/// Extracted from the stacker closure for use in the TCO loop.
fn bind_params_to_scope<VM: ExpressionVM>(
    vm: &mut VM,
    fn_name: &str,
    params: &[crate::parser::ast::Parameter],
    args: &[Value],
) -> Result<(), RuntimeError> {
    let mut arg_index = 0;
    let args_len = args.len();
    for param in params {
        if param.is_variadic {
            let remaining: Vec<Value> = args[arg_index..].to_vec();
            vm.define_local_variable(param.name.clone(), Value::Array(remaining))?;
            arg_index = args_len;
        } else if arg_index < args_len {
            let arg = &args[arg_index];
            if let Some(expected_type) = &param.type_annotation {
                if !matches!(arg, Value::Null)
                    && !crate::runtime::execution::statements::param_type_matches(arg, expected_type)
                {
                    return Err(RuntimeError::new(format!(
                        "type mismatch: parameter '{}' of '{}' expects '{}' but got '{}'",
                        param.name, fn_name,
                        crate::runtime::execution::statements::type_annotation_display(expected_type),
                        crate::runtime::execution::statements::value_type_display(arg),
                    )).with_code(crate::runtime::errors::ErrorCode::E0011));
                }
            }
            vm.define_local_variable(param.name.clone(), arg.clone())?;
            arg_index += 1;
        } else if let Some(default_expr) = &param.default_value {
            let default_val = super::ExpressionEvaluator::evaluate(vm, default_expr)?;
            vm.define_local_variable(param.name.clone(), default_val)?;
            arg_index += 1;
        } else {
            return Err(vm.create_error(format!("Missing required parameter: {}", param.name)));
        }
    }
    Ok(())
}

pub fn call_user_function<VM: ExpressionVM>(
    vm: &mut VM,
    name: &str,
    params: &[crate::parser::ast::Parameter],
    body: &[Statement],
    captured_env: &HashMap<String, Value>,
    args: &[Value],
    expr: &Expression,
) -> Result<Value, RuntimeError> {
    let params = params.to_vec();
    let body = body.to_vec();
    let captured_env = captured_env.clone();

    log_debug(&format!(
        "Calling function '{}' with {} arguments, function has {} parameters",
        name,
        args.len(),
        params.len()
    ));

    // Guard against infinite recursion before pushing the next frame.
    // Kept at 50 in debug mode — larger enums use more Rust stack per frame.
    const MAX_CALL_DEPTH: usize = crate::runtime::errors::MAX_CALL_DEPTH;
    if vm.call_stack_depth() >= MAX_CALL_DEPTH {
        return Err(RuntimeError::new(format!(
            "Maximum call stack depth ({}) exceeded — possible infinite recursion in '{}'",
            MAX_CALL_DEPTH, name
        )));
    }

    // Push call frame
    let span = match expr {
        Expression::FunctionCall { span, .. } => span,
        _ => &Span::default(),
    };
    vm.call_stack_push(CallFrame {
        function_name: name.to_string(),
        line: span.line,
        column: span.column,
    });

    // Push captured environment as a scope (for closures) - BEFORE parameters.
    // Use define_local_variable (not set_variable) so captured values land in the
    // newly-pushed scope rather than updating a same-named variable in an outer scope.
    // This ensures mutations inside the closure body are isolated from the caller's scope.
    if !captured_env.is_empty() {
        vm.push_scope();
        for (var_name, var_value) in &captured_env {
            vm.define_local_variable(var_name.clone(), var_value.clone())?;
        }
    }

    // Task E.5 — Tail-Call Optimization (TCO)
    // If the last statement in the body is `return → <same_fn>(...)`, we can
    // reuse the current call frame instead of recursing. This turns O(n) stack
    // usage into O(1) for tail-recursive functions.
    //
    // Safety: enforce a TCO iteration limit so that infinitely-recursive
    // tail-recursive functions still produce an error rather than looping forever.
    // 100 K iterations is unreachable in real programs but stops true infinite loops quickly.
    const TCO_LIMIT: usize = 100_000;
    if detect_self_tail_recursive(name, &body) {
        let mut current_args = args.to_vec();
        let mut tco_iter: usize = 0;
        let result = 'tco: loop {
            tco_iter += 1;
            if tco_iter > TCO_LIMIT {
                break 'tco Err(RuntimeError::new(format!(
                    "Maximum call depth ({}) exceeded — possible infinite recursion (tail-recursive) in '{}'",
                    TCO_LIMIT, name
                )));
            }
            vm.push_scope(); // params scope — popped at end of each iteration

            // Bind current iteration's args to params
            if let Err(e) = bind_params_to_scope(vm, name, &params, &current_args) {
                vm.pop_scope();
                break 'tco Err(e);
            }

            // Execute all statements except the last (tail call)
            let body_len = body.len();
            let mut early_result: Option<Result<Value, RuntimeError>> = None;
            for stmt in &body[..body_len.saturating_sub(1)] {
                if let Statement::Return { value, .. } = stmt {
                    let v = if let Some(e) = value {
                        match super::ExpressionEvaluator::evaluate(vm, e) {
                            Ok(v) => v,
                            Err(e) => { early_result = Some(Err(e)); break; }
                        }
                    } else { Value::Null };
                    early_result = Some(Ok(v));
                    break;
                }
                match vm.execute_statement(stmt) {
                    Ok(_) => {}
                    Err(e) => match e.take_return_value() {
                        Ok(v) => { early_result = Some(Ok(v)); break; }
                        Err(other) => {
                            // O.1: break/continue must NOT escape function boundaries.
                            let err = if other.is_break_signal() || other.is_continue_signal() {
                                RuntimeError::new("break/continue cannot escape a function boundary".to_string())
                                    .with_code(crate::runtime::errors::ErrorCode::E0040)
                            } else {
                                other
                            };
                            early_result = Some(Err(err));
                            break;
                        }
                    }
                }
            }

            if let Some(r) = early_result {
                vm.pop_scope();
                break 'tco r;
            }

            // Evaluate new args for the tail call (last statement).
            // N.4: Also handles bare `fn_name(args)` as the last statement.
            let new_args_result = match body.last() {
                Some(Statement::Return { value: Some(Expression::FunctionCall { arguments, .. }), .. })
                | Some(Statement::Expression(Expression::FunctionCall { arguments, .. })) => arguments
                    .iter()
                    .map(|a| super::ExpressionEvaluator::evaluate(vm, a))
                    .collect::<Result<Vec<_>, _>>(),
                _ => Ok(vec![]),
            };

            vm.pop_scope(); // pop params scope before rebinding
            match new_args_result {
                Ok(new_args) => { current_args = new_args; }
                Err(e) => break 'tco Err(e),
            }
            // continue 'tco with updated args
        };

        if !captured_env.is_empty() { vm.pop_scope(); }
        vm.call_stack_pop();
        return result;
    }

    // Non-TCO path: push scope then use stacker
    // Push new scope for function parameters
    vm.push_scope();

    // Use stacker::maybe_grow so that deep recursion transparently extends the
    // Rust thread stack rather than overflowing it. When remaining stack space
    // falls below 128 KB, stacker spawns a helper thread with an 8 MB stack
    // segment and runs the continuation there — transparent to callers.
    //
    // Use a closure to ensure cleanup on early return
    let result = stacker::maybe_grow(128 * 1024, 8 * 1024 * 1024, || (|| -> Result<Value, RuntimeError> {
        // Bind arguments with variadic support
        let mut arg_index = 0;
        let args_len = args.len();

        for param in &params {
            if param.is_variadic {
                let remaining_args: Vec<Value> = args[arg_index..].to_vec();
                log_debug(&format!(
                    "Binding variadic parameter '{}' with {} arguments",
                    param.name,
                    remaining_args.len()
                ));
                vm.define_local_variable(param.name.clone(), Value::Array(remaining_args))?;
                arg_index = args_len;
            } else if arg_index < args_len {
                let arg = &args[arg_index];
                log_debug(&format!("Binding parameter '{}' = {:?}", param.name, arg));
                // Runtime type enforcement for typed parameters
                if let Some(expected_type) = &param.type_annotation {
                    if !matches!(arg, Value::Null) && !crate::runtime::execution::statements::param_type_matches(arg, expected_type) {
                        return Err(RuntimeError::new(format!(
                            "type mismatch: parameter '{}' of '{}' expects '{}' but got '{}'",
                            param.name,
                            name,
                            crate::runtime::execution::statements::type_annotation_display(expected_type),
                            crate::runtime::execution::statements::value_type_display(arg),
                        )).with_code(crate::runtime::errors::ErrorCode::E0011));
                    }
                }
                vm.define_local_variable(param.name.clone(), arg.clone())?;
                arg_index += 1;
            } else if let Some(default_expr) = &param.default_value {
                log_debug(&format!(
                    "Using default value for parameter '{}'",
                    param.name
                ));
                let default_val = super::ExpressionEvaluator::evaluate(vm, default_expr)?;
                vm.define_local_variable(param.name.clone(), default_val)?;
                arg_index += 1;
            } else {
                return Err(vm.create_error(format!("Missing required parameter: {}", param.name)));
            }
        }

        if arg_index < args_len {
            log_warn(&format!(
                "Extra arguments provided to function '{}' ({} unused)",
                name,
                args_len - arg_index
            ));
        }

        // ── Generator function detection (Task 14.5) ─────────────────────
        // If the body contains any `yield →` statements, run in generator mode.
        // Yields are collected via a thread-local, allowing nested yields in loops.
        let is_generator = body.iter().any(stmt_contains_yield);
        if is_generator {
            use crate::runtime::execution::statements::GENERATOR_COLLECTOR;
            // Install the collector
            GENERATOR_COLLECTOR.with(|c| *c.borrow_mut() = Some(Vec::new()));
            for stmt in &body {
                match vm.execute_statement(stmt) {
                    Ok(_) => {}
                    Err(e) => match e.take_return_value() {
                        Ok(_) => break, // `return` ends the generator
                        Err(other) => {
                            GENERATOR_COLLECTOR.with(|c| *c.borrow_mut() = None);
                            // O.1: break/continue must NOT escape function boundaries.
                            let err = if other.is_break_signal() || other.is_continue_signal() {
                                RuntimeError::new("break/continue cannot escape a function boundary".to_string())
                                    .with_code(crate::runtime::errors::ErrorCode::E0040)
                            } else {
                                other
                            };
                            return Err(err);
                        }
                    },
                }
            }
            // Collect and remove
            let yielded = GENERATOR_COLLECTOR.with(|c| c.borrow_mut().take().unwrap_or_default());
            return Ok(Value::Array(yielded));
        }
        // ─────────────────────────────────────────────────────────────────

        let mut result = Value::Null;
        for stmt in &body {
            // Fast path: direct top-level return (avoids extra stack frames per recursion level)
            if let Statement::Return { value, .. } = stmt {
                result = if let Some(expr) = value {
                    super::ExpressionEvaluator::evaluate(vm, expr)?
                } else {
                    Value::Null
                };
                break;
            }
            // For all other statements, handle ReturnValue signals from nested control flow
            // (e.g., `return` inside `if`, `for`, `while`, `match`, `try`)
            match vm.execute_statement(stmt) {
                Ok(_) => {}
                Err(e) => match e.take_return_value() {
                    Ok(v) => {
                        result = v;
                        break;
                    }
                    Err(other) => {
                        // O.1: break/continue must NOT escape function boundaries.
                        if other.is_break_signal() || other.is_continue_signal() {
                            return Err(RuntimeError::new(
                                "break/continue cannot escape a function boundary".to_string(),
                            ).with_code(crate::runtime::errors::ErrorCode::E0040));
                        }
                        return Err(other);
                    }
                },
            }
        }

        Ok(result)
    })());

    // Always clean up scope and call frame, even on error
    vm.pop_scope(); // params scope
    if !captured_env.is_empty() {
        vm.pop_scope(); // captured env scope
    }
    vm.call_stack_pop();

    result
}

/// Task 14.5: Return true if a statement is or contains a `yield →` statement.
/// Used to detect generator functions at call time.
fn stmt_contains_yield(stmt: &Statement) -> bool {
    match stmt {
        Statement::Yield { .. } => true,
        Statement::If { then_branch, else_if_branches, else_branch, .. } => {
            then_branch.iter().any(stmt_contains_yield)
                || else_if_branches.iter().any(|(_, stmts)| stmts.iter().any(stmt_contains_yield))
                || else_branch.as_ref().is_some_and(|b| b.iter().any(stmt_contains_yield))
        }
        Statement::While { body, .. }
        | Statement::For { body, .. }
        | Statement::DoWhile { body, .. }
        | Statement::Repeat { body, .. } => body.iter().any(stmt_contains_yield),
        Statement::Match { cases, default, .. } => {
            cases.iter().any(|(_, _, stmts)| stmts.iter().any(stmt_contains_yield))
                || default.as_ref().is_some_and(|b| b.iter().any(stmt_contains_yield))
        }
        _ => false,
    }
}
