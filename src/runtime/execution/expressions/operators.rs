// Binary and unary operator evaluation

use crate::parser::ast::Expression;
use crate::runtime::core::Value;
use crate::runtime::errors::RuntimeError;
use crate::runtime::operators::OperatorRegistry;
use super::ExpressionVM;

pub fn evaluate_binary_op<VM: ExpressionVM>(
    vm: &mut VM,
    left: &Box<Expression>,
    op: &crate::parser::ast::BinaryOperator,
    right: &Box<Expression>,
) -> Result<Value, RuntimeError> {
    let left_val = super::ExpressionEvaluator::evaluate(vm, left)?;
    let right_val = super::ExpressionEvaluator::evaluate(vm, right)?;
    OperatorRegistry::apply_binary(op, &left_val, &right_val)
}

pub fn evaluate_unary_op<VM: ExpressionVM>(
    vm: &mut VM,
    op: &crate::parser::ast::UnaryOperator,
    operand: &Box<Expression>,
) -> Result<Value, RuntimeError> {
    let val = super::ExpressionEvaluator::evaluate(vm, operand)?;
    OperatorRegistry::apply_unary(op, &val)
}

