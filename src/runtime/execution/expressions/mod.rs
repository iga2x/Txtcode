// Expression evaluation module - handles all expression types
// Modular structure for better maintainability

use crate::parser::ast::{Expression, Statement, Literal};
use crate::runtime::core::Value;
use crate::runtime::errors::RuntimeError;
use crate::runtime::operators::OperatorRegistry;
use crate::typecheck::types::Type;
use crate::runtime::permissions::PermissionResource;
use std::collections::{HashMap, HashSet};

pub use function_calls::{evaluate_function_call, call_user_function};
pub use member_access::evaluate_member;
pub use lambdas::evaluate_lambda;
pub use collections::{evaluate_array, evaluate_map, evaluate_set, evaluate_slice};
pub use operators::{evaluate_binary_op, evaluate_unary_op};
pub use optional::{evaluate_optional_member, evaluate_optional_call, evaluate_optional_index};

mod function_calls;
mod member_access;
mod lambdas;
mod collections;
mod operators;
mod optional;

/// Trait for VM methods needed by expression evaluation
pub trait ExpressionVM {
    fn get_variable(&self, name: &str) -> Option<Value>;
    fn set_variable(&mut self, name: String, value: Value) -> Result<(), RuntimeError>;
    fn push_scope(&mut self);
    fn pop_scope(&mut self);
    fn create_error(&self, message: String) -> RuntimeError;
    fn check_permission_with_audit(&mut self, resource: &PermissionResource, scope: Option<&str>) -> Result<(), RuntimeError>;
    fn check_rate_limit(&mut self, action: &str) -> Result<(), RuntimeError>;
    fn map_stdlib_to_action(&self, name: &str, args: &[Value]) -> Option<(String, String)>;
    fn check_intent(&self, function_name: &str, action: &str, resource: &str) -> Result<(), RuntimeError>;
    fn call_stack_current_frame(&self) -> Option<&crate::runtime::core::CallFrame>;
    fn call_stack_depth(&self) -> usize;
    fn call_stack_push(&mut self, frame: crate::runtime::core::CallFrame);
    fn call_stack_pop(&mut self);
    fn audit_trail_log_action(&mut self, action: String, resource: String, context: Option<String>, result: crate::runtime::audit::AuditResult, ai_metadata: Option<&crate::runtime::audit::AIMetadata>);
    fn ai_metadata(&self) -> &crate::runtime::audit::AIMetadata;
    fn struct_defs(&self) -> &HashMap<String, Vec<(String, Type)>>;
    fn enum_defs(&self) -> &HashMap<String, Vec<(String, Option<Expression>)>>;
    fn gc_register_allocation(&mut self, value: &Value);
    fn debug(&self) -> bool;
    fn verbose(&self) -> bool;
    fn exec_allowed(&self) -> Option<bool>;
    fn execute_statement(&mut self, stmt: &Statement) -> Result<Value, RuntimeError>;
    fn extract_free_variables(body: &Expression, param_names: &HashSet<String>) -> HashSet<String>;
    fn capture_environment(&self, var_names: &HashSet<String>) -> HashMap<String, Value>;
    
    // Capability functions need a CapabilityExecutor
    // This is handled in the VM implementation since we can't pass trait objects here
    fn handle_capability_function(&mut self, name: &str, args: &[Value]) -> Result<Option<Value>, RuntimeError>;
    
    // StdLib calls need a FunctionExecutor
    fn call_stdlib_function(&mut self, name: &str, args: &[Value]) -> Result<Value, RuntimeError>;
}

/// Expression evaluator - handles all expression types
pub struct ExpressionEvaluator;

