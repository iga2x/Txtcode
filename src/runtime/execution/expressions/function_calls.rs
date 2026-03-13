// Function call evaluation (stdlib, user functions, struct instantiation)

use super::ExpressionVM;
use crate::parser::ast::{Expression, Span, Statement};
use crate::runtime::core::{CallFrame, Value};
use crate::runtime::errors::RuntimeError;
use crate::runtime::permissions::PermissionResource;
use crate::tools::logger::{log_debug, log_warn};
use std::collections::HashMap;

pub fn evaluate_function_call<VM: ExpressionVM>(
    vm: &mut VM,
    name: &str,
    arguments: &[Expression],
    expr: &Expression,
) -> Result<Value, RuntimeError> {
    let args: Vec<Value> = arguments
        .iter()
        .map(|arg| super::ExpressionEvaluator::evaluate(vm, arg))
        .collect::<Result<_, _>>()?;

    // Single canonical permission gate: intent → capability → rate-limit → permission → audit
    // Every privileged stdlib call passes through check_permission_with_audit, which internally
    // enforces the full pipeline. Scope (path/hostname/cmd) is extracted here so the audit log
    // and capability scope-matching receive the real resource value.

    // Filesystem reads
    if name == "read_file"
        || name == "file_exists"
        || name == "is_file"
        || name == "is_dir"
        || name == "list_dir"
    {
        if let Some(path) = args.first().and_then(|v| match v {
            Value::String(p) => Some(p.as_str()),
            _ => None,
        }) {
            vm.check_rate_limit(name)?;
            vm.check_permission_with_audit(
                &PermissionResource::FileSystem("read".to_string()),
                Some(path),
            )?;
        }
    }

    // Filesystem writes / deletes
    if name == "write_file"
        || name == "append_file"
        || name == "copy_file"
        || name == "move_file"
        || name == "temp_file"
        || name == "watch_file"
        || name == "symlink_create"
        || name == "mkdir"
        || name == "delete"
        || name == "rmdir"
    {
        if let Some(path) = args.first().and_then(|v| match v {
            Value::String(p) => Some(p.as_str()),
            _ => None,
        }) {
            vm.check_rate_limit(name)?;
            let action = if name == "delete" || name == "rmdir" {
                "delete"
            } else {
                "write"
            };
            vm.check_permission_with_audit(
                &PermissionResource::FileSystem(action.to_string()),
                Some(path),
            )?;
        }
    }

    // Network connections
    if name == "http_get"
        || name == "http_post"
        || name == "tcp_connect"
        || name == "udp_send"
        || name == "resolve"
    {
        let hostname_opt = args.first().and_then(|v| match v {
            Value::String(url) => Some(
                url.split("//")
                    .nth(1)
                    .and_then(|s| s.split('/').next())
                    .and_then(|s| s.split(':').next())
                    .unwrap_or(url.as_str()),
            ),
            _ => None,
        });
        if let Some(hostname) = hostname_opt {
            vm.check_rate_limit(name)?;
            if !hostname.is_empty() {
                vm.check_permission_with_audit(
                    &PermissionResource::Network("connect".to_string()),
                    Some(hostname),
                )?;
            }
        }
    }

    // Process execution
    if name == "exec" || name == "spawn" || name == "pipe_exec" {
        let cmd_opt = args.first().and_then(|v| match v {
            Value::String(cmd) => cmd.split_whitespace().next().map(|s| s.to_string()),
            _ => None,
        });
        if let Some(cmd) = cmd_opt {
            vm.check_rate_limit(name)?;
            vm.check_permission_with_audit(
                &PermissionResource::System("exec".to_string()),
                Some(&cmd),
            )?;
        }
    }

    // System environment access
    if name == "getenv" || name == "setenv" {
        vm.check_rate_limit(name)?;
        vm.check_permission_with_audit(&PermissionResource::System("env".to_string()), None)?;
    }

    // Handle capability functions directly (before stdlib)
    if name == "grant_capability"
        || name == "use_capability"
        || name == "revoke_capability"
        || name == "capability_valid"
    {
        match vm.handle_capability_function(name, &args)? {
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
    match vm.call_stdlib_function(name, &args) {
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
        return call_user_function(vm, name, &params, &body, &captured_env, &args, expr);
    }

    // Try method call: "obj.method" pattern
    if let Some(dot_pos) = name.find('.') {
        let obj_name = &name[..dot_pos];
        let method_name = &name[dot_pos + 1..];
        if let Some(obj_val) = vm.get_variable(obj_name) {
            return call_method(vm, obj_val, method_name, &args, obj_name);
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
    _vm: &mut VM,
    obj: Value,
    method: &str,
    args: &[Value],
    obj_name: &str,
) -> Result<Value, RuntimeError> {
    match &obj {
        Value::String(s) => call_string_method(s, method, args, obj_name),
        Value::Array(arr) => call_array_method(arr, method, args, obj_name),
        Value::Map(map) => call_map_method(map, method, args, obj_name),
        Value::Set(set) => call_set_method(set, method, args, obj_name),
        _ => Err(RuntimeError::new(format!(
            "Type {:?} has no method '{}'",
            obj, method
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
        "toLower" | "toLowerCase" => Ok(Value::String(s.to_lowercase())),
        "toUpper" | "toUpperCase" => Ok(Value::String(s.to_uppercase())),
        "trim" => Ok(Value::String(s.trim().to_string())),
        "trimStart" | "trimLeft" => Ok(Value::String(s.trim_start().to_string())),
        "trimEnd" | "trimRight" => Ok(Value::String(s.trim_end().to_string())),
        "len" | "length" => Ok(Value::Integer(s.chars().count() as i64)),
        "reverse" => Ok(Value::String(s.chars().rev().collect())),
        "chars" | "toChars" => Ok(Value::Array(
            s.chars().map(|c| Value::String(c.to_string())).collect(),
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
                    Value::String(sep) => Some(sep.as_str()),
                    _ => None,
                })
                .unwrap_or(" ");
            let parts: Vec<Value> = s.split(sep).map(|p| Value::String(p.to_string())).collect();
            Ok(Value::Array(parts))
        }
        "startsWith" => {
            let prefix = args
                .first()
                .and_then(|v| match v {
                    Value::String(p) => Some(p.as_str()),
                    _ => None,
                })
                .unwrap_or("");
            Ok(Value::Boolean(s.starts_with(prefix)))
        }
        "endsWith" => {
            let suffix = args
                .first()
                .and_then(|v| match v {
                    Value::String(p) => Some(p.as_str()),
                    _ => None,
                })
                .unwrap_or("");
            Ok(Value::Boolean(s.ends_with(suffix)))
        }
        "contains" | "includes" => {
            let needle = args
                .first()
                .and_then(|v| match v {
                    Value::String(n) => Some(n.as_str()),
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
                s.find(needle.as_str()).map(|i| i as i64).unwrap_or(-1),
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
            Ok(Value::String(s.replace(from.as_str(), to.as_str())))
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
            Ok(Value::String(chars[start..end].iter().collect()))
        }
        "repeat" => {
            let n = args
                .first()
                .and_then(|v| match v {
                    Value::Integer(i) => Some(*i as usize),
                    _ => None,
                })
                .unwrap_or(1);
            Ok(Value::String(s.repeat(n)))
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
                .unwrap_or_else(|| " ".to_string());
            if s.len() >= len {
                Ok(Value::String(s.to_string()))
            } else {
                let needed = len - s.len();
                let pad_str: String = pad.chars().cycle().take(needed).collect();
                Ok(Value::String(format!("{}{}", pad_str, s)))
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
                .unwrap_or_else(|| " ".to_string());
            if s.len() >= len {
                Ok(Value::String(s.to_string()))
            } else {
                let needed = len - s.len();
                let pad_str: String = pad.chars().cycle().take(needed).collect();
                Ok(Value::String(format!("{}{}", s, pad_str)))
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
            Ok(Value::String(joined.join(&sep)))
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
        _ => Err(RuntimeError::new(format!(
            "Array has no method '{}'",
            method
        ))),
    }
}

fn call_map_method(
    map: &std::collections::HashMap<String, Value>,
    method: &str,
    args: &[Value],
    _obj_name: &str,
) -> Result<Value, RuntimeError> {
    match method {
        "len" | "length" | "size" => Ok(Value::Integer(map.len() as i64)),
        "isEmpty" => Ok(Value::Boolean(map.is_empty())),
        "keys" => {
            let keys: Vec<Value> = map.keys().map(|k| Value::String(k.clone())).collect();
            Ok(Value::Array(keys))
        }
        "values" => {
            let vals: Vec<Value> = map.values().cloned().collect();
            Ok(Value::Array(vals))
        }
        "entries" | "items" => {
            let entries: Vec<Value> = map
                .iter()
                .map(|(k, v)| Value::Array(vec![Value::String(k.clone()), v.clone()]))
                .collect();
            Ok(Value::Array(entries))
        }
        "has" | "contains" | "containsKey" => {
            let key = args
                .first()
                .and_then(|v| match v {
                    Value::String(k) => Some(k.as_str()),
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
            Ok(map.get(&key).cloned().unwrap_or(Value::Null))
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
    const MAX_CALL_DEPTH: usize = 50;
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

    // Push captured environment as a scope (for closures) - BEFORE parameters
    if !captured_env.is_empty() {
        vm.push_scope();
        for (var_name, var_value) in &captured_env {
            vm.set_variable(var_name.clone(), var_value.clone())?;
        }
    }

    // Push new scope for function parameters
    vm.push_scope();

    // Use a closure to ensure cleanup on early return
    let result = (|| -> Result<Value, RuntimeError> {
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
                    Err(other) => return Err(other),
                },
            }
        }

        Ok(result)
    })();

    // Always clean up scope and call frame, even on error
    vm.pop_scope();
    if !captured_env.is_empty() {
        vm.pop_scope();
    }
    vm.call_stack_pop();

    result
}
