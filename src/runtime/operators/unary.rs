use crate::parser::ast::UnaryOperator;
use crate::runtime::core::Value;
use crate::runtime::errors::RuntimeError;
use crate::runtime::operators::logical::LogicalOps;

/// Unary operators: not, -, ~
pub struct UnaryOps;

impl UnaryOps {
    pub fn apply(op: &UnaryOperator, val: &Value) -> Result<Value, RuntimeError> {
        match op {
            UnaryOperator::Not => Ok(Value::Boolean(!LogicalOps::is_truthy(val))),
            UnaryOperator::Minus => Self::minus(val),
            UnaryOperator::BitNot => Self::bit_not(val),
            UnaryOperator::Increment => Self::increment(val),
            UnaryOperator::Decrement => Self::decrement(val),
        }
    }

    fn minus(val: &Value) -> Result<Value, RuntimeError> {
        match val {
            Value::Integer(i) => Ok(Value::Integer(-i)),
            Value::Float(f) => Ok(Value::Float(-f)),
            _ => Err(RuntimeError::new("Invalid operand for negation".to_string())),
        }
    }

    fn bit_not(val: &Value) -> Result<Value, RuntimeError> {
        match val {
            Value::Integer(i) => Ok(Value::Integer(!i)),
            _ => Err(RuntimeError::new("Bitwise not requires an integer".to_string())),
        }
    }

    fn increment(val: &Value) -> Result<Value, RuntimeError> {
        match val {
            Value::Integer(i) => Ok(Value::Integer(i + 1)),
            Value::Float(f) => Ok(Value::Float(f + 1.0)),
            _ => Err(RuntimeError::new("Increment requires a number".to_string())),
        }
    }

    fn decrement(val: &Value) -> Result<Value, RuntimeError> {
        match val {
            Value::Integer(i) => Ok(Value::Integer(i - 1)),
            Value::Float(f) => Ok(Value::Float(f - 1.0)),
            _ => Err(RuntimeError::new("Decrement requires a number".to_string())),
        }
    }
}

