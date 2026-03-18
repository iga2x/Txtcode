// Expression evaluation module - handles all expression types
// Modular structure for better maintainability

use crate::parser::ast::{Expression, Literal, Statement};
use crate::runtime::core::Value;
use crate::runtime::errors::RuntimeError;
use crate::runtime::operators::OperatorRegistry;
use crate::runtime::permissions::PermissionResource;
use crate::typecheck::types::Type;
use std::collections::{HashMap, HashSet};

pub use collections::{evaluate_array, evaluate_map, evaluate_set, evaluate_slice};
pub use function_calls::{call_user_function, evaluate_function_call};
pub use lambdas::evaluate_lambda;
pub use member_access::evaluate_member;
pub use operators::{evaluate_binary_op, evaluate_unary_op};
pub use optional::{evaluate_optional_call, evaluate_optional_index, evaluate_optional_member};

mod collections;
mod function_calls;
mod lambdas;
mod member_access;
mod operators;
mod optional;

/// Trait for VM methods needed by expression evaluation
pub trait ExpressionVM {
    fn get_variable(&self, name: &str) -> Option<Value>;
    fn set_variable(&mut self, name: String, value: Value) -> Result<(), RuntimeError>;
    /// Bind a variable directly in the current (innermost) scope without searching outer scopes.
    /// Must be used for function parameter binding to prevent a callee's parameter from
    /// overwriting a same-named variable in the caller's scope.
    fn define_local_variable(&mut self, name: String, value: Value) -> Result<(), RuntimeError>;
    fn push_scope(&mut self);
    fn pop_scope(&mut self);
    fn create_error(&self, message: String) -> RuntimeError;
    fn check_permission_with_audit(
        &mut self,
        resource: &PermissionResource,
        scope: Option<&str>,
    ) -> Result<(), RuntimeError>;
    fn check_rate_limit(&mut self, action: &str) -> Result<(), RuntimeError>;
    fn map_stdlib_to_action(&self, name: &str, args: &[Value]) -> Option<(String, String)>;
    fn check_intent(
        &self,
        function_name: &str,
        action: &str,
        resource: &str,
    ) -> Result<(), RuntimeError>;
    fn call_stack_current_frame(&self) -> Option<&crate::runtime::core::CallFrame>;
    fn call_stack_depth(&self) -> usize;
    fn call_stack_push(&mut self, frame: crate::runtime::core::CallFrame);
    fn call_stack_pop(&mut self);
    fn audit_trail_log_action(
        &mut self,
        action: String,
        resource: String,
        context: Option<String>,
        result: crate::runtime::audit::AuditResult,
        ai_metadata: Option<&crate::runtime::audit::AIMetadata>,
    );
    fn ai_metadata(&self) -> &crate::runtime::audit::AIMetadata;
    fn struct_defs(&self) -> &HashMap<String, Vec<(String, Type)>>;
    fn enum_defs(&self) -> &HashMap<String, Vec<(String, Option<Expression>)>>;
    fn gc_register_allocation(&mut self, value: &Value);
    fn debug(&self) -> bool;
    fn verbose(&self) -> bool;
    fn exec_allowed(&self) -> Option<bool>;
    fn strict_types(&self) -> bool;
    fn execute_statement(&mut self, stmt: &Statement) -> Result<Value, RuntimeError>;
    fn extract_free_variables(body: &Expression, param_names: &HashSet<String>) -> HashSet<String>;
    fn capture_environment(&self, var_names: &HashSet<String>) -> HashMap<String, Value>;

    // Capability functions need a CapabilityExecutor
    // This is handled in the VM implementation since we can't pass trait objects here
    fn handle_capability_function(
        &mut self,
        name: &str,
        args: &[Value],
    ) -> Result<Option<Value>, RuntimeError>;

    // StdLib calls need a FunctionExecutor
    fn call_stdlib_function(&mut self, name: &str, args: &[Value]) -> Result<Value, RuntimeError>;

    /// True if the function `name` was declared with the `async` keyword.
    /// Default: false (VM implementations override to provide real async dispatch).
    fn is_async_function(&self, _name: &str) -> bool { false }

    /// Snapshot the global scope for use in a spawned async thread.
    /// Default: empty map (synchronous fallback, no async spawning).
    fn globals_snapshot(&self) -> HashMap<String, Value> { HashMap::new() }

