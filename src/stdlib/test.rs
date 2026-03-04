use crate::runtime::{Value, RuntimeError};

/// Testing framework library
pub struct TestLib;

impl TestLib {
    /// Call a test library function
    pub fn call_function(name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
        // For now, we'll use a static approach since we can't maintain state easily
        // In a real implementation, this would be integrated with the VM
        match name {
            "assert" => {
                if args.len() < 1 || args.len() > 2 {
                    return Err(RuntimeError::new("assert requires 1 or 2 arguments (condition, message?)".to_string()));
                }
                let condition = match &args[0] {
                    Value::Boolean(b) => *b,
                    _ => return Err(RuntimeError::new("assert condition must be a boolean".to_string())),
                };
                let message = if args.len() == 2 {
                    match &args[1] {
                        Value::String(s) => s.clone(),
                        v => v.to_string(),
                    }
                } else {
                    "Assertion failed".to_string()
                };
                
                if !condition {
                    eprintln!("❌ ASSERTION FAILED: {}", message);
                    return Err(RuntimeError::new(format!("Assertion failed: {}", message)));
                }
                Ok(Value::Null)
            }
            "assert_eq" | "assert_equal" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(RuntimeError::new("assert_eq requires 2 or 3 arguments (expected, actual, message?)".to_string()));
                }
                let expected = &args[0];
                let actual = &args[1];
                let message = if args.len() == 3 {
                    match &args[2] {
                        Value::String(s) => s.clone(),
                        v => v.to_string(),
                    }
                } else {
                    format!("Expected {:?}, got {:?}", expected, actual)
                };
                
                if expected != actual {
                    eprintln!("❌ ASSERTION FAILED: {}", message);
                    return Err(RuntimeError::new(format!("Assertion failed: {}", message)));
                }
                Ok(Value::Null)
            }
            "assert_ne" | "assert_not_equal" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(RuntimeError::new("assert_ne requires 2 or 3 arguments (a, b, message?)".to_string()));
                }
                let a = &args[0];
                let b = &args[1];
                let message = if args.len() == 3 {
                    match &args[2] {
                        Value::String(s) => s.clone(),
                        v => v.to_string(),
                    }
                } else {
                    format!("Values should not be equal: {:?}", a)
                };
                
                if a == b {
                    eprintln!("❌ ASSERTION FAILED: {}", message);
                    return Err(RuntimeError::new(format!("Assertion failed: {}", message)));
                }
                Ok(Value::Null)
            }
            "assert_true" => {
                if args.len() < 1 || args.len() > 2 {
                    return Err(RuntimeError::new("assert_true requires 1 or 2 arguments (value, message?)".to_string()));
                }
                let value = match &args[0] {
                    Value::Boolean(b) => *b,
                    _ => return Err(RuntimeError::new("assert_true requires a boolean".to_string())),
                };
                let message = if args.len() == 2 {
                    match &args[1] {
                        Value::String(s) => s.clone(),
                        v => v.to_string(),
                    }
                } else {
                    "Expected true".to_string()
                };
                
                if !value {
                    eprintln!("❌ ASSERTION FAILED: {}", message);
                    return Err(RuntimeError::new(format!("Assertion failed: {}", message)));
                }
                Ok(Value::Null)
            }
            "assert_false" => {
                if args.len() < 1 || args.len() > 2 {
                    return Err(RuntimeError::new("assert_false requires 1 or 2 arguments (value, message?)".to_string()));
                }
                let value = match &args[0] {
                    Value::Boolean(b) => *b,
                    _ => return Err(RuntimeError::new("assert_false requires a boolean".to_string())),
                };
                let message = if args.len() == 2 {
                    match &args[1] {
                        Value::String(s) => s.clone(),
                        v => v.to_string(),
                    }
                } else {
                    "Expected false".to_string()
                };
                
                if value {
                    eprintln!("❌ ASSERTION FAILED: {}", message);
                    return Err(RuntimeError::new(format!("Assertion failed: {}", message)));
                }
                Ok(Value::Null)
            }
            "test_passed" => {
                println!("✅ Test passed");
                Ok(Value::Boolean(true))
            }
            "test_failed" => {
                if args.len() > 0 {
                    let message = match &args[0] {
                        Value::String(s) => s.clone(),
                        v => v.to_string(),
                    };
                    eprintln!("❌ Test failed: {}", message);
                } else {
                    eprintln!("❌ Test failed");
                }
                Ok(Value::Boolean(false))
            }
            _ => Err(RuntimeError::new(format!("Unknown test function: {}", name))),
        }
    }
}

