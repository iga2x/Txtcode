// Binary and unary operator evaluation

use crate::parser::ast::{Expression, BinaryOperator, UnaryOperator};
use crate::runtime::core::Value;
use crate::runtime::errors::RuntimeError;
use crate::runtime::operators::OperatorRegistry;
use super::ExpressionVM;

pub fn evaluate_binary_op<VM: ExpressionVM>(
    vm: &mut VM,
    left: &Box<Expression>,
    op: &BinaryOperator,
    right: &Box<Expression>,
) -> Result<Value, RuntimeError> {
    // Handle pipe operator specially: left |> right_func
    // right_func must evaluate to a callable (Function or String lambda name).
    if matches!(op, BinaryOperator::Pipe) {
        let left_val = super::ExpressionEvaluator::evaluate(vm, left)?;
        let right_val = super::ExpressionEvaluator::evaluate(vm, right)?;
        return pipe_call(vm, left_val, right_val, right.as_ref());
    }
    let left_val = super::ExpressionEvaluator::evaluate(vm, left)?;
    let right_val = super::ExpressionEvaluator::evaluate(vm, right)?;
    OperatorRegistry::apply_binary(op, &left_val, &right_val)
}

fn pipe_call<VM: ExpressionVM>(
    vm: &mut VM,
    arg: Value,
    func: Value,
    func_expr: &Expression,
) -> Result<Value, RuntimeError> {
    use crate::runtime::execution::expressions::function_calls::call_user_function;
    match func {
        Value::Function(ref name, ref params, ref body, ref captured_env) => {
            call_user_function(vm, name, params, body, captured_env, &[arg], func_expr)
        }
        Value::String(ref func_name) => {
            // Lambda/function stored as name string — look it up in scope
            if let Some(Value::Function(ref n, ref params, ref body, ref env)) =
                vm.get_variable(func_name.as_str())
            {
                let (n, params, body, env) = (n.clone(), params.clone(), body.clone(), env.clone());
                call_user_function(vm, &n, &params, &body, &env, &[arg], func_expr)
            } else {
                Err(RuntimeError::new(format!(
                    "Pipe operator |>: '{}' is not a callable function",
                    func_name
                )))
            }
        }
        other => Err(RuntimeError::new(format!(
            "Pipe operator |> requires a callable on the right side, got: {}",
            other.to_string()
        ))),
    }
}

pub fn evaluate_unary_op<VM: ExpressionVM>(
    vm: &mut VM,
    op: &UnaryOperator,
    operand: &Box<Expression>,
) -> Result<Value, RuntimeError> {
    let val = super::ExpressionEvaluator::evaluate(vm, operand)?;
    let result = OperatorRegistry::apply_unary(op, &val)?;
    if matches!(op, UnaryOperator::Increment | UnaryOperator::Decrement) {
        if let Expression::Identifier(name) = operand.as_ref() {
            vm.set_variable(name.clone(), result.clone())?;
        }
    }
    Ok(result)
}