    /// Whether `exec` is allowed in this VM context.
    fn exec_allowed_bool(&self) -> bool { true }

    /// Attempt to spawn `name` as an async task.
    ///
    /// Returns `Some(Ok(Value::Future(…)))` when the function is async and a
    /// thread was spawned.  Returns `None` when the function is synchronous
    /// (caller should fall through to the normal `call_user_function` path).
    ///
    /// The default implementation always returns `None` (no async support).
    /// `VirtualMachine` overrides this to provide real thread-based async.
    fn maybe_spawn_async(
        &mut self,
        _name: &str,
        _params: Vec<crate::parser::ast::Parameter>,
        _body: Vec<Statement>,
        _captured_env: HashMap<String, Value>,
        _args: Vec<Value>,
    ) -> Option<Result<Value, RuntimeError>> {
        None
    }
}

/// Check whether a runtime Value is compatible with the declared Type.
/// Returns true if assignment is valid (same type, or widening conversions).
fn type_matches(value: &Value, expected: &Type) -> bool {
    match (value, expected) {
        (Value::Integer(_), Type::Int) => true,
        (Value::Integer(_), Type::Float) => true, // int widens to float
        (Value::Float(_), Type::Float) => true,
        (Value::String(_), Type::String) => true,
        (Value::Char(_), Type::Char) => true,
        (Value::Char(_), Type::String) => true, // char widens to string
        (Value::Boolean(_), Type::Bool) => true,
        (Value::Array(_), Type::Array(_)) => true,
        (Value::Map(_), Type::Map(_)) => true,
        (Value::Null, _) => true, // null is always accepted (nullable semantics)
        (_, Type::Identifier(_)) => true, // user-defined type — checked by name at struct level
        (_, Type::Generic(_)) => true,    // generic param — unchecked at runtime
        _ => false,
    }
}

/// Expression evaluator - handles all expression types
pub struct ExpressionEvaluator;

