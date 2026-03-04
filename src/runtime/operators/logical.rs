use crate::parser::ast::BinaryOperator;
use crate::runtime::core::Value;
use crate::runtime::errors::RuntimeError;

/// Logical operators: and, or
pub struct LogicalOps;

impl LogicalOps {
    pub fn apply(op: &BinaryOperator, left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match op {
            BinaryOperator::And => Ok(Value::Boolean(Self::is_truthy(left) && Self::is_truthy(right))),
            BinaryOperator::Or => Ok(Value::Boolean(Self::is_truthy(left) || Self::is_truthy(right))),
            _ => Err(RuntimeError::new(format!("Not a logical operator: {:?}", op))),
        }
    }

    pub fn is_truthy(val: &Value) -> bool {
        match val {
            Value::Boolean(false) | Value::Null => false,
            Value::Integer(0) | Value::Float(0.0) => false,
            Value::String(s) => !s.is_empty(),
            Value::Array(arr) => !arr.is_empty(),
            Value::Map(map) => !map.is_empty(),
            _ => true,
        }
    }
}