impl ExpressionEvaluator {
    /// Evaluate an expression using the provided VM context
    pub fn evaluate<VM: ExpressionVM>(vm: &mut VM, expr: &Expression) -> Result<Value, RuntimeError> {
        match expr {
            Expression::Literal(lit) => {
                Ok(match lit {
                    Literal::Integer(i) => Value::Integer(*i),
                    Literal::Float(f) => Value::Float(*f),
                    Literal::String(s) => Value::String(s.clone()),
                    Literal::Char(c) => Value::Char(*c),
                    Literal::Boolean(b) => Value::Boolean(*b),
                    Literal::Null => Value::Null,
                })
            }
            Expression::Identifier(name) => {
                // Check if it's a variable
                if let Some(value) = vm.get_variable(name) {
                    Ok(value)
                } else {
                    // Not a variable - could be an enum type (handled in Member expression)
                    Err(vm.create_error(format!("Undefined variable: {}", name)))
                }
            }
            Expression::BinaryOp { left, op, right, .. } => {
                evaluate_binary_op(vm, left, op, right)
            }
            Expression::UnaryOp { op, operand, .. } => {
                evaluate_unary_op(vm, op, operand)
            }
            Expression::FunctionCall { name, arguments, .. } => {
                evaluate_function_call(vm, name, arguments, expr)
            }
            Expression::Index { target, index, .. } => {
                let obj = Self::evaluate(vm, target)?;
                let idx = Self::evaluate(vm, index)?;
                match (obj, idx) {
                    (Value::Array(arr), Value::Integer(i)) => {
                        arr.get(i as usize)
                            .cloned()
                            .ok_or_else(|| RuntimeError::new("Index out of bounds".to_string()))
                    }
                    (Value::Map(map), Value::String(key)) => {
                        map.get(&key)
                            .cloned()
                            .ok_or_else(|| RuntimeError::new(format!("Key not found: {}", key)))
                    }
                    _ => Err(RuntimeError::new("Invalid index operation".to_string())),
                }
            }
            Expression::Array { elements, .. } => {
                evaluate_array(vm, elements)
            }
            Expression::Map { entries, .. } => {
                evaluate_map(vm, entries)
            }
            Expression::Set { elements, .. } => {
                evaluate_set(vm, elements)
            }
            Expression::Member { target, name, .. } => {
                evaluate_member(vm, target, name)
            }
            Expression::Lambda { params, body, .. } => {
                evaluate_lambda(vm, params, body)
            }
            Expression::Ternary { condition, true_expr, false_expr, .. } => {
                let cond = Self::evaluate(vm, condition)?;
                if OperatorRegistry::is_truthy(&cond) {
                    Self::evaluate(vm, true_expr)
                } else {
                    Self::evaluate(vm, false_expr)
                }
            }
            Expression::Await { expression, .. } => {
                // Evaluate the expression (should be a Future or async function call)
                let future_value = Self::evaluate(vm, expression)?;
                // For now, just return the value (async support will be added later)
                Ok(future_value)
            }
            Expression::InterpolatedString { segments, .. } => {
                use crate::parser::ast::InterpolatedSegment;
                let mut result = String::new();
                for segment in segments {
                    match segment {
                        InterpolatedSegment::Text(s) => {
                            result.push_str(&s);
                        }
                        InterpolatedSegment::Expression(expr) => {
                            // Evaluate expression and convert to string
                            let val = Self::evaluate(vm, &expr)?;
                            result.push_str(&val.to_string());
                        }
                    }
                }
                Ok(Value::String(result))
            }
            Expression::Slice { target, start, end, step, .. } => {
                evaluate_slice(vm, target, start, end, step)
            }
            Expression::OptionalMember { target, name, .. } => {
                evaluate_optional_member(vm, target, name)
            }
            Expression::OptionalCall { target, arguments, .. } => {
                evaluate_optional_call(vm, target, arguments)
            }
            Expression::OptionalIndex { target, index, .. } => {
                evaluate_optional_index(vm, target, index)
            }
            Expression::MethodCall { object, method, arguments, .. } => {
                // Evaluate the object expression, then dispatch the method
                let obj_val = Self::evaluate(vm, object)?;
                let args: Vec<Value> = arguments.iter()
                    .map(|a| Self::evaluate(vm, a))
                    .collect::<Result<_, _>>()?;
                function_calls::call_method_on_value(vm, obj_val, method, &args)
            }
            Expression::StructLiteral { name, fields, .. } => {
                // Look up struct definition
                let struct_def = vm.struct_defs().get(name).cloned();
                let mut field_map = HashMap::new();
                for (field_name, field_expr) in fields {
                    let val = Self::evaluate(vm, field_expr)?;
                    field_map.insert(field_name.clone(), val);
                }
                // Validate fields if struct def exists
                if let Some(def) = &struct_def {
                    for (def_field, _) in def {
                        if !field_map.contains_key(def_field) {
                            field_map.insert(def_field.clone(), Value::Null);
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