impl ExpressionEvaluator {
    /// Evaluate an expression using the provided VM context
    pub fn evaluate<VM: ExpressionVM>(
        vm: &mut VM,
        expr: &Expression,
    ) -> Result<Value, RuntimeError> {
        match expr {
            Expression::Literal(lit) => Ok(match lit {
                Literal::Integer(i) => Value::Integer(*i),
                Literal::Float(f) => Value::Float(*f),
                Literal::String(s) => Value::String(s.clone()),
                Literal::Char(c) => Value::Char(*c),
                Literal::Boolean(b) => Value::Boolean(*b),
                Literal::Null => Value::Null,
            }),
            Expression::Identifier(name) => {
                // Check if it's a variable
                if let Some(value) = vm.get_variable(name) {
                    Ok(value)
                } else {
                    // Not a variable - could be an enum type (handled in Member expression)
                    Err(vm.create_error(format!("Undefined variable: {}", name)))
                }
            }
            Expression::BinaryOp {
                left, op, right, ..
            } => evaluate_binary_op(vm, left.as_ref(), op, right.as_ref()),
            Expression::UnaryOp { op, operand, .. } => evaluate_unary_op(vm, op, operand),
            Expression::FunctionCall {
                name, arguments, ..
            } => evaluate_function_call(vm, name, arguments, expr),
            Expression::Index { target, index, .. } => {
                let obj = Self::evaluate(vm, target)?;
                let idx = Self::evaluate(vm, index)?;
                match (obj, idx) {
                    (Value::Array(arr), Value::Integer(i)) => arr
                        .get(i as usize)
                        .cloned()
                        .ok_or_else(|| RuntimeError::new("Index out of bounds".to_string())),
                    (Value::Map(map), Value::String(key)) => map
                        .get(&key)
                        .cloned()
                        .ok_or_else(|| RuntimeError::new(format!("Key not found: {}", key))),
                    _ => Err(RuntimeError::new("Invalid index operation".to_string())),
                }
            }
            Expression::Array { elements, .. } => evaluate_array(vm, elements),
            Expression::Map { entries, .. } => evaluate_map(vm, entries),
            Expression::Set { elements, .. } => evaluate_set(vm, elements),
            Expression::Member { target, name, .. } => evaluate_member(vm, target, name),
            Expression::Lambda { params, body, .. } => evaluate_lambda(vm, params, body),
            Expression::Ternary {
                condition,
                true_expr,
                false_expr,
                ..
            } => {
                let cond = Self::evaluate(vm, condition)?;
                if OperatorRegistry::is_truthy(&cond) {
                    Self::evaluate(vm, true_expr)
                } else {
                    Self::evaluate(vm, false_expr)
                }
            }
            Expression::Await { expression, .. } => {
                let val = Self::evaluate(vm, expression)?;
                match val {
                    Value::Future(handle) => {
                        // Block until the spawned thread delivers its result.
                        handle.resolve().map_err(|e| vm.create_error(e))
                    }
                    other => {
                        // `await` on a non-future is a transparent no-op —
                        // consistent with JavaScript's `await nonPromise` semantics.
                        Ok(other)
                    }
                }
            }
            Expression::InterpolatedString { segments, .. } => {
                use crate::parser::ast::InterpolatedSegment;
                let mut result = String::new();
                for segment in segments {
                    match segment {
                        InterpolatedSegment::Text(s) => {
                            result.push_str(s);
                        }
                        InterpolatedSegment::Expression(expr) => {
                            // Evaluate expression and convert to string
                            let val = Self::evaluate(vm, expr)?;
                            result.push_str(&val.to_string());
                        }
                    }
                }
                Ok(Value::String(result))
            }
            Expression::Slice {
                target,
                start,
                end,
                step,
                ..
            } => evaluate_slice(vm, target, start, end, step),
            Expression::OptionalMember { target, name, .. } => {
                evaluate_optional_member(vm, target.as_ref(), name)
            }
            Expression::OptionalCall {
                target, arguments, ..
            } => evaluate_optional_call(vm, target.as_ref(), arguments),
            Expression::OptionalIndex { target, index, .. } => {
                evaluate_optional_index(vm, target, index)
            }
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                // Evaluate the object expression, then dispatch the method
                let obj_val = Self::evaluate(vm, object)?;
                let args: Vec<Value> = arguments
                    .iter()
                    .map(|a| Self::evaluate(vm, a))
                    .collect::<Result<_, _>>()?;
                function_calls::call_method_on_value(vm, obj_val, method, &args)
            }
            Expression::StructLiteral { name, fields, .. } => {
                // Look up struct definition
                let struct_def = vm.struct_defs().get(name).cloned();
                let strict = vm.strict_types();
                let mut field_map = HashMap::new();
                for (field_name, field_expr) in fields {
                    let val = Self::evaluate(vm, field_expr)?;
                    field_map.insert(field_name.clone(), val);
                }
                // Validate fields against struct definition
                if let Some(def) = &struct_def {
                    // Fill missing fields with Null; check type of provided fields.
                    for (def_field, expected_type) in def {
                        match field_map.get(def_field) {
                            None => {
                                field_map.insert(def_field.clone(), Value::Null);
                            }
                            Some(val) => {
                                if !type_matches(val, expected_type) {
                                    let msg = format!(
                                        "Struct field type mismatch: '{}.{}' expected {:?}, got {}",
                                        name, def_field, expected_type,
                                        val.type_name()
                                    );
                                    if strict {
                                        return Err(RuntimeError::new(msg)
                                            .with_code(crate::runtime::errors::ErrorCode::E0016));
                                    }
                                    // Advisory (non-strict): surface as a warning but continue
                                    eprintln!("[WARNING] {}", msg);
                                }
                            }
                        }
                    }
                    // Unknown fields — those present in field_map but not in def
                    let def_names: std::collections::HashSet<&String> =
                        def.iter().map(|(n, _)| n).collect();
                    for key in field_map.keys() {
                        if !def_names.contains(key) {
                            let msg = format!(
                                "Struct '{}' has no field '{}'. Known fields: {}",
                                name, key,
                                def.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>().join(", ")
                            );
                            if strict {
                                return Err(RuntimeError::new(msg)
                                    .with_code(crate::runtime::errors::ErrorCode::E0016));
                            }
                            eprintln!("[WARNING] {}", msg);
                        }
                    }
                }
                Ok(Value::Struct(name.clone(), field_map))
            }
            Expression::Spread { value, .. } => {
                // Spread outside an array literal context — evaluate inner value
                Self::evaluate(vm, value)
            }
        }
    }
}
