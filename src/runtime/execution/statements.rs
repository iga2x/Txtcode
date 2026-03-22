use crate::parser::ast::*;
use std::sync::Arc;
use crate::runtime::core::Value;
use crate::runtime::errors::RuntimeError;
use crate::runtime::operators::OperatorRegistry;
use crate::tools::logger::log_debug;
use crate::typecheck::types::Type;
use std::cell::RefCell;
use std::collections::HashMap;

// Task 14.5: Thread-local yield collector for generator functions.
// When Some(vec), yield statements push to the vec instead of raising a signal.
// This allows `yield` inside nested for/if/while to work correctly.
thread_local! {
    pub(crate) static GENERATOR_COLLECTOR: RefCell<Option<Vec<Value>>> = RefCell::new(None);
}

// Task 15.1: Thread-local nursery handle collector for structured concurrency.
// When Some(vec), `nursery_spawn` pushes FutureHandles here instead of returning them.
// The nursery block awaits all collected handles on exit.
thread_local! {
    pub(crate) static NURSERY_HANDLES: RefCell<Option<Vec<crate::runtime::core::value::FutureHandle>>> = RefCell::new(None);
}

pub fn param_type_matches(value: &Value, expected: &Type) -> bool {
    type_matches_value(value, expected)
}

pub fn type_annotation_display(t: &Type) -> String {
    type_annotation_name(t)
}

pub fn value_type_display(value: &Value) -> &'static str {
    value_type_name(value)
}

fn type_matches_value(value: &Value, expected: &Type) -> bool {
    match (value, expected) {
        // K.1: Unknown means no annotation — always accept any value
        (_, Type::Unknown) => true,
        (Value::Integer(_), Type::Int) => true,
        (Value::Integer(_), Type::Float) => true,
        (Value::Float(_), Type::Float) => true,
        (Value::String(_), Type::String) => true,
        (Value::Char(_), Type::Char) => true,
        (Value::Char(_), Type::String) => true,
        (Value::Boolean(_), Type::Bool) => true,
        (Value::Array(_), Type::Array(_)) => true,
        (Value::Map(_), Type::Map(_)) => true,
        // Null is always allowed (nullable semantics)
        (Value::Null, Type::Nullable(_)) | (Value::Null, Type::Null) => true,
        // Non-null value vs Nullable<T>: check inner type
        (v, Type::Nullable(inner)) => type_matches_value(v, inner),
        // Struct/user-defined types and generics — no runtime struct type info, allow
        (_, Type::Identifier(_)) => true,
        (_, Type::Generic(_)) => true,
        _ => false,
    }
}

fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::Integer(_) => "int",
        Value::Float(_) => "float",
        Value::String(_) => "string",
        Value::Char(_) => "char",
        Value::Boolean(_) => "bool",
        Value::Array(_) => "array",
        Value::Map(_) => "map",
        Value::Null => "null",
        Value::Function(_, _, _, _) => "function",
        Value::Result(true, _) => "Ok",
        Value::Result(false, _) => "Err",
        Value::Future(_) => "future",
        _ => "unknown",
    }
}

fn type_annotation_name(t: &Type) -> String {
    match t {
        Type::Int => "int".to_string(),
        Type::Float => "float".to_string(),
        Type::String => "string".to_string(),
        Type::Char => "char".to_string(),
        Type::Bool => "bool".to_string(),
        Type::Array(inner) => format!("array[{}]", type_annotation_name(inner)),
        Type::Map(inner) => format!("map[{}]", type_annotation_name(inner)),
        Type::Set(inner) => format!("set[{}]", type_annotation_name(inner)),
        Type::Null => "null".to_string(),
        Type::Nullable(inner) => format!("{}?", type_annotation_name(inner)),
        Type::Identifier(name) => name.clone(),
        Type::Generic(name) => name.clone(),
        Type::Future(inner) => format!("Future<{}>", type_annotation_name(inner)),
        Type::Function { .. } => "function".to_string(),
        Type::Unknown => "unknown".to_string(),
    }
}

