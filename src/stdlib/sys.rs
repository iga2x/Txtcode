use crate::runtime::vm::{Value, RuntimeError};
use std::env;

/// System library
pub struct SysLib;

impl SysLib {
    /// Call a system library function
    pub fn call_function(name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
        match name {
            "getenv" => {
                if args.len() != 1 {
                    return Err(RuntimeError {
                        message: "getenv requires 1 argument (name)".to_string(),
                    });
                }
                match &args[0] {
                    Value::String(name) => {
                        Ok(env::var(name)
                            .map(Value::String)
                            .unwrap_or(Value::Null))
                    }
                    _ => Err(RuntimeError {
                        message: "getenv requires a string".to_string(),
                    }),
                }
            }
            "setenv" => {
                if args.len() != 2 {
                    return Err(RuntimeError {
                        message: "setenv requires 2 arguments (name, value)".to_string(),
                    });
                }
                match (&args[0], &args[1]) {
                    (Value::String(name), Value::String(value)) => {
                        env::set_var(name, value);
                        Ok(Value::Null)
                    }
                    _ => Err(RuntimeError {
                        message: "setenv requires strings".to_string(),
                    }),
                }
            }
            "platform" => {
                Ok(Value::String(std::env::consts::OS.to_string()))
            }
            "arch" => {
                Ok(Value::String(std::env::consts::ARCH.to_string()))
            }
            "exec" => {
                if args.len() < 1 {
                    return Err(RuntimeError {
                        message: "exec requires at least 1 argument (command)".to_string(),
                    });
                }
                match &args[0] {
                    Value::String(cmd) => {
                        // In a full implementation, this would execute the command
                        // For security, this is disabled by default
                        Ok(Value::String(format!("Exec: {} (disabled for security)", cmd)))
                    }
                    _ => Err(RuntimeError {
                        message: "exec requires a string command".to_string(),
                    }),
                }
            }
            _ => Err(RuntimeError {
                message: format!("Unknown system function: {}", name),
            }),
        }
    }
}
