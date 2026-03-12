use crate::parser::ast::BinaryOperator;
use crate::runtime::core::Value;
use crate::runtime::errors::RuntimeError;

/// Comparison operators: ==, !=, <, >, <=, >=
pub struct ComparisonOps;

impl ComparisonOps {
    pub fn apply(op: &BinaryOperator, left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match op {
            BinaryOperator::Equal => Ok(Value::Boolean(Self::values_equal(left, right))),
            BinaryOperator::NotEqual => Ok(Value::Boolean(!Self::values_equal(left, right))),
            BinaryOperator::Less => Self::less(left, right),
            BinaryOperator::Greater => Self::greater(left, right),
            BinaryOperator::LessEqual => Self::less_equal(left, right),
            BinaryOperator::GreaterEqual => Self::greater_equal(left, right),
            _ => Err(RuntimeError::new(format!(
                "Not a comparison operator: {:?}",
                op
            ))),
        }
    }

    pub fn values_equal(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Integer(i1), Value::Integer(i2)) => i1 == i2,
            (Value::Float(f1), Value::Float(f2)) => (f1 - f2).abs() < f64::EPSILON,
            (Value::String(s1), Value::String(s2)) => s1 == s2,
            (Value::Char(c1), Value::Char(c2)) => c1 == c2,
            (Value::Boolean(b1), Value::Boolean(b2)) => b1 == b2,
            (Value::Null, Value::Null) => true,
            _ => false,
        }
    }

    fn less(left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a < b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Boolean(a < b)),
            (Value::Integer(a), Value::Float(b)) => Ok(Value::Boolean((*a as f64) < *b)),
            (Value::Float(a), Value::Integer(b)) => Ok(Value::Boolean(*a < (*b as f64))),
            (Value::String(a), Value::String(b)) => Ok(Value::Boolean(a < b)),
            (Value::Char(a), Value::Char(b)) => Ok(Value::Boolean(a < b)),
            _ => Err(RuntimeError::new(
                "Invalid operands for comparison".to_string(),
            )),
        }
    }

    fn greater(left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a > b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Boolean(a > b)),
            (Value::Integer(a), Value::Float(b)) => Ok(Value::Boolean((*a as f64) > *b)),
            (Value::Float(a), Value::Integer(b)) => Ok(Value::Boolean(*a > (*b as f64))),
            (Value::String(a), Value::String(b)) => Ok(Value::Boolean(a > b)),
            (Value::Char(a), Value::Char(b)) => Ok(Value::Boolean(a > b)),
            _ => Err(RuntimeError::new(
                "Invalid operands for comparison".to_string(),
            )),
        }
    }

    fn less_equal(left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a <= b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Boolean(a <= b)),
            (Value::Integer(a), Value::Float(b)) => Ok(Value::Boolean((*a as f64) <= *b)),
            (Value::Float(a), Value::Integer(b)) => Ok(Value::Boolean(*a <= (*b as f64))),
            (Value::String(a), Value::String(b)) => Ok(Value::Boolean(a <= b)),
            (Value::Char(a), Value::Char(b)) => Ok(Value::Boolean(a <= b)),
            _ => Err(RuntimeError::new(
                "Invalid operands for comparison".to_string(),
            )),
        }
    }

    fn greater_equal(left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a >= b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Boolean(a >= b)),
            (Value::Integer(a), Value::Float(b)) => Ok(Value::Boolean((*a as f64) >= *b)),
            (Value::Float(a), Value::Integer(b)) => Ok(Value::Boolean(*a >= (*b as f64))),
            (Value::String(a), Value::String(b)) => Ok(Value::Boolean(a >= b)),
            (Value::Char(a), Value::Char(b)) => Ok(Value::Boolean(a >= b)),
            _ => Err(RuntimeError::new(
                "Invalid operands for comparison".to_string(),
            )),
        }
    }
}
