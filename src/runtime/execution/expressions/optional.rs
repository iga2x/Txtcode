// Optional chaining evaluation (?., ?(), ?[])

use super::ExpressionVM;
use crate::parser::ast::Expression;
use crate::runtime::core::Value;
use crate::runtime::errors::RuntimeError;

pub fn evaluate_optional_member<VM: ExpressionVM>(
    vm: &mut VM,
    target: &Expression,
    name: &str,
) -> Result<Value, RuntimeError> {
    let target_val = super::ExpressionEvaluator::evaluate(vm, target)?;
    if matches!(target_val, Value::Null) {
        Ok(Value::Null)
    } else {
        match target_val {
            Value::Map(map) => Ok(map.get(name).cloned().unwrap_or(Value::Null)),
            Value::Struct(_struct_name, fields) => {
                Ok(fields.get(name).cloned().unwrap_or(Value::Null))
            }
            Value::Enum(_enum_name, _variant, _) => Ok(Value::Null),
            _ => Ok(Value::Null),
        }
    }
}

pub fn evaluate_optional_call<VM: ExpressionVM>(
    vm: &mut VM,
    target: &Expression,
    arguments: &[Expression],
) -> Result<Value, RuntimeError> {
    let target_val = super::ExpressionEvaluator::evaluate(vm, target)?;
    if matches!(target_val, Value::Null) {
        Ok(Value::Null)
    } else {
        let function_name = match target {
            Expression::Identifier(name) => name.clone(),
            Expression::Member {
                target: obj, name, ..
            } => {
                if let Expression::Identifier(obj_name) = obj.as_ref() {
                    format!("{}.{}", obj_name, name)
                } else {
                    return Ok(Value::Null);
                }
            }
            _ => return Ok(Value::Null),
        };

        let args: Vec<Value> = arguments
            .iter()
            .map(|arg| super::ExpressionEvaluator::evaluate(vm, arg))
            .collect::<Result<_, _>>()?;

        // Try capability functions
        if function_name == "grant_capability"
            || function_name == "use_capability"
            || function_name == "revoke_capability"
            || function_name == "capability_valid"
        {
            match vm.handle_capability_function(&function_name, &args)? {
                Some(result) => return Ok(result),
                None => {
                    // If VM doesn't handle it, return null (optional chaining)
                    return Ok(Value::Null);
                }
            }
        }

        // Try stdlib
        match vm.call_stdlib_function(&function_name, &args) {
            Ok(result) => Ok(result),
            Err(_) => {
                // Try user-defined function (simplified version for optional call)
                if let Some(Value::Function(_, _params, _body, _captured_env)) =
                    vm.get_variable(&function_name)
                {
                    // Reuse logic from evaluate_function_call but simplified
                    // For now, return null if function call fails
                    Ok(Value::Null)
                } else {
                    Ok(Value::Null)
                }
            }
        }
    }
}

pub fn evaluate_optional_index<VM: ExpressionVM>(
    vm: &mut VM,
    target: &Expression,
    index: &Expression,
) -> Result<Value, RuntimeError> {
    let target_val = super::ExpressionEvaluator::evaluate(vm, target)?;
    if matches!(target_val, Value::Null) {
        Ok(Value::Null)
    } else {
        let idx = super::ExpressionEvaluator::evaluate(vm, index)?;
        match (target_val, idx) {
            (Value::Array(arr), Value::Integer(i)) => {
                Ok(arr.get(i as usize).cloned().unwrap_or(Value::Null))
            }
            (Value::Map(map), Value::String(key)) => {
                Ok(map.get(&key).cloned().unwrap_or(Value::Null))
            }
            _ => Ok(Value::Null),
        }
    }
}
