use crate::parser::ast::BinaryOperator;
use crate::runtime::core::Value;
use crate::runtime::errors::RuntimeError;

/// Bitwise operators: &, |, ^, <<, >>
pub struct BitwiseOps;

impl BitwiseOps {
    pub fn apply(op: &BinaryOperator, left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match op {
            BinaryOperator::BitwiseAnd => Self::bitwise_and(left, right),
            BinaryOperator::BitwiseOr => Self::bitwise_or(left, right),
            BinaryOperator::BitwiseXor => Self::bitwise_xor(left, right),
            BinaryOperator::LeftShift => Self::left_shift(left, right),
            BinaryOperator::RightShift => Self::right_shift(left, right),
            _ => Err(RuntimeError::new(format!(
                "Not a bitwise operator: {:?}",
                op
            ))),
        }
    }

    fn bitwise_and(left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a & b)),
            _ => Err(RuntimeError::new(
                "Bitwise AND requires integers".to_string(),
            )),
        }
    }

    fn bitwise_or(left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a | b)),
            _ => Err(RuntimeError::new(
                "Bitwise OR requires integers".to_string(),
            )),
        }
    }

    fn bitwise_xor(left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a ^ b)),
            _ => Err(RuntimeError::new(
                "Bitwise XOR requires integers".to_string(),
            )),
        }
    }

    fn left_shift(left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => {
                if *b < 0 || *b > 63 {
                    Err(RuntimeError::new(
                        "Shift amount must be between 0 and 63".to_string(),
                    ))
                } else {
                    Ok(Value::Integer(a << b))
                }
            }
            _ => Err(RuntimeError::new(
                "Left shift requires integers".to_string(),
            )),
        }
    }

    fn right_shift(left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => {
                if *b < 0 || *b > 63 {
                    Err(RuntimeError::new(
                        "Shift amount must be between 0 and 63".to_string(),
                    ))
                } else {
                    Ok(Value::Integer(a >> b))
                }
            }
            _ => Err(RuntimeError::new(
                "Right shift requires integers".to_string(),
            )),
        }
    }
}
