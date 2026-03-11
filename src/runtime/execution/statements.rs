use crate::parser::ast::*;
use crate::runtime::core::Value;
use crate::runtime::errors::RuntimeError;
use crate::runtime::operators::OperatorRegistry;
use crate::tools::logger::log_debug;
use crate::typecheck::types::Type;
use std::collections::HashMap;

/// Statement execution (non-control-flow statements)
pub struct StatementExecutor;

impl StatementExecutor {
    /// Execute a statement
    pub fn execute(vm: &mut impl StatementVM, stmt: &Statement) -> Result<Value, RuntimeError> {
        match stmt {
            Statement::Assignment { pattern, value, .. } => {
                let val = vm.evaluate_expression(value)?;
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
                        map.insert(key, val);
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
                        let mut map = std::collections::HashMap::new();
                        map.insert(key, val);
                        Value::Map(map)
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
            Statement::Break { .. } => Err(RuntimeError::break_signal()),
            Statement::Continue { .. } => Err(RuntimeError::continue_signal()),
            Statement::FunctionDef {
                name,
                type_params: _,
                params,
                body,
                intent,
                ai_hint,
                allowed_actions,
                forbidden_actions,
                ..
            } => {
                // Convert CapabilityExpr to String for intent registration
                let allowed_strings: Vec<String> =
                    allowed_actions.iter().map(|cap| cap.to_string()).collect();
                let forbidden_strings: Vec<String> = forbidden_actions
                    .iter()
                    .map(|cap| cap.to_string())
                    .collect();
                // Regular functions don't capture environment (empty closure)
                // Note: type_params are stored but not used at runtime (type erasure)
                // They're used for type checking only
                vm.set_global(
                    name.clone(),
                    Value::Function(
                        name.clone(),
                        params.clone(),
                        body.clone(),
                        HashMap::new(), // No captured environment for regular functions
                    ),
                )?;

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
            Statement::Struct { name, fields, .. } => {
                // Register struct definition
                log_debug(&format!(
                    "Registering struct '{}' with {} fields",
                    name,
                    fields.len()
                ));
                vm.register_struct(name.clone(), fields.clone());
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

                // Grant permission
                vm.grant_permission(perm_resource, scope.clone());

                Ok(Value::Null)
            }
            Statement::TypeAlias { name, target, .. } => {
                // Register type alias: store as a special variable or just no-op at runtime
                // Type aliases are primarily for static analysis; at runtime we register the name
                vm.set_variable(
                    format!("__type_alias_{}", name),
                    Value::String(target.clone()),
                )?;
                Ok(Value::Null)
            }
            Statement::NamedError { name, message, .. } => {
                // Register named error: evaluate message expression and store
                let msg = vm.evaluate_expression(message)?;
                vm.set_variable(format!("__named_error_{}", name), msg)?;
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
            | Statement::Repeat { .. } => {
                unreachable!("Control flow statements should be handled by ControlFlowExecutor")
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
}