/// Statement execution (non-control-flow statements)
pub struct StatementExecutor;

impl StatementExecutor {
    /// Execute a statement
    pub fn execute(vm: &mut impl StatementVM, stmt: &Statement) -> Result<Value, RuntimeError> {
        match stmt {
            Statement::Assignment { pattern, type_annotation, value, .. } => {
                let val = vm.evaluate_expression(value)?;
                // Runtime type enforcement: if the assignment has a type annotation,
                // validate the actual value matches (skip for Null — always allowed)
                if let Some(expected_type) = type_annotation {
                    if !matches!(val, Value::Null) && !type_matches_value(&val, expected_type) {
                        let var_name = match pattern {
                            Pattern::Identifier(n) => n.as_str(),
                            _ => "<pattern>",
                        };
                        return Err(RuntimeError::new(format!(
                            "type mismatch: variable '{}' declared as '{}' but got '{}'",
                            var_name,
                            type_annotation_name(expected_type),
                            value_type_name(&val),
                        )).with_code(crate::runtime::errors::ErrorCode::E0011));
                    }
                }
                vm.bind_pattern(pattern, &val)?;
                Ok(Value::Null)
            }
            Statement::IndexAssignment {
                target,
                index,
                value,
                ..
            } => {
                let idx = vm.evaluate_expression(index)?;
                let val = vm.evaluate_expression(value)?;
                // Get the object name from the target expression
                let obj_name = match target {
                    Expression::Identifier(name) => name.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            "Index assignment target must be an identifier".to_string(),
                        ))
                    }
                };
                // Get the current object, modify it, store it back
                let obj = vm.get_variable(&obj_name).unwrap_or(Value::Null);
                let updated = match (obj, idx) {
                    (Value::Map(mut map), Value::String(key)) => {
                        map.insert(key.to_string(), val);
                        Value::Map(map)
                    }
                    (Value::Array(mut arr), Value::Integer(i)) => {
                        let i = i as usize;
                        if i < arr.len() {
                            arr[i] = val;
                        } else {
                            return Err(RuntimeError::new(format!(
                                "Array index {} out of bounds (len={})",
                                i,
                                arr.len()
                            )));
                        }
                        Value::Array(arr)
                    }
                    (Value::Null, Value::String(key)) => {
                        // Auto-create map if variable is null
                        let mut map = indexmap::IndexMap::new();
                        map.insert(key.to_string(), val);
                        Value::Map(map)
                    }
                    (Value::Struct(sname, mut fields), Value::String(key)) => {
                        // Struct field assignment: check type if struct def is known.
                        let struct_def = vm.struct_defs().get(&sname).cloned();
                        let strict = vm.strict_types();
                        if let Some(def) = struct_def {
                            match def.iter().find(|(f, _)| f.as_str() == key.as_ref()) {
                                None => {
                                    let known: Vec<&str> = def.iter().map(|(f, _)| f.as_str()).collect();
                                    let msg = format!(
                                        "Struct '{}' has no field '{}'. Known fields: {}",
                                        sname, key, known.join(", ")
                                    );
                                    if strict {
                                        return Err(RuntimeError::new(msg)
                                            .with_code(crate::runtime::errors::ErrorCode::E0016));
                                    }
                                    eprintln!("[WARNING] {}", msg);
                                }
                                Some((_, expected_type)) => {
                                    if !type_matches_value(&val, expected_type) {
                                        let msg = format!(
                                            "Struct field type mismatch: '{}.{}' expected {:?}, got {}",
                                            sname, key, expected_type, val.type_name()
                                        );
                                        if strict {
                                            return Err(RuntimeError::new(msg)
                                                .with_code(crate::runtime::errors::ErrorCode::E0016));
                                        }
                                        eprintln!("[WARNING] {}", msg);
                                    }
                                }
                            }
                        }
                        fields.insert(key.to_string(), val);
                        Value::Struct(sname, fields)
                    }
                    (obj, idx) => {
                        return Err(RuntimeError::new(format!(
                            "Cannot index-assign: {:?}[{:?}]",
                            obj, idx
                        )))
                    }
                };
                vm.set_variable(obj_name, updated)?;
                Ok(Value::Null)
            }
            Statement::CompoundAssignment {
                name, op, value, ..
            } => {
                let current = vm.get_variable(name).unwrap_or(Value::Null);
                let val = vm.evaluate_expression(value)?;
                let result = OperatorRegistry::apply_binary(op, &current, &val)?;
                vm.set_variable(name.clone(), result)?;
                Ok(Value::Null)
            }
            Statement::Expression(expr) => vm.evaluate_expression(expr),
            Statement::Return { value, .. } => {
                let val = if let Some(expr) = value {
                    vm.evaluate_expression(expr)?
                } else {
                    Value::Null
                };
                Err(RuntimeError::return_value(val))
            }
            Statement::Yield { value, .. } => {
                let val = vm.evaluate_expression(value)?;
                // If we're inside a generator (thread-local collector is active), push directly.
                // Otherwise raise a signal (fallback for non-generator usage).
                if GENERATOR_COLLECTOR.with(|c| c.borrow().is_some()) {
                    GENERATOR_COLLECTOR.with(|c| {
                        if let Some(ref mut collector) = *c.borrow_mut() {
                            collector.push(val);
                        }
                    });
                    Ok(Value::Null)
                } else {
                    Err(RuntimeError::yield_value(val))
                }
            }
            Statement::Break { .. } => Err(RuntimeError::break_signal()),
            Statement::Continue { .. } => Err(RuntimeError::continue_signal()),
            Statement::FunctionDef {
                name,
                type_params: _,
                params,
                body,
                is_async,
                intent,
                ai_hint,
                allowed_actions,
                forbidden_actions,
                ..
            } => {
                // Register async functions so the expression evaluator knows to
                // spawn a thread when they are called without `await`.
                if *is_async {
                    vm.register_async_function(name);
                }
                // Convert CapabilityExpr to String for intent registration
                let allowed_strings: Vec<String> =
                    allowed_actions.iter().map(|cap| cap.to_string()).collect();
                let forbidden_strings: Vec<String> = forbidden_actions
                    .iter()
                    .map(|cap| cap.to_string())
                    .collect();
                // W.3: Capture enclosing locals when defined inside a function scope.
                // Top-level functions get an empty captured env (no outer locals to capture).
                let captured_env = if vm.is_in_local_scope() {
                    vm.snapshot_local_vars()
                } else {
                    HashMap::new()
                };

                let func_val = Value::Function(
                    name.clone(),
                    params.clone(),
                    body.clone(),
                    captured_env,
                );

                // W.4: If name is "Type.method", register as a struct method in addition
                // to (or instead of) storing as a variable.
                if let Some(dot_pos) = name.find('.') {
                    let struct_name = &name[..dot_pos];
                    let method_name = name[dot_pos + 1..].to_string();
                    vm.register_struct_method(struct_name, method_name, func_val.clone());
                    // Also store under the full name so direct calls still work.
                    vm.set_global(name.clone(), func_val)?;
                } else if vm.is_in_local_scope() {
                    // Nested non-method function: store in local scope (closure).
                    vm.set_variable(name.clone(), func_val)?;
                } else {
                    // Top-level function: store in globals.
                    vm.set_global(name.clone(), func_val)?;
                }

                // Register intent if declared
                if let Some(intent_str) = intent {
                    let mut declaration =
                        crate::runtime::intent::IntentDeclaration::new(intent_str.clone());
                    if let Some(hint) = ai_hint {
                        declaration = declaration.with_ai_hint(hint.clone());
                    }
                    if !allowed_strings.is_empty() {
                        declaration = declaration.with_allowed_actions(allowed_strings.clone());
                    }
                    if !forbidden_strings.is_empty() {
                        declaration = declaration.with_forbidden_actions(forbidden_strings.clone());
                    }
                    vm.register_function_intent(name.clone(), declaration);
                }

                Ok(Value::Null)
            }
            Statement::Assert {
                condition, message, ..
            } => {
                let cond = vm.evaluate_expression(condition)?;
                if !OperatorRegistry::is_truthy(&cond) {
                    let msg = if let Some(msg_expr) = message {
                        vm.evaluate_expression(msg_expr)?.to_string()
                    } else {
                        "Assertion failed".to_string()
                    };
                    return Err(RuntimeError::new(msg));
                }
                Ok(Value::Null)
            }
            Statement::Enum { name, variants, .. } => {
                // Register enum definition
                log_debug(&format!(
                    "Registering enum '{}' with {} variants",
                    name,
                    variants.len()
                ));
                vm.register_enum(name.clone(), variants.clone());
                Ok(Value::Null)
            }
            Statement::Struct { name, fields, implements, .. } => {
                // Register struct definition
                log_debug(&format!(
                    "Registering struct '{}' with {} fields",
                    name,
                    fields.len()
                ));
                vm.register_struct(name.clone(), fields.clone());
                // Store implements list as __implements_<Name> in scope
                if !implements.is_empty() {
                    let list = Value::Array(
                        implements.iter().map(|p| Value::String(Arc::from(p.clone()))).collect()
                    );
                    let _ = vm.set_variable(format!("__implements_{}", name), list);
                }
                Ok(Value::Null)
            }
            Statement::Protocol { name, methods, .. } => {
                // Store protocol as __protocol_<Name> = [[method_name, [param_types], return]]
                let method_list: Vec<Value> = methods.iter().map(|(mname, params, ret)| {
                    let mut m = indexmap::IndexMap::new();
                    m.insert("name".to_string(), Value::String(Arc::from(mname.clone())));
                    m.insert("params".to_string(), Value::Array(
                        params.iter().map(|p| Value::String(Arc::from(p.clone()))).collect()
                    ));
                    m.insert("return_type".to_string(), ret.as_ref()
                        .map(|r| Value::String(Arc::from(r.clone())))
                        .unwrap_or(Value::Null));
                    Value::Map(m)
                }).collect();
                let _ = vm.set_variable(format!("__protocol_{}", name), Value::Array(method_list));
                Ok(Value::Null)
            }
            Statement::Import {
                modules,
                from,
                alias,
                ..
            } => vm.execute_import(modules, from, alias),
            Statement::Export { names, .. } => vm.execute_export(names),
            Statement::Const { name, value, .. } => {
                let val = vm.evaluate_expression(value)?;
                vm.set_const(name.clone(), val);
                Ok(Value::Null)
            }
            Statement::Permission {
                resource,
                action,
                scope,
                ..
            } => {
                use crate::runtime::permissions::PermissionResource;

                // Convert to PermissionResource
                let perm_resource = match resource.as_str() {
                    "fs" => PermissionResource::FileSystem(action.clone()),
                    "net" => PermissionResource::Network(action.clone()),
                    "sys" => PermissionResource::System(action.clone()),
                    _ => {
                        return Err(RuntimeError::new(format!(
                            "Unknown permission resource: {}. Expected 'fs', 'net', or 'sys'",
                            resource
                        )));
                    }
                };

                // Grant the permission (parser syntax: `permission → fs.read → /tmp/*`
                // produces resource="fs", action="read", scope=Some("/tmp/*")).
                // The action field is the sub-action (read/write/connect/exec/info),
                // not an enforcement mode — there is no deny/require syntax in the grammar.
                vm.grant_permission(perm_resource, scope.clone());

                Ok(Value::Null)
            }
            Statement::TypeAlias { name, target, .. } => {
                // Register type alias: store as a special variable or just no-op at runtime
                // Type aliases are primarily for static analysis; at runtime we register the name
                vm.set_variable(
                    format!("__type_alias_{}", name),
                    Value::String(Arc::from(target.clone())),
                )?;
                Ok(Value::Null)
            }
            Statement::NamedError { name, message, .. } => {
                // Register named error: evaluate message expression and store
                let msg = vm.evaluate_expression(message)?;
                vm.set_variable(format!("__named_error_{}", name), msg)?;
                Ok(Value::Null)
            }
            Statement::Impl { struct_name, methods, .. } => {
                // Execute each method definition, then register it under the struct name.
                for method_stmt in methods {
                    // Execute the FunctionDef so it lands in the current scope,
                    // then retrieve the Value::Function and register it as a method.
                    vm.execute_nested_statement(method_stmt)?;
                    if let Statement::FunctionDef { name, .. } = method_stmt {
                        if let Some(func_val) = vm.get_variable(name) {
                            vm.register_struct_method(struct_name, name.clone(), func_val);
                        }
                    }
                }
                Ok(Value::Null)
            }
            // Control flow statements are handled by ControlFlowExecutor
            // These should not reach here
            Statement::If { .. }
            | Statement::While { .. }
            | Statement::DoWhile { .. }
            | Statement::For { .. }
            | Statement::Match { .. }
            | Statement::Try { .. }
            | Statement::Repeat { .. }
            | Statement::Nursery { .. } => {
                unreachable!("Control flow statements should be handled by ControlFlowExecutor")
            }
            Statement::Error { message, .. } => {
                // Task E.4: Error recovery nodes are silently skipped at runtime.
                // They should never reach execution (the CLI stops on parse errors),
                // but the LSP runs programs with partial ASTs.
                Err(RuntimeError::new(format!("[parse error] {}", message)))
            }
        }
    }
}

