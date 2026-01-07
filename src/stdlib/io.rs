use crate::runtime::vm::{Value, RuntimeError};
use std::fs;
use std::path::Path;

/// I/O library
pub struct IOLib;

impl IOLib {
    /// Call an I/O library function
    pub fn call_function(name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
        match name {
            "read_file" => {
                if args.len() != 1 {
                    return Err(RuntimeError {
                        message: "read_file requires 1 argument (path)".to_string(),
                    });
                }
                match &args[0] {
                    Value::String(path) => {
                        fs::read_to_string(path)
                            .map(Value::String)
                            .map_err(|e| RuntimeError {
                                message: format!("Failed to read file: {}", e),
                            })
                    }
                    _ => Err(RuntimeError {
                        message: "read_file requires a string path".to_string(),
                    }),
                }
            }
            "write_file" => {
                if args.len() != 2 {
                    return Err(RuntimeError {
                        message: "write_file requires 2 arguments (path, content)".to_string(),
                    });
                }
                match (&args[0], &args[1]) {
                    (Value::String(path), Value::String(content)) => {
                        fs::write(path, content)
                            .map(|_| Value::Null)
                            .map_err(|e| RuntimeError {
                                message: format!("Failed to write file: {}", e),
                            })
                    }
                    _ => Err(RuntimeError {
                        message: "write_file requires strings".to_string(),
                    }),
                }
            }
            "file_exists" => {
                if args.len() != 1 {
                    return Err(RuntimeError {
                        message: "file_exists requires 1 argument (path)".to_string(),
                    });
                }
                match &args[0] {
                    Value::String(path) => {
                        Ok(Value::Boolean(Path::new(path).exists()))
                    }
                    _ => Err(RuntimeError {
                        message: "file_exists requires a string path".to_string(),
                    }),
                }
            }
            "list_dir" => {
                if args.len() != 1 {
                    return Err(RuntimeError {
                        message: "list_dir requires 1 argument (path)".to_string(),
                    });
                }
                match &args[0] {
                    Value::String(path) => {
                        let entries: Result<Vec<Value>, _> = fs::read_dir(path)
                            .map_err(|e| RuntimeError {
                                message: format!("Failed to read directory: {}", e),
                            })?
                            .map(|entry| {
                                entry
                                    .map(|e| Value::String(e.path().to_string_lossy().to_string()))
                                    .map_err(|e| RuntimeError {
                                        message: format!("Failed to read entry: {}", e),
                                    })
                            })
                            .collect();
                        entries.map(Value::Array)
                    }
                    _ => Err(RuntimeError {
                        message: "list_dir requires a string path".to_string(),
                    }),
                }
            }
            _ => Err(RuntimeError {
                message: format!("Unknown I/O function: {}", name),
            }),
        }
    }
}
