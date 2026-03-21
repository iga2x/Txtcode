use crate::parser::ast::BinaryOperator;
use crate::runtime::core::Value;
use crate::runtime::errors::RuntimeError;

/// Arithmetic operators: +, -, *, /, %, **
pub struct ArithmeticOps;

impl ArithmeticOps {
    pub fn apply(op: &BinaryOperator, left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match op {
            BinaryOperator::Add => Self::add(left, right),
            BinaryOperator::Subtract => Self::subtract(left, right),
            BinaryOperator::Multiply => Self::multiply(left, right),
            BinaryOperator::Divide => Self::divide(left, right),
            BinaryOperator::Modulo => Self::modulo(left, right),
            BinaryOperator::Power => Self::power(left, right),
            _ => Err(RuntimeError::new(format!(
                "Not an arithmetic operator: {:?}",
                op
            ))),
        }
    }

    fn add(left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => a
                .checked_add(*b)
                .map(Value::Integer)
                .ok_or_else(|| RuntimeError::new(format!("Integer arithmetic overflow: {} + {}", a, b))),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
            (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a + *b as f64)),
            (Value::String(a), Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
            (Value::Char(a), Value::Char(b)) => Ok(Value::String(format!("{}{}", a, b))),
            (Value::String(a), Value::Char(b)) => Ok(Value::String(format!("{}{}", a, b))),
            (Value::Char(a), Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
            // String + any: auto-convert right to string (like JS/Python)
            (Value::String(a), other) => Ok(Value::String(format!("{}{}", a, other))),
            // any + String: auto-convert left to string
            (other, Value::String(b)) => Ok(Value::String(format!("{}{}", other, b))),
            _ => Err(RuntimeError::new(
                "Invalid operands for addition".to_string(),
            )),
        }
    }

    fn subtract(left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => a
                .checked_sub(*b)
                .map(Value::Integer)
                .ok_or_else(|| RuntimeError::new(format!("Integer arithmetic overflow: {} - {}", a, b))),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
            (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(*a as f64 - b)),
            (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a - *b as f64)),
            _ => Err(RuntimeError::new(
                "Invalid operands for subtraction".to_string(),
            )),
        }
    }

    fn multiply(left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => a
                .checked_mul(*b)
                .map(Value::Integer)
                .ok_or_else(|| RuntimeError::new(format!("Integer arithmetic overflow: {} * {}", a, b))),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
            (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(*a as f64 * b)),
            (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a * *b as f64)),
            _ => Err(RuntimeError::new(
                "Invalid operands for multiplication".to_string(),
            )),
        }
    }

    fn divide(left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => {
                if *b == 0 {
                    Err(RuntimeError::new("Division by zero".to_string()))
                } else {
                    // Floor division (Python-style //): rounds toward negative infinity.
                    // Rust's `/` truncates toward zero; we adjust when signs differ.
                    let d = a / b;
                    let r = a % b;
                    // If the remainder is non-zero and the signs of a and b differ,
                    // the true quotient is between d and d-1 — floor takes d-1.
                    let floor_d = if r != 0 && (*a < 0) != (*b < 0) { d - 1 } else { d };
                    Ok(Value::Integer(floor_d))
                }
            }
            (Value::Float(a), Value::Float(b)) => {
                if *b == 0.0 {
                    Err(RuntimeError::new("Division by zero".to_string()))
                } else {
                    Ok(Value::Float(a / b))
                }
            }
            (Value::Integer(a), Value::Float(b)) => {
                if *b == 0.0 {
                    Err(RuntimeError::new("Division by zero".to_string()))
                } else {
                    Ok(Value::Float(*a as f64 / b))
                }
            }
            (Value::Float(a), Value::Integer(b)) => {
                if *b == 0 {
                    Err(RuntimeError::new("Division by zero".to_string()))
                } else {
                    Ok(Value::Float(a / *b as f64))
                }
            }
            _ => Err(RuntimeError::new(
                "Invalid operands for division".to_string(),
            )),
        }
    }

    fn modulo(left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => {
                if *b == 0 {
                    Err(RuntimeError::new("Modulo by zero".to_string()))
                } else {
                    // Floor modulo: result has the same sign as the divisor.
                    // Rust's `%` is truncating; rem_euclid always returns a non-negative
                    // result when divisor > 0, matching floor semantics for positive divisors.
                    let r = a % b;
                    let floor_r = if r != 0 && (*a < 0) != (*b < 0) { r + b } else { r };
                    Ok(Value::Integer(floor_r))
                }
            }
            (Value::Float(a), Value::Float(b)) => {
                if *b == 0.0 {
                    Err(RuntimeError::new("Modulo by zero".to_string()))
                } else {
                    Ok(Value::Float(a % b))
                }
            }
            (Value::Integer(a), Value::Float(b)) => {
                if *b == 0.0 {
                    Err(RuntimeError::new("Modulo by zero".to_string()))
                } else {
                    Ok(Value::Float(*a as f64 % b))
                }
            }
            (Value::Float(a), Value::Integer(b)) => {
                if *b == 0 {
                    Err(RuntimeError::new("Modulo by zero".to_string()))
                } else {
                    Ok(Value::Float(a % *b as f64))
                }
            }
            _ => Err(RuntimeError::new("Invalid operands for modulo".to_string())),
        }
    }

    fn power(left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => {
                if *b < 0 {
                    Err(RuntimeError::new(
                        "Negative exponent not supported for integers".to_string(),
                    ))
                } else {
                    a.checked_pow(*b as u32).map(Value::Integer).ok_or_else(|| {
                        RuntimeError::new(format!("Integer arithmetic overflow: {} ** {}", a, b))
                            .with_code(crate::runtime::errors::ErrorCode::E0033)
                    })
                }
            }
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.powf(*b))),
            (Value::Integer(a), Value::Float(b)) => Ok(Value::Float((*a as f64).powf(*b))),
            (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a.powi(*b as i32))),
            _ => Err(RuntimeError::new("Invalid operands for power".to_string())),
        }
    }
}
