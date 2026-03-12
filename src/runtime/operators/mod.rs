pub mod arithmetic;
pub mod bitwise;
pub mod comparison;
pub mod logical;
pub mod unary;

pub use arithmetic::ArithmeticOps;
pub use bitwise::BitwiseOps;
pub use comparison::ComparisonOps;
pub use logical::LogicalOps;
pub use unary::UnaryOps;

use crate::parser::ast::{BinaryOperator, UnaryOperator};
use crate::runtime::core::Value;
use crate::runtime::errors::RuntimeError;

/// Operator registry that dispatches to appropriate operator module
pub struct OperatorRegistry;

impl OperatorRegistry {
    pub fn apply_binary(
        op: &BinaryOperator,
        left: &Value,
        right: &Value,
    ) -> Result<Value, RuntimeError> {
        match op {
            BinaryOperator::Add
            | BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide
            | BinaryOperator::Modulo
            | BinaryOperator::Power => ArithmeticOps::apply(op, left, right),
            BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::Less
            | BinaryOperator::Greater
            | BinaryOperator::LessEqual
            | BinaryOperator::GreaterEqual => ComparisonOps::apply(op, left, right),
            BinaryOperator::And | BinaryOperator::Or => LogicalOps::apply(op, left, right),
            BinaryOperator::BitwiseAnd
            | BinaryOperator::BitwiseOr
            | BinaryOperator::BitwiseXor
            | BinaryOperator::LeftShift
            | BinaryOperator::RightShift => BitwiseOps::apply(op, left, right),
            BinaryOperator::NullCoalesce => {
                // Null coalesce: left ?? right
                // Returns left if it's not null, otherwise returns right
                if matches!(left, Value::Null) {
                    Ok(right.clone())
                } else {
                    Ok(left.clone())
                }
            }
            BinaryOperator::Pipe => {
                // Pipe: left |> right_func — right must be callable
                // This path is only hit for complex rhs (non-identifier); simple pipes are desugared at parse time.
                Err(RuntimeError::new(
                    "Pipe operator |> with complex right-hand side is not supported. Use a named function.".to_string()
                ))
            }
        }
    }

    pub fn apply_unary(op: &UnaryOperator, val: &Value) -> Result<Value, RuntimeError> {
        UnaryOps::apply(op, val)
    }

    /// Check if a value is truthy (used by logical operators and control flow)
    pub fn is_truthy(val: &Value) -> bool {
        LogicalOps::is_truthy(val)
    }

    /// Check if two values are equal (used by comparison operators)
    pub fn values_equal(a: &Value, b: &Value) -> bool {
        ComparisonOps::values_equal(a, b)
    }
}
