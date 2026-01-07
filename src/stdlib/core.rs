use crate::runtime::vm::{Value, RuntimeError};
use std::collections::HashMap;

/// Core standard library functions
pub struct CoreLib;

impl CoreLib {
    /// Register all core functions in the environment
    pub fn register_functions(env: &mut HashMap<String, Value>) {
        // String functions
        env.insert("str_len".to_string(), Value::Function {
            name: "str_len".to_string(),
            params: vec![],
            body: vec![],
            closure: std::rc::Rc::new(crate::runtime::vm::Environment::new()),
        });
        
        // Math functions
        env.insert("math_abs".to_string(), Value::Function {
            name: "math_abs".to_string(),
            params: vec![],
            body: vec![],
            closure: std::rc::Rc::new(crate::runtime::vm::Environment::new()),
        });
        
        // Array functions
        env.insert("array_len".to_string(), Value::Function {
            name: "array_len".to_string(),
            params: vec![],
            body: vec![],
            closure: std::rc::Rc::new(crate::runtime::vm::Environment::new()),
        });
    }

    /// Call a core library function
    pub fn call_function(name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
        match name {
            // String functions
            "str_len" => {
                if args.len() != 1 {
                    return Err(RuntimeError {
                        message: "str_len requires 1 argument".to_string(),
                    });
                }
                match &args[0] {
                    Value::String(s) => Ok(Value::Integer(s.len() as i64)),
                    _ => Err(RuntimeError {
                        message: "str_len requires a string".to_string(),
                    }),
                }
            }
            "str_split" => {
                if args.len() != 2 {
                    return Err(RuntimeError {
                        message: "str_split requires 2 arguments".to_string(),
                    });
                }
                match (&args[0], &args[1]) {
                    (Value::String(s), Value::String(delim)) => {
                        let parts: Vec<Value> = s
                            .split(delim)
                            .map(|p| Value::String(p.to_string()))
                            .collect();
                        Ok(Value::Array(parts))
                    }
                    _ => Err(RuntimeError {
                        message: "str_split requires strings".to_string(),
                    }),
                }
            }
            "str_join" => {
                if args.len() != 2 {
                    return Err(RuntimeError {
                        message: "str_join requires 2 arguments".to_string(),
                    });
                }
                match (&args[0], &args[1]) {
                    (Value::Array(arr), Value::String(sep)) => {
                        let strings: Vec<String> = arr
                            .iter()
                            .map(|v| v.to_string())
                            .collect();
                        Ok(Value::String(strings.join(sep)))
                    }
                    _ => Err(RuntimeError {
                        message: "str_join requires array and string".to_string(),
                    }),
                }
            }
            "str_replace" => {
                if args.len() != 3 {
                    return Err(RuntimeError {
                        message: "str_replace requires 3 arguments".to_string(),
                    });
                }
                match (&args[0], &args[1], &args[2]) {
                    (Value::String(s), Value::String(old), Value::String(new)) => {
                        Ok(Value::String(s.replace(old, new)))
                    }
                    _ => Err(RuntimeError {
                        message: "str_replace requires strings".to_string(),
                    }),
                }
            }
            "str_contains" => {
                if args.len() != 2 {
                    return Err(RuntimeError {
                        message: "str_contains requires 2 arguments".to_string(),
                    });
                }
                match (&args[0], &args[1]) {
                    (Value::String(s), Value::String(sub)) => {
                        Ok(Value::Boolean(s.contains(sub)))
                    }
                    _ => Err(RuntimeError {
                        message: "str_contains requires strings".to_string(),
                    }),
                }
            }
            "str_starts_with" => {
                if args.len() != 2 {
                    return Err(RuntimeError {
                        message: "str_starts_with requires 2 arguments".to_string(),
                    });
                }
                match (&args[0], &args[1]) {
                    (Value::String(s), Value::String(prefix)) => {
                        Ok(Value::Boolean(s.starts_with(prefix)))
                    }
                    _ => Err(RuntimeError {
                        message: "str_starts_with requires strings".to_string(),
                    }),
                }
            }
            "str_ends_with" => {
                if args.len() != 2 {
                    return Err(RuntimeError {
                        message: "str_ends_with requires 2 arguments".to_string(),
                    });
                }
                match (&args[0], &args[1]) {
                    (Value::String(s), Value::String(suffix)) => {
                        Ok(Value::Boolean(s.ends_with(suffix)))
                    }
                    _ => Err(RuntimeError {
                        message: "str_ends_with requires strings".to_string(),
                    }),
                }
            }
            "str_upper" => {
                if args.len() != 1 {
                    return Err(RuntimeError {
                        message: "str_upper requires 1 argument".to_string(),
                    });
                }
                match &args[0] {
                    Value::String(s) => Ok(Value::String(s.to_uppercase())),
                    _ => Err(RuntimeError {
                        message: "str_upper requires a string".to_string(),
                    }),
                }
            }
            "str_lower" => {
                if args.len() != 1 {
                    return Err(RuntimeError {
                        message: "str_lower requires 1 argument".to_string(),
                    });
                }
                match &args[0] {
                    Value::String(s) => Ok(Value::String(s.to_lowercase())),
                    _ => Err(RuntimeError {
                        message: "str_lower requires a string".to_string(),
                    }),
                }
            }
            "str_trim" => {
                if args.len() != 1 {
                    return Err(RuntimeError {
                        message: "str_trim requires 1 argument".to_string(),
                    });
                }
                match &args[0] {
                    Value::String(s) => Ok(Value::String(s.trim().to_string())),
                    _ => Err(RuntimeError {
                        message: "str_trim requires a string".to_string(),
                    }),
                }
            }
            "str_strip" => {
                if args.len() < 1 || args.len() > 2 {
                    return Err(RuntimeError {
                        message: "str_strip requires 1 or 2 arguments".to_string(),
                    });
                }
                match &args[0] {
                    Value::String(s) => {
                        if args.len() == 2 {
                            match &args[1] {
                                Value::String(chars) => {
                                    let mut result = s.clone();
                                    for ch in chars.chars() {
                                        result = result.replace(ch, "");
                                    }
                                    Ok(Value::String(result))
                                }
                                _ => Err(RuntimeError {
                                    message: "str_strip second argument must be a string".to_string(),
                                }),
                            }
                        } else {
                            // Default: strip whitespace (same as trim)
                            Ok(Value::String(s.trim().to_string()))
                        }
                    }
                    _ => Err(RuntimeError {
                        message: "str_strip requires a string".to_string(),
                    }),
                }
            }
            "str_find" | "str_index_of" => {
                if args.len() != 2 {
                    return Err(RuntimeError {
                        message: "str_find/str_index_of requires 2 arguments".to_string(),
                    });
                }
                match (&args[0], &args[1]) {
                    (Value::String(s), Value::String(sub)) => {
                        if let Some(pos) = s.find(sub) {
                            Ok(Value::Integer(pos as i64))
                        } else {
                            Ok(Value::Integer(-1))
                        }
                    }
                    _ => Err(RuntimeError {
                        message: "str_find/str_index_of requires strings".to_string(),
                    }),
                }
            }
            "str_substring" => {
                if args.len() != 3 {
                    return Err(RuntimeError {
                        message: "str_substring requires 3 arguments (string, start, end)".to_string(),
                    });
                }
                match (&args[0], &args[1], &args[2]) {
                    (Value::String(s), Value::Integer(start), Value::Integer(end)) => {
                        let start_idx = *start as usize;
                        let end_idx = *end as usize;
                        if start_idx > s.len() || end_idx > s.len() || start_idx > end_idx {
                            return Err(RuntimeError {
                                message: format!("Invalid substring range: {} to {} for string of length {}", start_idx, end_idx, s.len()),
                            });
                        }
                        Ok(Value::String(s[start_idx..end_idx].to_string()))
                    }
                    _ => Err(RuntimeError {
                        message: "str_substring requires a string and two integer indices".to_string(),
                    }),
                }
            }
            "str_format" => {
                if args.len() < 1 {
                    return Err(RuntimeError {
                        message: "str_format requires at least 1 argument (template, ...args)".to_string(),
                    });
                }
                match &args[0] {
                    Value::String(template) => {
                        let mut result = template.clone();
                        // Simple placeholder replacement: {} -> argument
                        let mut arg_index = 1;
                        while let Some(pos) = result.find("{}") {
                            if arg_index >= args.len() {
                                return Err(RuntimeError {
                                    message: format!("Not enough arguments for str_format: found {} placeholders but only {} arguments", 
                                        template.matches("{}").count(), args.len() - 1),
                                });
                            }
                            let replacement = args[arg_index].to_string();
                            result.replace_range(pos..pos+2, &replacement);
                            arg_index += 1;
                        }
                        Ok(Value::String(result))
                    }
                    _ => Err(RuntimeError {
                        message: "str_format requires a string template".to_string(),
                    }),
                }
            }

            // Math functions
            "math_abs" => {
                if args.len() != 1 {
                    return Err(RuntimeError {
                        message: "math_abs requires 1 argument".to_string(),
                    });
                }
                match &args[0] {
                    Value::Integer(n) => Ok(Value::Integer(n.abs())),
                    Value::Float(n) => Ok(Value::Float(n.abs())),
                    _ => Err(RuntimeError {
                        message: "math_abs requires a number".to_string(),
                    }),
                }
            }
            "math_min" => {
                if args.len() != 2 {
                    return Err(RuntimeError {
                        message: "math_min requires 2 arguments".to_string(),
                    });
                }
                match (&args[0], &args[1]) {
                    (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(*a.min(b))),
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.min(*b))),
                    _ => Err(RuntimeError {
                        message: "math_min requires numbers".to_string(),
                    }),
                }
            }
            "math_max" => {
                if args.len() != 2 {
                    return Err(RuntimeError {
                        message: "math_max requires 2 arguments".to_string(),
                    });
                }
                match (&args[0], &args[1]) {
                    (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(*a.max(b))),
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.max(*b))),
                    _ => Err(RuntimeError {
                        message: "math_max requires numbers".to_string(),
                    }),
                }
            }
            "math_sqrt" => {
                if args.len() != 1 {
                    return Err(RuntimeError {
                        message: "math_sqrt requires 1 argument".to_string(),
                    });
                }
                match &args[0] {
                    Value::Integer(n) => Ok(Value::Float((*n as f64).sqrt())),
                    Value::Float(n) => Ok(Value::Float(n.sqrt())),
                    _ => Err(RuntimeError {
                        message: "math_sqrt requires a number".to_string(),
                    }),
                }
            }
            "math_sin" => {
                if args.len() != 1 {
                    return Err(RuntimeError {
                        message: "math_sin requires 1 argument".to_string(),
                    });
                }
                match &args[0] {
                    Value::Integer(n) => Ok(Value::Float(libm::sin(*n as f64))),
                    Value::Float(n) => Ok(Value::Float(libm::sin(*n))),
                    _ => Err(RuntimeError {
                        message: "math_sin requires a number".to_string(),
                    }),
                }
            }
            "math_cos" => {
                if args.len() != 1 {
                    return Err(RuntimeError {
                        message: "math_cos requires 1 argument".to_string(),
                    });
                }
                match &args[0] {
                    Value::Integer(n) => Ok(Value::Float(libm::cos(*n as f64))),
                    Value::Float(n) => Ok(Value::Float(libm::cos(*n))),
                    _ => Err(RuntimeError {
                        message: "math_cos requires a number".to_string(),
                    }),
                }
            }
            "math_tan" => {
                if args.len() != 1 {
                    return Err(RuntimeError {
                        message: "math_tan requires 1 argument".to_string(),
                    });
                }
                match &args[0] {
                    Value::Integer(n) => Ok(Value::Float(libm::tan(*n as f64))),
                    Value::Float(n) => Ok(Value::Float(libm::tan(*n))),
                    _ => Err(RuntimeError {
                        message: "math_tan requires a number".to_string(),
                    }),
                }
            }
            "math_floor" => {
                if args.len() != 1 {
                    return Err(RuntimeError {
                        message: "math_floor requires 1 argument".to_string(),
                    });
                }
                match &args[0] {
                    Value::Integer(n) => Ok(Value::Integer(*n)),
                    Value::Float(n) => Ok(Value::Float(libm::floor(*n))),
                    _ => Err(RuntimeError {
                        message: "math_floor requires a number".to_string(),
                    }),
                }
            }
            "math_ceil" => {
                if args.len() != 1 {
                    return Err(RuntimeError {
                        message: "math_ceil requires 1 argument".to_string(),
                    });
                }
                match &args[0] {
                    Value::Integer(n) => Ok(Value::Integer(*n)),
                    Value::Float(n) => Ok(Value::Float(libm::ceil(*n))),
                    _ => Err(RuntimeError {
                        message: "math_ceil requires a number".to_string(),
                    }),
                }
            }
            "math_round" => {
                if args.len() != 1 {
                    return Err(RuntimeError {
                        message: "math_round requires 1 argument".to_string(),
                    });
                }
                match &args[0] {
                    Value::Integer(n) => Ok(Value::Integer(*n)),
                    Value::Float(n) => Ok(Value::Integer(libm::round(*n) as i64)),
                    _ => Err(RuntimeError {
                        message: "math_round requires a number".to_string(),
                    }),
                }
            }
            "math_pow" => {
                if args.len() != 2 {
                    return Err(RuntimeError {
                        message: "math_pow requires 2 arguments (base, exponent)".to_string(),
                    });
                }
                match (&args[0], &args[1]) {
                    (Value::Integer(a), Value::Integer(b)) => {
                        Ok(Value::Float((*a as f64).powf(*b as f64)))
                    }
                    (Value::Float(a), Value::Float(b)) => {
                        Ok(Value::Float(a.powf(*b)))
                    }
                    (Value::Integer(a), Value::Float(b)) => {
                        Ok(Value::Float((*a as f64).powf(*b)))
                    }
                    (Value::Float(a), Value::Integer(b)) => {
                        Ok(Value::Float(a.powf(*b as f64)))
                    }
                    _ => Err(RuntimeError {
                        message: "math_pow requires numbers".to_string(),
                    }),
                }
            }
            "math_random" => {
                if args.len() != 0 {
                    return Err(RuntimeError {
                        message: "math_random requires 0 arguments".to_string(),
                    });
                }
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                use std::time::{SystemTime, UNIX_EPOCH};
                let seed = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64;
                let mut hasher = DefaultHasher::new();
                seed.hash(&mut hasher);
                let hash = hasher.finish();
                // Convert to float between 0 and 1
                let random = (hash % 1_000_000) as f64 / 1_000_000.0;
                Ok(Value::Float(random))
            }
            "math_pi" => {
                if args.len() != 0 {
                    return Err(RuntimeError {
                        message: "math_pi requires 0 arguments".to_string(),
                    });
                }
                Ok(Value::Float(std::f64::consts::PI))
            }
            "math_e" => {
                if args.len() != 0 {
                    return Err(RuntimeError {
                        message: "math_e requires 0 arguments".to_string(),
                    });
                }
                Ok(Value::Float(std::f64::consts::E))
            }
            "math_log" => {
                if args.len() < 1 || args.len() > 2 {
                    return Err(RuntimeError {
                        message: "math_log requires 1 or 2 arguments (number, base?)".to_string(),
                    });
                }
                match &args[0] {
                    Value::Integer(n) => {
                        let num = *n as f64;
                        if args.len() == 2 {
                            match &args[1] {
                                Value::Integer(b) => {
                                    let base = *b as f64;
                                    if base <= 0.0 || base == 1.0 {
                                        return Err(RuntimeError {
                                            message: "math_log base must be positive and not equal to 1".to_string(),
                                        });
                                    }
                                    Ok(Value::Float(libm::log(num) / libm::log(base)))
                                }
                                Value::Float(b) => {
                                    if *b <= 0.0 || *b == 1.0 {
                                        return Err(RuntimeError {
                                            message: "math_log base must be positive and not equal to 1".to_string(),
                                        });
                                    }
                                    Ok(Value::Float(libm::log(num) / libm::log(*b)))
                                }
                                _ => Err(RuntimeError {
                                    message: "math_log base must be a number".to_string(),
                                }),
                            }
                        } else {
                            // Natural logarithm (base e)
                            Ok(Value::Float(libm::log(num)))
                        }
                    }
                    Value::Float(n) => {
                        if args.len() == 2 {
                            match &args[1] {
                                Value::Integer(b) => {
                                    let base = *b as f64;
                                    if base <= 0.0 || base == 1.0 {
                                        return Err(RuntimeError {
                                            message: "math_log base must be positive and not equal to 1".to_string(),
                                        });
                                    }
                                    Ok(Value::Float(libm::log(*n) / libm::log(base)))
                                }
                                Value::Float(b) => {
                                    if *b <= 0.0 || *b == 1.0 {
                                        return Err(RuntimeError {
                                            message: "math_log base must be positive and not equal to 1".to_string(),
                                        });
                                    }
                                    Ok(Value::Float(libm::log(*n) / libm::log(*b)))
                                }
                                _ => Err(RuntimeError {
                                    message: "math_log base must be a number".to_string(),
                                }),
                            }
                        } else {
                            Ok(Value::Float(libm::log(*n)))
                        }
                    }
                    _ => Err(RuntimeError {
                        message: "math_log requires a number".to_string(),
                    }),
                }
            }
            "math_exp" => {
                if args.len() != 1 {
                    return Err(RuntimeError {
                        message: "math_exp requires 1 argument".to_string(),
                    });
                }
                match &args[0] {
                    Value::Integer(n) => Ok(Value::Float(libm::exp(*n as f64))),
                    Value::Float(n) => Ok(Value::Float(libm::exp(*n))),
                    _ => Err(RuntimeError {
                        message: "math_exp requires a number".to_string(),
                    }),
                }
            }

            // Array functions
            "array_len" => {
                if args.len() != 1 {
                    return Err(RuntimeError {
                        message: "array_len requires 1 argument".to_string(),
                    });
                }
                match &args[0] {
                    Value::Array(arr) => Ok(Value::Integer(arr.len() as i64)),
                    _ => Err(RuntimeError {
                        message: "array_len requires an array".to_string(),
                    }),
                }
            }
            "array_append" => {
                if args.len() != 2 {
                    return Err(RuntimeError {
                        message: "array_append requires 2 arguments".to_string(),
                    });
                }
                match &args[0] {
                    Value::Array(arr) => {
                        let mut new_arr = arr.clone();
                        new_arr.push(args[1].clone());
                        Ok(Value::Array(new_arr))
                    }
                    _ => Err(RuntimeError {
                        message: "array_append requires an array".to_string(),
                    }),
                }
            }
            "array_map" => {
                if args.len() != 2 {
                    return Err(RuntimeError {
                        message: "array_map requires 2 arguments".to_string(),
                    });
                }
                // Simplified - would need function execution
                match &args[0] {
                    Value::Array(arr) => {
                        // For now, just return the array
                        Ok(Value::Array(arr.clone()))
                    }
                    _ => Err(RuntimeError {
                        message: "array_map requires an array".to_string(),
                    }),
                }
            }
            "array_filter" => {
                if args.len() != 2 {
                    return Err(RuntimeError {
                        message: "array_filter requires 2 arguments".to_string(),
                    });
                }
                // Note: Full implementation requires VM integration for function execution
                // For now, validate arguments and return empty array
                // TODO: Execute predicate function for each element
                match &args[0] {
                    Value::Array(arr) => {
                        // Basic validation - full implementation needs function execution
                        if let Value::Function { .. } = &args[1] {
                            // Function provided but can't execute without VM
                            // Return empty array as placeholder
                            Ok(Value::Array(Vec::new()))
                        } else {
                            Err(RuntimeError {
                                message: "array_filter requires a function as second argument".to_string(),
                            })
                        }
                    }
                    _ => Err(RuntimeError {
                        message: "array_filter requires an array".to_string(),
                    }),
                }
            }
            "array_reduce" => {
                if args.len() != 3 {
                    return Err(RuntimeError {
                        message: "array_reduce requires 3 arguments (array, function, initial)".to_string(),
                    });
                }
                // Note: Full implementation requires VM integration
                match &args[0] {
                    Value::Array(arr) => {
                        if let Value::Function { .. } = &args[1] {
                            // Return initial value as placeholder
                            Ok(args[2].clone())
                        } else {
                            Err(RuntimeError {
                                message: "array_reduce requires a function as second argument".to_string(),
                            })
                        }
                    }
                    _ => Err(RuntimeError {
                        message: "array_reduce requires an array".to_string(),
                    }),
                }
            }
            "array_find" => {
                if args.len() != 2 {
                    return Err(RuntimeError {
                        message: "array_find requires 2 arguments".to_string(),
                    });
                }
                // Note: Full implementation requires VM integration
                match &args[0] {
                    Value::Array(arr) => {
                        if let Value::Function { .. } = &args[1] {
                            // Return null as placeholder (not found)
                            Ok(Value::Null)
                        } else {
                            Err(RuntimeError {
                                message: "array_find requires a function as second argument".to_string(),
                            })
                        }
                    }
                    _ => Err(RuntimeError {
                        message: "array_find requires an array".to_string(),
                    }),
                }
            }
            "array_find_index" => {
                if args.len() != 2 {
                    return Err(RuntimeError {
                        message: "array_find_index requires 2 arguments".to_string(),
                    });
                }
                // Note: Full implementation requires VM integration
                match &args[0] {
                    Value::Array(_arr) => {
                        if let Value::Function { .. } = &args[1] {
                            // Return -1 as placeholder (not found)
                            Ok(Value::Integer(-1))
                        } else {
                            Err(RuntimeError {
                                message: "array_find_index requires a function as second argument".to_string(),
                            })
                        }
                    }
                    _ => Err(RuntimeError {
                        message: "array_find_index requires an array".to_string(),
                    }),
                }
            }
            "array_some" => {
                if args.len() != 2 {
                    return Err(RuntimeError {
                        message: "array_some requires 2 arguments".to_string(),
                    });
                }
                // Note: Full implementation requires VM integration
                match &args[0] {
                    Value::Array(_arr) => {
                        if let Value::Function { .. } = &args[1] {
                            Ok(Value::Boolean(false))
                        } else {
                            Err(RuntimeError {
                                message: "array_some requires a function as second argument".to_string(),
                            })
                        }
                    }
                    _ => Err(RuntimeError {
                        message: "array_some requires an array".to_string(),
                    }),
                }
            }
            "array_every" => {
                if args.len() != 2 {
                    return Err(RuntimeError {
                        message: "array_every requires 2 arguments".to_string(),
                    });
                }
                // Note: Full implementation requires VM integration
                match &args[0] {
                    Value::Array(_arr) => {
                        if let Value::Function { .. } = &args[1] {
                            Ok(Value::Boolean(false))
                        } else {
                            Err(RuntimeError {
                                message: "array_every requires a function as second argument".to_string(),
                            })
                        }
                    }
                    _ => Err(RuntimeError {
                        message: "array_every requires an array".to_string(),
                    }),
                }
            }
            "array_insert" => {
                if args.len() != 3 {
                    return Err(RuntimeError {
                        message: "array_insert requires 3 arguments (array, index, value)".to_string(),
                    });
                }
                match (&args[0], &args[1]) {
                    (Value::Array(arr), Value::Integer(idx)) => {
                        let index = *idx as usize;
                        if index > arr.len() {
                            return Err(RuntimeError {
                                message: format!("Index {} out of bounds for array of length {}", index, arr.len()),
                            });
                        }
                        let mut new_arr = arr.clone();
                        new_arr.insert(index, args[2].clone());
                        Ok(Value::Array(new_arr))
                    }
                    _ => Err(RuntimeError {
                        message: "array_insert requires an array and integer index".to_string(),
                    }),
                }
            }
            "array_remove" => {
                if args.len() != 2 {
                    return Err(RuntimeError {
                        message: "array_remove requires 2 arguments (array, index)".to_string(),
                    });
                }
                match (&args[0], &args[1]) {
                    (Value::Array(arr), Value::Integer(idx)) => {
                        let index = *idx as usize;
                        if index >= arr.len() {
                            return Err(RuntimeError {
                                message: format!("Index {} out of bounds for array of length {}", index, arr.len()),
                            });
                        }
                        let mut new_arr = arr.clone();
                        new_arr.remove(index);
                        Ok(Value::Array(new_arr))
                    }
                    _ => Err(RuntimeError {
                        message: "array_remove requires an array and integer index".to_string(),
                    }),
                }
            }
            "array_pop" => {
                if args.len() != 1 {
                    return Err(RuntimeError {
                        message: "array_pop requires 1 argument".to_string(),
                    });
                }
                match &args[0] {
                    Value::Array(arr) => {
                        if arr.is_empty() {
                            Ok(Value::Null)
                        } else {
                            let mut new_arr = arr.clone();
                            let popped = new_arr.pop().unwrap();
                            Ok(popped)
                        }
                    }
                    _ => Err(RuntimeError {
                        message: "array_pop requires an array".to_string(),
                    }),
                }
            }
            "array_reverse" => {
                if args.len() != 1 {
                    return Err(RuntimeError {
                        message: "array_reverse requires 1 argument".to_string(),
                    });
                }
                match &args[0] {
                    Value::Array(arr) => {
                        let mut new_arr = arr.clone();
                        new_arr.reverse();
                        Ok(Value::Array(new_arr))
                    }
                    _ => Err(RuntimeError {
                        message: "array_reverse requires an array".to_string(),
                    }),
                }
            }
            "array_slice" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(RuntimeError {
                        message: "array_slice requires 2 or 3 arguments (array, start, end?)".to_string(),
                    });
                }
                match (&args[0], &args[1]) {
                    (Value::Array(arr), Value::Integer(start)) => {
                        let start_idx = *start as usize;
                        let end_idx = if args.len() == 3 {
                            match &args[2] {
                                Value::Integer(end) => *end as usize,
                                _ => return Err(RuntimeError {
                                    message: "array_slice end index must be an integer".to_string(),
                                }),
                            }
                        } else {
                            arr.len()
                        };
                        if start_idx > arr.len() || end_idx > arr.len() || start_idx > end_idx {
                            return Err(RuntimeError {
                                message: format!("Invalid slice range: {} to {} for array of length {}", start_idx, end_idx, arr.len()),
                            });
                        }
                        Ok(Value::Array(arr[start_idx..end_idx].to_vec()))
                    }
                    _ => Err(RuntimeError {
                        message: "array_slice requires an array and integer start index".to_string(),
                    }),
                }
            }
            "array_concat" => {
                if args.len() != 2 {
                    return Err(RuntimeError {
                        message: "array_concat requires 2 arguments".to_string(),
                    });
                }
                match (&args[0], &args[1]) {
                    (Value::Array(arr1), Value::Array(arr2)) => {
                        let mut new_arr = arr1.clone();
                        new_arr.extend_from_slice(arr2);
                        Ok(Value::Array(new_arr))
                    }
                    _ => Err(RuntimeError {
                        message: "array_concat requires two arrays".to_string(),
                    }),
                }
            }
            "array_index_of" => {
                if args.len() != 2 {
                    return Err(RuntimeError {
                        message: "array_index_of requires 2 arguments".to_string(),
                    });
                }
                match &args[0] {
                    Value::Array(arr) => {
                        for (i, item) in arr.iter().enumerate() {
                            // Simple equality check
                            if let (Value::Integer(a), Value::Integer(b)) = (item, &args[1]) {
                                if a == b {
                                    return Ok(Value::Integer(i as i64));
                                }
                            } else if let (Value::Float(a), Value::Float(b)) = (item, &args[1]) {
                                if (a - b).abs() < f64::EPSILON {
                                    return Ok(Value::Integer(i as i64));
                                }
                            } else if let (Value::String(a), Value::String(b)) = (item, &args[1]) {
                                if a == b {
                                    return Ok(Value::Integer(i as i64));
                                }
                            } else if let (Value::Boolean(a), Value::Boolean(b)) = (item, &args[1]) {
                                if a == b {
                                    return Ok(Value::Integer(i as i64));
                                }
                            } else if matches!((item, &args[1]), (Value::Null, Value::Null)) {
                                return Ok(Value::Integer(i as i64));
                            }
                        }
                        Ok(Value::Integer(-1))
                    }
                    _ => Err(RuntimeError {
                        message: "array_index_of requires an array".to_string(),
                    }),
                }
            }

            // Type conversion
            "to_string" | "string" => {
                if args.len() != 1 {
                    return Err(RuntimeError {
                        message: "to_string/string requires 1 argument".to_string(),
                    });
                }
                Ok(Value::String(args[0].to_string()))
            }
            "to_int" | "int" => {
                if args.len() != 1 {
                    return Err(RuntimeError {
                        message: "to_int/int requires 1 argument".to_string(),
                    });
                }
                match &args[0] {
                    Value::Integer(n) => Ok(Value::Integer(*n)),
                    Value::Float(n) => Ok(Value::Integer(*n as i64)),
                    Value::String(s) => {
                        s.parse::<i64>()
                            .map(Value::Integer)
                            .map_err(|_| RuntimeError {
                                message: "Cannot convert string to int".to_string(),
                            })
                    }
                    _ => Err(RuntimeError {
                        message: "Cannot convert to int".to_string(),
                    }),
                }
            }
            "to_float" | "float" => {
                if args.len() != 1 {
                    return Err(RuntimeError {
                        message: "to_float/float requires 1 argument".to_string(),
                    });
                }
                match &args[0] {
                    Value::Integer(n) => Ok(Value::Float(*n as f64)),
                    Value::Float(n) => Ok(Value::Float(*n)),
                    Value::String(s) => {
                        s.parse::<f64>()
                            .map(Value::Float)
                            .map_err(|_| RuntimeError {
                                message: "Cannot convert string to float".to_string(),
                            })
                    }
                    _ => Err(RuntimeError {
                        message: "Cannot convert to float".to_string(),
                    }),
                }
            }
            "to_bool" | "bool" => {
                if args.len() != 1 {
                    return Err(RuntimeError {
                        message: "to_bool/bool requires 1 argument".to_string(),
                    });
                }
                match &args[0] {
                    Value::Boolean(b) => Ok(Value::Boolean(*b)),
                    Value::Integer(n) => Ok(Value::Boolean(*n != 0)),
                    Value::Float(n) => Ok(Value::Boolean(*n != 0.0)),
                    Value::String(s) => Ok(Value::Boolean(!s.is_empty())),
                    Value::Null => Ok(Value::Boolean(false)),
                    Value::Array(arr) => Ok(Value::Boolean(!arr.is_empty())),
                    Value::Map(map) => Ok(Value::Boolean(!map.is_empty())),
                    _ => Ok(Value::Boolean(true)),
                }
            }

            // Utility functions
            "len" => {
                if args.len() != 1 {
                    return Err(RuntimeError {
                        message: "len requires 1 argument".to_string(),
                    });
                }
                match &args[0] {
                    Value::String(s) => Ok(Value::Integer(s.len() as i64)),
                    Value::Array(arr) => Ok(Value::Integer(arr.len() as i64)),
                    Value::Map(map) => Ok(Value::Integer(map.len() as i64)),
                    _ => Err(RuntimeError {
                        message: "len requires string, array, or map".to_string(),
                    }),
                }
            }
            "type" => {
                if args.len() != 1 {
                    return Err(RuntimeError {
                        message: "type requires 1 argument".to_string(),
                    });
                }
                Ok(Value::String(args[0].type_name().to_string()))
            }
            "input" => {
                if args.len() > 1 {
                    return Err(RuntimeError {
                        message: "input requires 0 or 1 argument (prompt?)".to_string(),
                    });
                }
                // Print prompt if provided
                if args.len() == 1 {
                    match &args[0] {
                        Value::String(prompt) => {
                            print!("{}", prompt);
                            use std::io::Write;
                            let _ = std::io::stdout().flush();
                        }
                        _ => {
                            print!("{}", args[0].to_string());
                            use std::io::Write;
                            let _ = std::io::stdout().flush();
                        }
                    }
                }
                // Read from stdin
                let mut line = String::new();
                use std::io;
                match io::stdin().read_line(&mut line) {
                    Ok(_) => {
                        // Remove trailing newline
                        line = line.trim_end().to_string();
                        Ok(Value::String(line))
                    }
                    Err(e) => Err(RuntimeError {
                        message: format!("Error reading input: {}", e),
                    }),
                }
            }

            _ => Err(RuntimeError {
                message: format!("Unknown function: {}", name),
            }),
        }
    }
}