/// Trait for VM methods needed by statement execution
pub trait StatementVM {
    fn evaluate_expression(&mut self, expr: &Expression) -> Result<Value, RuntimeError>;
    fn get_variable(&self, name: &str) -> Option<Value>;
    fn set_variable(&mut self, name: String, value: Value) -> Result<(), RuntimeError>;
    fn set_global(&mut self, name: String, value: Value) -> Result<(), RuntimeError>;
    fn bind_pattern(&mut self, pattern: &Pattern, value: &Value) -> Result<(), RuntimeError>;
    fn struct_defs(&self) -> &HashMap<String, Vec<(String, Type)>>;
    fn strict_types(&self) -> bool;
    fn register_enum(&mut self, name: String, variants: Vec<(String, Option<Expression>)>);
    fn register_struct(&mut self, name: String, fields: Vec<(String, Type)>);
    fn execute_import(
        &mut self,
        modules: &[String],
        from: &Option<String>,
        alias: &Option<String>,
    ) -> Result<Value, RuntimeError>;
    fn execute_export(&mut self, names: &[String]) -> Result<Value, RuntimeError>;
    fn set_const(&mut self, name: String, value: Value);
    fn grant_permission(
        &mut self,
        resource: crate::runtime::permissions::PermissionResource,
        scope: Option<String>,
    );
    fn register_function_intent(
        &mut self,
        name: String,
        declaration: crate::runtime::intent::IntentDeclaration,
    );
    /// Mark `name` as an async function so the expression evaluator can spawn a
    /// thread when it is called without `await`.
    fn register_async_function(&mut self, name: &str);
    /// Register a method on a struct type (impl block).
    fn register_struct_method(&mut self, struct_name: &str, method_name: String, func: Value);
    /// Execute a nested statement (used by impl block to define methods in scope).
    fn execute_nested_statement(&mut self, stmt: &Statement) -> Result<Value, RuntimeError>;
    /// W.3: Returns true when currently inside at least one local scope.
    fn is_in_local_scope(&self) -> bool;
    /// W.3: Snapshot all locally-visible variables for closure capture.
    fn snapshot_local_vars(&self) -> HashMap<String, Value>;
}
