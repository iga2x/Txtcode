use crate::runtime::{RuntimeError, Value};
use std::sync::Arc;
use crate::stdlib::FunctionExecutor;

/// Core standard library functions
pub struct CoreLib;

impl CoreLib {
    pub fn call_function<E: FunctionExecutor>(
        name: &str,
        args: &[Value],
        executor: Option<&mut E>,
    ) -> Result<Value, RuntimeError> {
        match name {
            "print" => {
                if let Some(val) = args.first() {
                    println!("{}", val);
                }
                Ok(Value::Null)
            }
            "len" => {
                if let Some(val) = args.first() {
                    match val {
                        Value::String(s) => Ok(Value::Integer(s.len() as i64)),
                        Value::Array(arr) => Ok(Value::Integer(arr.len() as i64)),
                        Value::Map(map) => Ok(Value::Integer(map.len() as i64)),
                        Value::Set(set) => Ok(Value::Integer(set.len() as i64)),
                        _ => Err(RuntimeError::new(
                            "len() requires string, array, map, or set".to_string(),
                        )),
                    }
                } else {
                    Err(RuntimeError::new("len() requires one argument".to_string()))
                }
            }
            "type" => {
                if let Some(val) = args.first() {
                    let type_name = match val {
                        Value::Integer(_) => "int",
                        Value::Float(_) => "float",
                        Value::String(_) => "string",
                        Value::Char(_) => "char",
                        Value::Boolean(_) => "bool",
                        Value::Null => "null",
                        Value::Array(_) => "array",
                        Value::Map(_) => "map",
                        Value::Set(_) => "set",
                        Value::Function(_, _, _, _) => "function",
                        Value::Struct(_, _) => "struct",
                        Value::Enum(_, _, _) => "enum",
                        Value::Result(_, _) => "result",
                        Value::Future(_) => "future",
                        Value::FunctionRef(_) => "function_ref",
                        Value::Bytes(_) => "bytes",
                    };
                    Ok(Value::String(Arc::from(type_name.to_string())))
                } else {
                    Err(RuntimeError::new(
                        "type() requires one argument".to_string(),
                    ))
                }
            }
            "input" => {
                use std::io::{self, Write};
                print!("> ");
                io::stdout()
                    .flush()
                    .map_err(|e| RuntimeError::new(format!("input: flush failed: {}", e)))?;
                let mut input = String::new();
                io::stdin()
                    .read_line(&mut input)
                    .map_err(|e| RuntimeError::new(format!("input: read failed: {}", e)))?;
                Ok(Value::String(Arc::from(input.trim().to_string())))
            }
            // Math functions
            "math_sin" | "sin" => {
                if let Some(Value::Float(x)) = args.first() {
                    Ok(Value::Float(x.sin()))
                } else if let Some(Value::Integer(x)) = args.first() {
                    Ok(Value::Float((*x as f64).sin()))
                } else {
                    Err(RuntimeError::new("sin() requires a number".to_string()))
                }
            }
            "math_cos" | "cos" => {
                if let Some(Value::Float(x)) = args.first() {
                    Ok(Value::Float(x.cos()))
                } else if let Some(Value::Integer(x)) = args.first() {
                    Ok(Value::Float((*x as f64).cos()))
                } else {
                    Err(RuntimeError::new("cos() requires a number".to_string()))
                }
            }
            "math_tan" | "tan" => {
                if let Some(Value::Float(x)) = args.first() {
                    Ok(Value::Float(x.tan()))
                } else if let Some(Value::Integer(x)) = args.first() {
                    Ok(Value::Float((*x as f64).tan()))
                } else {
                    Err(RuntimeError::new("tan() requires a number".to_string()))
                }
            }
            "math_log" | "log" => {
                if args.len() == 1 {
                    if let Some(Value::Float(x)) = args.first() {
                        Ok(Value::Float(x.ln()))
                    } else if let Some(Value::Integer(x)) = args.first() {
                        Ok(Value::Float((*x as f64).ln()))
                    } else {
                        Err(RuntimeError::new("log() requires a number".to_string()))
                    }
                } else if args.len() == 2 {
                    let base = match args.get(1).unwrap() {
                        Value::Float(b) => *b,
                        Value::Integer(b) => *b as f64,
                        _ => {
                            return Err(RuntimeError::new(
                                "log() base must be a number".to_string(),
                            ))
                        }
                    };
                    let x = match args.first().unwrap() {
                        Value::Float(x) => *x,
                        Value::Integer(x) => *x as f64,
                        _ => return Err(RuntimeError::new("log() requires a number".to_string())),
                    };
                    Ok(Value::Float(x.log(base)))
                } else {
                    Err(RuntimeError::new(
                        "log() requires 1 or 2 arguments".to_string(),
                    ))
                }
            }
            "math_sqrt" | "sqrt" => {
                if let Some(Value::Float(x)) = args.first() {
                    if *x < 0.0 {
                        Err(RuntimeError::new("sqrt() of negative number".to_string()))
                    } else {
                        Ok(Value::Float(x.sqrt()))
                    }
                } else if let Some(Value::Integer(x)) = args.first() {
                    if *x < 0 {
                        Err(RuntimeError::new("sqrt() of negative number".to_string()))
                    } else {
                        Ok(Value::Float((*x as f64).sqrt()))
                    }
                } else {
                    Err(RuntimeError::new("sqrt() requires a number".to_string()))
                }
            }
            "math_pow" | "pow" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new("pow() requires 2 arguments".to_string()));
                }
                let base = match args.first().unwrap() {
                    Value::Float(b) => *b,
                    Value::Integer(b) => *b as f64,
                    _ => return Err(RuntimeError::new("pow() base must be a number".to_string())),
                };
                let exp = match args.get(1).unwrap() {
                    Value::Float(e) => *e,
                    Value::Integer(e) => *e as f64,
                    _ => {
                        return Err(RuntimeError::new(
                            "pow() exponent must be a number".to_string(),
                        ))
                    }
                };
                Ok(Value::Float(base.powf(exp)))
            }
            "math_abs" | "abs" => {
                if let Some(Value::Float(x)) = args.first() {
                    Ok(Value::Float(x.abs()))
                } else if let Some(Value::Integer(x)) = args.first() {
                    Ok(Value::Integer(x.abs()))
                } else {
                    Err(RuntimeError::new("abs() requires a number".to_string()))
                }
            }
            "math_floor" | "floor" => {
                if let Some(Value::Float(x)) = args.first() {
                    Ok(Value::Float(x.floor()))
                } else if let Some(Value::Integer(x)) = args.first() {
                    Ok(Value::Integer(*x))
                } else {
                    Err(RuntimeError::new("floor() requires a number".to_string()))
                }
            }
            "math_ceil" | "ceil" => {
                if let Some(Value::Float(x)) = args.first() {
                    Ok(Value::Float(x.ceil()))
                } else if let Some(Value::Integer(x)) = args.first() {
                    Ok(Value::Integer(*x))
                } else {
                    Err(RuntimeError::new("ceil() requires a number".to_string()))
                }
            }
            "math_round" | "round" => {
                if let Some(Value::Float(x)) = args.first() {
                    Ok(Value::Float(x.round()))
                } else if let Some(Value::Integer(x)) = args.first() {
                    Ok(Value::Integer(*x))
                } else {
                    Err(RuntimeError::new("round() requires a number".to_string()))
                }
            }
            // String functions
            "str_split" | "split" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(RuntimeError::new(
                        "split() requires 1 or 2 arguments".to_string(),
                    ));
                }
                let s = match args.first().unwrap() {
                    Value::String(s) => s,
                    _ => return Err(RuntimeError::new("split() requires a string".to_string())),
                };
                let delimiter = if args.len() == 2 {
                    match args.get(1).unwrap() {
                        Value::String(d) => d.as_ref(),
                        _ => {
                            return Err(RuntimeError::new(
                                "split() delimiter must be a string".to_string(),
                            ))
                        }
                    }
                } else {
                    " "
                };
                let parts: Vec<Value> = s
                    .split(delimiter)
                    .map(|p| Value::String(Arc::from(p.to_string())))
                    .collect();
                Ok(Value::Array(parts))
            }
            "str_join" | "join" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "join() requires 2 arguments (array, separator)".to_string(),
                    ));
                }
                let arr = match args.first().unwrap() {
                    Value::Array(a) => a,
                    _ => {
                        return Err(RuntimeError::new(
                            "join() first argument must be an array".to_string(),
                        ))
                    }
                };
                let sep = match args.get(1).unwrap() {
                    Value::String(s) => s,
                    _ => {
                        return Err(RuntimeError::new(
                            "join() second argument must be a string".to_string(),
                        ))
                    }
                };
                let strings: Vec<String> = arr
                    .iter()
                    .map(|v| match v {
                        Value::String(s) => s.to_string(),
                        _ => v.to_string(),
                    })
                    .collect();
                Ok(Value::String(Arc::from(strings.join(sep))))
            }
            // R.4: str_build(parts) — O(n) string concatenation from an array of parts.
            // Avoids the O(n²) cost of `+` in a loop by pre-allocating the total capacity.
            "str_build" => {
                let arr = match args.first() {
                    Some(Value::Array(a)) => a.clone(),
                    Some(other) => {
                        // Scalar: just convert to string
                        return Ok(Value::String(Arc::from(other.to_string())));
                    }
                    None => return Ok(Value::String(Arc::from(String::new()))),
                };
                let parts: Vec<String> = arr.iter().map(|v| v.to_string()).collect();
                let total_len: usize = parts.iter().map(|s| s.len()).sum();
                let mut result = String::with_capacity(total_len);
                for part in &parts {
                    result.push_str(part);
                }
                Ok(Value::String(Arc::from(result)))
            }
            "str_replace" | "replace" => {
                if args.len() != 3 {
                    return Err(RuntimeError::new(
                        "replace() requires 3 arguments (string, old, new)".to_string(),
                    ));
                }
                let s = match args.first().unwrap() {
                    Value::String(s) => s,
                    _ => {
                        return Err(RuntimeError::new(
                            "replace() first argument must be a string".to_string(),
                        ))
                    }
                };
                let old = match args.get(1).unwrap() {
                    Value::String(o) => o,
                    _ => {
                        return Err(RuntimeError::new(
                            "replace() second argument must be a string".to_string(),
                        ))
                    }
                };
                let new = match args.get(2).unwrap() {
                    Value::String(n) => n,
                    _ => {
                        return Err(RuntimeError::new(
                            "replace() third argument must be a string".to_string(),
                        ))
                    }
                };
                Ok(Value::String(Arc::from(s.replace(old.as_ref(), new.as_ref()))))
            }
            "str_trim" | "trim" => {
                if let Some(Value::String(s)) = args.first() {
                    Ok(Value::String(Arc::from(s.trim().to_string())))
                } else {
                    Err(RuntimeError::new("trim() requires a string".to_string()))
                }
            }
            "str_substring" | "substring" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(RuntimeError::new(
                        "substring() requires 2 or 3 arguments".to_string(),
                    ));
                }
                let s = match args.first().unwrap() {
                    Value::String(s) => s,
                    _ => {
                        return Err(RuntimeError::new(
                            "substring() first argument must be a string".to_string(),
                        ))
                    }
                };
                let char_count = s.chars().count();
                let start_i = match args.get(1).unwrap() {
                    Value::Integer(i) => *i,
                    _ => {
                        return Err(RuntimeError::new(
                            "substring() start must be an integer".to_string(),
                        ))
                    }
                };
                let end_i = if args.len() == 3 {
                    match args.get(2).unwrap() {
                        Value::Integer(i) => *i,
                        _ => {
                            return Err(RuntimeError::new(
                                "substring() end must be an integer".to_string(),
                            ))
                        }
                    }
                } else {
                    char_count as i64
                };
                if start_i < 0 || end_i < 0 {
                    return Err(RuntimeError::new(
                        "substring() indices must be non-negative".to_string(),
                    ));
                }
                let start = start_i as usize;
                let end = end_i as usize;
                if start > char_count || end > char_count || start > end {
                    return Err(RuntimeError::new(
                        "substring() indices out of bounds".to_string(),
                    ));
                }
                let result: String = s.chars().skip(start).take(end - start).collect();
                Ok(Value::String(Arc::from(result)))
            }
            "str_indexOf" | "indexOf" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "indexOf() requires 2 arguments".to_string(),
                    ));
                }
                let s = match args.first().unwrap() {
                    Value::String(s) => s,
                    _ => {
                        return Err(RuntimeError::new(
                            "indexOf() first argument must be a string".to_string(),
                        ))
                    }
                };
                let search = match args.get(1).unwrap() {
                    Value::String(ss) => ss,
                    _ => {
                        return Err(RuntimeError::new(
                            "indexOf() second argument must be a string".to_string(),
                        ))
                    }
                };
                match s.find(search.as_ref()) {
                    Some(idx) => Ok(Value::Integer(idx as i64)),
                    None => Ok(Value::Integer(-1)),
                }
            }
            "str_startsWith" | "startsWith" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "startsWith() requires 2 arguments".to_string(),
                    ));
                }
                let s = match args.first().unwrap() {
                    Value::String(s) => s,
                    _ => {
                        return Err(RuntimeError::new(
                            "startsWith() first argument must be a string".to_string(),
                        ))
                    }
                };
                let prefix = match args.get(1).unwrap() {
                    Value::String(p) => p,
                    _ => {
                        return Err(RuntimeError::new(
                            "startsWith() second argument must be a string".to_string(),
                        ))
                    }
                };
                Ok(Value::Boolean(s.starts_with(prefix.as_ref())))
            }
            "str_endsWith" | "endsWith" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "endsWith() requires 2 arguments".to_string(),
                    ));
                }
                let s = match args.first().unwrap() {
                    Value::String(s) => s,
                    _ => {
                        return Err(RuntimeError::new(
                            "endsWith() first argument must be a string".to_string(),
                        ))
                    }
                };
                let suffix = match args.get(1).unwrap() {
                    Value::String(s) => s,
                    _ => {
                        return Err(RuntimeError::new(
                            "endsWith() second argument must be a string".to_string(),
                        ))
                    }
                };
                Ok(Value::Boolean(s.ends_with(suffix.as_ref())))
            }
            "str_toUpper" | "toUpper" => {
                if let Some(Value::String(s)) = args.first() {
                    Ok(Value::String(Arc::from(s.to_uppercase())))
                } else {
                    Err(RuntimeError::new("toUpper() requires a string".to_string()))
                }
            }
            "str_toLower" | "toLower" => {
                if let Some(Value::String(s)) = args.first() {
                    Ok(Value::String(Arc::from(s.to_lowercase())))
                } else {
                    Err(RuntimeError::new("toLower() requires a string".to_string()))
                }
            }
            // Array functions
            "array_map" | "map" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "map() requires 2 arguments (array, function)".to_string(),
                    ));
                }
                let arr = match args.first().unwrap() {
                    Value::Array(a) => a,
                    _ => {
                        return Err(RuntimeError::new(
                            "map() first argument must be an array".to_string(),
                        ))
                    }
                };
                let func = args.get(1).unwrap();

                let executor = executor.ok_or_else(|| {
                    RuntimeError::new("map() requires function executor".to_string())
                })?;

                let mut result = Vec::new();
                for item in arr.iter() {
                    let call_args = vec![item.clone()];
                    let mapped_value = executor.call_function_value(func, &call_args)?;
                    result.push(mapped_value);
                }
                Ok(Value::Array(result))
            }
            "array_filter" | "filter" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "filter() requires 2 arguments (array, function)".to_string(),
                    ));
                }
                let arr = match args.first().unwrap() {
                    Value::Array(a) => a,
                    _ => {
                        return Err(RuntimeError::new(
                            "filter() first argument must be an array".to_string(),
                        ))
                    }
                };
                let func = args.get(1).unwrap();

                let executor = executor.ok_or_else(|| {
                    RuntimeError::new("filter() requires function executor".to_string())
                })?;

                let mut result = Vec::new();
                for item in arr.iter() {
                    let call_args = vec![item.clone()];
                    let predicate_result = executor.call_function_value(func, &call_args)?;
                    if let Value::Boolean(true) = predicate_result {
                        result.push(item.clone());
                    }
                }
                Ok(Value::Array(result))
            }
            "array_reduce" | "reduce" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(RuntimeError::new(
                        "reduce() requires 2 or 3 arguments".to_string(),
                    ));
                }
                let arr = match args.first().unwrap() {
                    Value::Array(a) => a,
                    _ => {
                        return Err(RuntimeError::new(
                            "reduce() first argument must be an array".to_string(),
                        ))
                    }
                };
                let func = args.get(1).unwrap();

                let executor = executor.ok_or_else(|| {
                    RuntimeError::new("reduce() requires function executor".to_string())
                })?;

                let mut accumulator = if args.len() == 3 {
                    args.get(2).unwrap().clone()
                } else if arr.is_empty() {
                    return Err(RuntimeError::new(
                        "reduce() on empty array requires initial value".to_string(),
                    ));
                } else {
                    arr[0].clone()
                };

                let start_index = if args.len() == 3 { 0 } else { 1 };
                for item in arr.iter().skip(start_index) {
                    let call_args = vec![accumulator.clone(), item.clone()];
                    accumulator = executor.call_function_value(func, &call_args)?;
                }
                Ok(accumulator)
            }
            "array_find" | "find" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "find() requires 2 arguments (array, function)".to_string(),
                    ));
                }
                let arr = match args.first().unwrap() {
                    Value::Array(a) => a,
                    _ => {
                        return Err(RuntimeError::new(
                            "find() first argument must be an array".to_string(),
                        ))
                    }
                };
                let func = args.get(1).unwrap();

                let executor = executor.ok_or_else(|| {
                    RuntimeError::new("find() requires function executor".to_string())
                })?;

                for item in arr.iter() {
                    let call_args = vec![item.clone()];
                    let predicate_result = executor.call_function_value(func, &call_args)?;
                    if let Value::Boolean(true) = predicate_result {
                        return Ok(item.clone());
                    }
                }
                Ok(Value::Null)
            }
            "array_sort" | "sort" => {
                if args.len() > 1 {
                    return Err(RuntimeError::new(
                        "sort() requires 0 or 1 argument".to_string(),
                    ));
                }
                if let Some(Value::Array(mut arr)) = args.first().cloned() {
                    // Simple numeric/string sort
                    arr.sort_by(|a, b| match (a, b) {
                        (Value::Integer(i1), Value::Integer(i2)) => i1.cmp(i2),
                        (Value::Float(f1), Value::Float(f2)) => {
                            f1.partial_cmp(f2).unwrap_or(std::cmp::Ordering::Equal)
                        }
                        (Value::String(s1), Value::String(s2)) => s1.cmp(s2),
                        _ => std::cmp::Ordering::Equal,
                    });
                    Ok(Value::Array(arr))
                } else {
                    Err(RuntimeError::new("sort() requires an array".to_string()))
                }
            }
            "array_reverse" | "reverse" => {
                if let Some(Value::Array(mut arr)) = args.first().cloned() {
                    arr.reverse();
                    Ok(Value::Array(arr))
                } else {
                    Err(RuntimeError::new("reverse() requires an array".to_string()))
                }
            }
            "array_concat" | "concat" => {
                if args.is_empty() {
                    return Err(RuntimeError::new(
                        "concat() requires at least 1 argument".to_string(),
                    ));
                }
                let mut result = Vec::new();
                for arg in args {
                    match arg {
                        Value::Array(arr) => result.extend(arr.clone()),
                        _ => result.push(arg.clone()),
                    }
                }
                Ok(Value::Array(result))
            }
            "array_slice" => {
                if args.is_empty() || args.len() > 3 {
                    return Err(RuntimeError::new(
                        "array_slice() requires 1-3 arguments".to_string(),
                    ));
                }
                let arr = match args.first().unwrap() {
                    Value::Array(a) => a,
                    _ => {
                        return Err(RuntimeError::new(
                            "array_slice() first argument must be an array".to_string(),
                        ))
                    }
                };
                let start = if args.len() >= 2 {
                    match args.get(1).unwrap() {
                        Value::Integer(i) => {
                            if *i < 0 {
                                let r = arr.len() as i64 + i;
                                if r < 0 {
                                    return Err(RuntimeError::new(format!(
                                        "array_slice() start index {} out of bounds for array of length {}",
                                        i, arr.len()
                                    )));
                                }
                                r as usize
                            } else {
                                *i as usize
                            }
                        }
                        _ => {
                            return Err(RuntimeError::new(
                                "array_slice() start must be an integer".to_string(),
                            ))
                        }
                    }
                } else {
                    0
                };
                let end = if args.len() == 3 {
                    match args.get(2).unwrap() {
                        Value::Integer(i) => {
                            if *i < 0 {
                                let r = arr.len() as i64 + i;
                                if r < 0 {
                                    return Err(RuntimeError::new(format!(
                                        "array_slice() end index {} out of bounds for array of length {}",
                                        i, arr.len()
                                    )));
                                }
                                r as usize
                            } else {
                                *i as usize
                            }
                        }
                        _ => {
                            return Err(RuntimeError::new(
                                "array_slice() end must be an integer".to_string(),
                            ))
                        }
                    }
                } else {
                    arr.len()
                };
                if start > arr.len() || end > arr.len() || start > end {
                    return Err(RuntimeError::new(
                        "array_slice() indices out of bounds".to_string(),
                    ));
                }
                Ok(Value::Array(arr[start..end].to_vec()))
            }
            "max" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new("max() requires 2 arguments".to_string()));
                }
                match (&args[0], &args[1]) {
                    (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(*a.max(b))),
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.max(*b))),
                    (Value::Integer(a), Value::Float(b)) => Ok(Value::Float((*a as f64).max(*b))),
                    (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a.max(*b as f64))),
                    _ => Err(RuntimeError::new("max() requires two numbers".to_string())),
                }
            }
            "min" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new("min() requires 2 arguments".to_string()));
                }
                match (&args[0], &args[1]) {
                    (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(*a.min(b))),
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.min(*b))),
                    (Value::Integer(a), Value::Float(b)) => Ok(Value::Float((*a as f64).min(*b))),
                    (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a.min(*b as f64))),
                    _ => Err(RuntimeError::new("min() requires two numbers".to_string())),
                }
            }
            // Result type constructors and accessors
            "ok" => {
                let inner = args.first().cloned().unwrap_or(Value::Null);
                Ok(Value::Result(true, Box::new(inner)))
            }
            "err" => {
                let inner = args.first().cloned().unwrap_or(Value::Null);
                Ok(Value::Result(false, Box::new(inner)))
            }
            "is_ok" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::new("is_ok() requires one argument".to_string())
                })?;
                Ok(Value::Boolean(matches!(val, Value::Result(true, _))))
            }
            "is_err" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::new("is_err() requires one argument".to_string())
                })?;
                Ok(Value::Boolean(matches!(val, Value::Result(false, _))))
            }
            "unwrap" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::new("unwrap() requires one argument".to_string())
                })?;
                match val {
                    Value::Result(true, inner) => Ok(*inner.clone()),
                    Value::Result(false, inner) => Err(RuntimeError::new(format!(
                        "unwrap() called on Err: {}",
                        inner
                    ))),
                    _ => Err(RuntimeError::new(
                        "unwrap() requires a Result value".to_string(),
                    )),
                }
            }
            "unwrap_or" => {
                if args.len() < 2 {
                    return Err(RuntimeError::new(
                        "unwrap_or() requires two arguments: result and default".to_string(),
                    ));
                }
                match &args[0] {
                    Value::Result(true, inner) => Ok(*inner.clone()),
                    Value::Result(false, _) => Ok(args[1].clone()),
                    _ => Err(RuntimeError::new(
                        "unwrap_or() requires a Result value as first argument".to_string(),
                    )),
                }
            }
            // Math extension functions
            "math_clamp" => {
                if args.len() != 3 {
                    return Err(RuntimeError::new(
                        "math_clamp requires 3 arguments (value, min, max)".to_string(),
                    ));
                }
                match (&args[0], &args[1], &args[2]) {
                    (Value::Float(v), Value::Float(mn), Value::Float(mx)) => {
                        Ok(Value::Float(v.max(*mn).min(*mx)))
                    }
                    (Value::Integer(v), Value::Integer(mn), Value::Integer(mx)) => {
                        Ok(Value::Integer((*v).max(*mn).min(*mx)))
                    }
                    (Value::Float(v), Value::Integer(mn), Value::Integer(mx)) => {
                        Ok(Value::Float(v.max(*mn as f64).min(*mx as f64)))
                    }
                    (Value::Integer(v), Value::Float(mn), Value::Float(mx)) => {
                        Ok(Value::Float((*v as f64).max(*mn).min(*mx)))
                    }
                    _ => Err(RuntimeError::new(
                        "math_clamp requires numeric arguments".to_string(),
                    )),
                }
            }

            "math_lerp" => {
                if args.len() != 3 {
                    return Err(RuntimeError::new(
                        "math_lerp requires 3 arguments (a, b, t)".to_string(),
                    ));
                }
                let a = match &args[0] {
                    Value::Float(f) => *f,
                    Value::Integer(i) => *i as f64,
                    _ => {
                        return Err(RuntimeError::new(
                            "math_lerp arguments must be numeric".to_string(),
                        ))
                    }
                };
                let b = match &args[1] {
                    Value::Float(f) => *f,
                    Value::Integer(i) => *i as f64,
                    _ => {
                        return Err(RuntimeError::new(
                            "math_lerp arguments must be numeric".to_string(),
                        ))
                    }
                };
                let t = match &args[2] {
                    Value::Float(f) => *f,
                    Value::Integer(i) => *i as f64,
                    _ => {
                        return Err(RuntimeError::new(
                            "math_lerp arguments must be numeric".to_string(),
                        ))
                    }
                };
                Ok(Value::Float(a + (b - a) * t))
            }

            "math_gcd" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "math_gcd requires 2 arguments (a, b)".to_string(),
                    ));
                }
                match (&args[0], &args[1]) {
                    (Value::Integer(a), Value::Integer(b)) => {
                        fn gcd(mut a: i64, mut b: i64) -> i64 {
                            a = a.abs();
                            b = b.abs();
                            while b != 0 {
                                let t = b;
                                b = a % b;
                                a = t;
                            }
                            a
                        }
                        Ok(Value::Integer(gcd(*a, *b)))
                    }
                    _ => Err(RuntimeError::new(
                        "math_gcd requires integer arguments".to_string(),
                    )),
                }
            }

            "math_lcm" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "math_lcm requires 2 arguments (a, b)".to_string(),
                    ));
                }
                match (&args[0], &args[1]) {
                    (Value::Integer(a), Value::Integer(b)) => {
                        fn gcd(mut a: i64, mut b: i64) -> i64 {
                            a = a.abs();
                            b = b.abs();
                            while b != 0 {
                                let t = b;
                                b = a % b;
                                a = t;
                            }
                            a
                        }
                        let g = gcd(*a, *b);
                        if g == 0 {
                            return Ok(Value::Integer(0));
                        }
                        Ok(Value::Integer((a * b).abs() / g))
                    }
                    _ => Err(RuntimeError::new(
                        "math_lcm requires integer arguments".to_string(),
                    )),
                }
            }

            "math_factorial" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "math_factorial requires 1 argument (n)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::Integer(n) => {
                        if *n < 0 {
                            return Err(RuntimeError::new(
                                "math_factorial requires non-negative integer".to_string(),
                            ));
                        }
                        if *n > 20 {
                            return Err(RuntimeError::new(
                                "math_factorial: n too large (max 20 for i64)".to_string(),
                            ));
                        }
                        let mut result: i64 = 1;
                        for i in 2..=*n {
                            result *= i;
                        }
                        Ok(Value::Integer(result))
                    }
                    _ => Err(RuntimeError::new(
                        "math_factorial requires an integer argument".to_string(),
                    )),
                }
            }

            "math_combinations" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "math_combinations requires 2 arguments (n, k)".to_string(),
                    ));
                }
                match (&args[0], &args[1]) {
                    (Value::Integer(n), Value::Integer(k)) => {
                        if *n < 0 || *k < 0 {
                            return Err(RuntimeError::new(
                                "math_combinations requires non-negative integers".to_string(),
                            ));
                        }
                        if *k > *n {
                            return Ok(Value::Integer(0));
                        }
                        let k = (*k).min(*n - *k); // C(n,k) = C(n,n-k)
                        let mut result: i64 = 1;
                        for i in 0..k {
                            result = result
                                .checked_mul(*n - i)
                                .and_then(|v| v.checked_div(i + 1))
                                .ok_or_else(|| {
                                    RuntimeError::new("math_combinations: overflow".to_string())
                                })?;
                        }
                        Ok(Value::Integer(result))
                    }
                    _ => Err(RuntimeError::new(
                        "math_combinations requires integer arguments".to_string(),
                    )),
                }
            }

            "math_random_int" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "math_random_int requires 2 arguments (min, max)".to_string(),
                    ));
                }
                match (&args[0], &args[1]) {
                    (Value::Integer(min), Value::Integer(max)) => {
                        if min > max {
                            return Err(RuntimeError::new(format!(
                                "math_random_int: min ({}) must be <= max ({})",
                                min, max
                            )));
                        }
                        use rand::Rng;
                        let seed = executor.as_ref().and_then(|e| e.deterministic_random_seed());
                        if let Some(s) = seed {
                            use rand::{SeedableRng, rngs::StdRng};
                            let mut rng = StdRng::seed_from_u64(s);
                            Ok(Value::Integer(rng.gen_range(*min..=*max)))
                        } else {
                            let mut rng = rand::thread_rng();
                            Ok(Value::Integer(rng.gen_range(*min..=*max)))
                        }
                    }
                    _ => Err(RuntimeError::new(
                        "math_random_int requires integer arguments".to_string(),
                    )),
                }
            }

            "math_random_float" => {
                use rand::Rng;
                let seed = executor.as_ref().and_then(|e| e.deterministic_random_seed());
                if let Some(s) = seed {
                    use rand::{SeedableRng, rngs::StdRng};
                    let mut rng = StdRng::seed_from_u64(s);
                    Ok(Value::Float(rng.gen::<f64>()))
                } else {
                    let mut rng = rand::thread_rng();
                    Ok(Value::Float(rng.gen::<f64>()))
                }
            }

            // String padding and formatting
            "str_pad_left" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(RuntimeError::new(
                        "str_pad_left requires 2-3 arguments (str, width, pad_char?)".to_string(),
                    ));
                }
                let s = match &args[0] {
                    Value::String(s) => s.to_string(),
                    _ => {
                        return Err(RuntimeError::new(
                            "str_pad_left requires string as first argument".to_string(),
                        ))
                    }
                };
                let width = match &args[1] {
                    Value::Integer(n) => {
                        if *n < 0 {
                            return Err(RuntimeError::new(
                                "str_pad_left width must be non-negative".to_string(),
                            ));
                        }
                        *n as usize
                    }
                    _ => {
                        return Err(RuntimeError::new(
                            "str_pad_left width must be integer".to_string(),
                        ))
                    }
                };
                let pad_char = if args.len() == 3 {
                    match &args[2] {
                        Value::String(c) => c.chars().next().unwrap_or(' '),
                        _ => ' ',
                    }
                } else {
                    ' '
                };
                let s_len = s.chars().count();
                if s_len >= width {
                    return Ok(Value::String(Arc::from(s)));
                }
                let padding: String = std::iter::repeat_n(pad_char, width - s_len).collect();
                Ok(Value::String(Arc::from(format!("{}{}", padding, s))))
            }

            "str_pad_right" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(RuntimeError::new(
                        "str_pad_right requires 2-3 arguments (str, width, pad_char?)".to_string(),
                    ));
                }
                let s = match &args[0] {
                    Value::String(s) => s.to_string(),
                    _ => {
                        return Err(RuntimeError::new(
                            "str_pad_right requires string as first argument".to_string(),
                        ))
                    }
                };
                let width = match &args[1] {
                    Value::Integer(n) => {
                        if *n < 0 {
                            return Err(RuntimeError::new(
                                "str_pad_right width must be non-negative".to_string(),
                            ));
                        }
                        *n as usize
                    }
                    _ => {
                        return Err(RuntimeError::new(
                            "str_pad_right width must be integer".to_string(),
                        ))
                    }
                };
                let pad_char = if args.len() == 3 {
                    match &args[2] {
                        Value::String(c) => c.chars().next().unwrap_or(' '),
                        _ => ' ',
                    }
                } else {
                    ' '
                };
                let s_len = s.chars().count();
                if s_len >= width {
                    return Ok(Value::String(Arc::from(s)));
                }
                let padding: String = std::iter::repeat_n(pad_char, width - s_len).collect();
                Ok(Value::String(Arc::from(format!("{}{}", s, padding))))
            }

            "str_wrap" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "str_wrap requires 2 arguments (str, width)".to_string(),
                    ));
                }
                let text = match &args[0] {
                    Value::String(s) => s.to_string(),
                    _ => return Err(RuntimeError::new("str_wrap requires a string".to_string())),
                };
                let width = match &args[1] {
                    Value::Integer(n) => *n as usize,
                    _ => {
                        return Err(RuntimeError::new(
                            "str_wrap width must be integer".to_string(),
                        ))
                    }
                };
                if width == 0 {
                    return Err(RuntimeError::new("str_wrap width must be > 0".to_string()));
                }
                let mut lines = Vec::new();
                let mut current_line = String::new();
                let mut current_len = 0usize;
                for word in text.split_whitespace() {
                    let word_len = word.chars().count();
                    if current_len == 0 {
                        current_line.push_str(word);
                        current_len = word_len;
                    } else if current_len + 1 + word_len <= width {
                        current_line.push(' ');
                        current_line.push_str(word);
                        current_len += 1 + word_len;
                    } else {
                        lines.push(current_line.clone());
                        current_line = word.to_string();
                        current_len = word_len;
                    }
                }
                if !current_line.is_empty() {
                    lines.push(current_line);
                }
                Ok(Value::String(Arc::from(lines.join("\n"))))
            }

            "str_dedent" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "str_dedent requires 1 argument (str)".to_string(),
                    ));
                }
                let text = match &args[0] {
                    Value::String(s) => s.to_string(),
                    _ => {
                        return Err(RuntimeError::new(
                            "str_dedent requires a string".to_string(),
                        ))
                    }
                };
                let lines: Vec<&str> = text.lines().collect();
                let min_indent = lines
                    .iter()
                    .filter(|l| !l.trim().is_empty())
                    .map(|l| l.len() - l.trim_start().len())
                    .min()
                    .unwrap_or(0);
                let dedented: Vec<&str> = lines
                    .iter()
                    .map(|l| {
                        if l.len() >= min_indent {
                            &l[min_indent..]
                        } else {
                            l.trim_start()
                        }
                    })
                    .collect();
                Ok(Value::String(Arc::from(dedented.join("\n"))))
            }

            "str_count" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "str_count requires 2 arguments (str, substr)".to_string(),
                    ));
                }
                match (&args[0], &args[1]) {
                    (Value::String(s), Value::String(sub)) => {
                        if sub.is_empty() {
                            return Ok(Value::Integer(0));
                        }
                        let count = s.matches(sub.as_ref()).count();
                        Ok(Value::Integer(count as i64))
                    }
                    _ => Err(RuntimeError::new(
                        "str_count requires string arguments".to_string(),
                    )),
                }
            }

            // Base32 encoding/decoding
            "base32_encode" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "base32_encode requires 1 argument (str)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(s) => {
                        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
                        let input = s.as_bytes();
                        let mut output = String::new();
                        let mut i = 0;
                        while i < input.len() {
                            let b0 = input[i] as u32;
                            let b1 = if i + 1 < input.len() {
                                input[i + 1] as u32
                            } else {
                                0
                            };
                            let b2 = if i + 2 < input.len() {
                                input[i + 2] as u32
                            } else {
                                0
                            };
                            let b3 = if i + 3 < input.len() {
                                input[i + 3] as u32
                            } else {
                                0
                            };
                            let b4 = if i + 4 < input.len() {
                                input[i + 4] as u32
                            } else {
                                0
                            };
                            output.push(ALPHABET[((b0 >> 3) & 0x1f) as usize] as char);
                            output
                                .push(ALPHABET[(((b0 << 2) | (b1 >> 6)) & 0x1f) as usize] as char);
                            if i + 1 < input.len() {
                                output.push(ALPHABET[((b1 >> 1) & 0x1f) as usize] as char);
                            } else {
                                output.push('=');
                            }
                            if i + 1 < input.len() {
                                output.push(
                                    ALPHABET[(((b1 << 4) | (b2 >> 4)) & 0x1f) as usize] as char,
                                );
                            } else {
                                output.push('=');
                            }
                            if i + 2 < input.len() {
                                output.push(
                                    ALPHABET[(((b2 << 1) | (b3 >> 7)) & 0x1f) as usize] as char,
                                );
                            } else {
                                output.push('=');
                            }
                            if i + 3 < input.len() {
                                output.push(ALPHABET[((b3 >> 2) & 0x1f) as usize] as char);
                            } else {
                                output.push('=');
                            }
                            if i + 3 < input.len() {
                                output.push(
                                    ALPHABET[(((b3 << 3) | (b4 >> 5)) & 0x1f) as usize] as char,
                                );
                            } else {
                                output.push('=');
                            }
                            if i + 4 < input.len() {
                                output.push(ALPHABET[(b4 & 0x1f) as usize] as char);
                            } else {
                                output.push('=');
                            }
                            i += 5;
                        }
                        Ok(Value::String(Arc::from(output)))
                    }
                    _ => Err(RuntimeError::new(
                        "base32_encode requires a string".to_string(),
                    )),
                }
            }

            "base32_decode" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "base32_decode requires 1 argument (str)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(s) => {
                        let s_upper = s.to_uppercase();
                        let s_trimmed = s_upper.trim_end_matches('=');
                        let decode_char = |c: u8| -> Option<u8> {
                            match c {
                                b'A'..=b'Z' => Some(c - b'A'),
                                b'2'..=b'7' => Some(c - b'2' + 26),
                                _ => None,
                            }
                        };
                        let bits: Vec<u8> = s_trimmed.bytes().filter_map(decode_char).collect();
                        let mut output = Vec::new();
                        let mut i = 0;
                        while i + 1 < bits.len() {
                            let b0 = bits[i];
                            let b1 = bits[i + 1];
                            output.push((b0 << 3) | (b1 >> 2));
                            if i + 2 < bits.len() && i + 3 < bits.len() {
                                let b2 = bits[i + 2];
                                let b3 = bits[i + 3];
                                output.push((b1 << 6) | (b2 << 1) | (b3 >> 4));
                                if i + 4 < bits.len() {
                                    let b4 = bits[i + 4];
                                    output.push((b3 << 4) | (b4 >> 1));
                                    if i + 5 < bits.len() && i + 6 < bits.len() {
                                        let b5 = bits[i + 5];
                                        let b6 = bits[i + 6];
                                        output.push((b4 << 7) | (b5 << 2) | (b6 >> 3));
                                        if i + 7 < bits.len() {
                                            let b7 = bits[i + 7];
                                            output.push((b6 << 5) | b7);
                                        }
                                    }
                                }
                            }
                            i += 8;
                        }
                        match String::from_utf8(output) {
                            Ok(s) => Ok(Value::String(Arc::from(s))),
                            Err(_) => Err(RuntimeError::new(
                                "base32_decode produced invalid UTF-8".to_string(),
                            )),
                        }
                    }
                    _ => Err(RuntimeError::new(
                        "base32_decode requires a string".to_string(),
                    )),
                }
            }

            // HTML escaping
            "html_escape" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "html_escape requires 1 argument (str)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(s) => {
                        let escaped = s
                            .replace('&', "&amp;")
                            .replace('<', "&lt;")
                            .replace('>', "&gt;")
                            .replace('"', "&quot;")
                            .replace('\'', "&#39;");
                        Ok(Value::String(Arc::from(escaped)))
                    }
                    _ => Err(RuntimeError::new(
                        "html_escape requires a string".to_string(),
                    )),
                }
            }

            // TOML encode/decode
            "toml_encode" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "toml_encode requires 1 argument (value)".to_string(),
                    ));
                }
                fn to_toml_value(v: &Value) -> Result<toml::Value, RuntimeError> {
                    match v {
                        Value::Null => Ok(toml::Value::String("null".to_string())),
                        Value::Boolean(b) => Ok(toml::Value::Boolean(*b)),
                        Value::Integer(i) => Ok(toml::Value::Integer(*i)),
                        Value::Float(f) => Ok(toml::Value::Float(*f)),
                        Value::String(s) => Ok(toml::Value::String(s.to_string())),
                        Value::Array(arr) => {
                            let items: Result<Vec<_>, _> = arr.iter().map(to_toml_value).collect();
                            Ok(toml::Value::Array(items?))
                        }
                        Value::Map(m) => {
                            let mut table = toml::value::Table::new();
                            for (k, v) in m {
                                table.insert(k.clone(), to_toml_value(v)?);
                            }
                            Ok(toml::Value::Table(table))
                        }
                        _ => Err(RuntimeError::new(
                            "toml_encode: unsupported value type".to_string(),
                        )),
                    }
                }
                let toml_val = to_toml_value(&args[0])?;
                match toml::to_string(&toml_val) {
                    Ok(s) => Ok(Value::String(Arc::from(s))),
                    Err(e) => Err(RuntimeError::new(format!("toml_encode failed: {}", e))),
                }
            }

            "toml_decode" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "toml_decode requires 1 argument (str)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(s) => {
                        let toml_val: toml::Value = toml::from_str(s)
                            .map_err(|e| RuntimeError::new(format!("toml_decode failed: {}", e)))?;
                        fn from_toml_value(v: toml::Value) -> Value {
                            match v {
                                toml::Value::Boolean(b) => Value::Boolean(b),
                                toml::Value::Integer(i) => Value::Integer(i),
                                toml::Value::Float(f) => Value::Float(f),
                                toml::Value::String(s) => Value::String(Arc::from(s)),
                                toml::Value::Array(arr) => {
                                    Value::Array(arr.into_iter().map(from_toml_value).collect())
                                }
                                toml::Value::Table(t) => Value::Map(
                                    t.into_iter()
                                        .map(|(k, v)| (k, from_toml_value(v)))
                                        .collect::<indexmap::IndexMap<_, _>>(),
                                ),
                                toml::Value::Datetime(dt) => Value::String(Arc::from(dt.to_string())),
                            }
                        }
                        Ok(from_toml_value(toml_val))
                    }
                    _ => Err(RuntimeError::new(
                        "toml_decode requires a string".to_string(),
                    )),
                }
            }

            // Task 17.2: parse/stringify aliases for consistency with json_parse/json_stringify
            "toml_parse"      => Self::call_function("toml_decode", args, executor),
            "toml_stringify"  => Self::call_function("toml_encode", args, executor),
            "yaml_parse"      => Self::call_function("yaml_decode", args, executor),
            "yaml_stringify"  => Self::call_function("yaml_encode", args, executor),

            // CSV encode/decode
            "csv_to_string" | "csv_encode" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "csv_encode requires 1 argument (rows: array of arrays)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::Array(rows) => {
                        let mut output = String::new();
                        for row in rows {
                            match row {
                                Value::Array(fields) => {
                                    let line: Vec<String> = fields
                                        .iter()
                                        .map(|f| {
                                            let s: String = match f {
                                                Value::String(s) => s.to_string(),
                                                Value::Integer(i) => i.to_string(),
                                                Value::Float(fl) => fl.to_string(),
                                                Value::Boolean(b) => b.to_string(),
                                                Value::Null => "".to_string(),
                                                _ => f.to_string(),
                                            };
                                            if s.contains(',')
                                                || s.contains('"')
                                                || s.contains('\n')
                                            {
                                                format!("\"{}\"", s.replace('"', "\"\""))
                                            } else {
                                                s
                                            }
                                        })
                                        .collect();
                                    output.push_str(&line.join(","));
                                    output.push('\n');
                                }
                                _ => {
                                    return Err(RuntimeError::new(
                                        "csv_encode: each row must be an array".to_string(),
                                    ))
                                }
                            }
                        }
                        Ok(Value::String(Arc::from(output)))
                    }
                    _ => Err(RuntimeError::new(
                        "csv_encode requires an array of arrays".to_string(),
                    )),
                }
            }

            "csv_decode" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "csv_decode requires 1 argument (str)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(s) => {
                        let mut rows: Vec<Value> = Vec::new();
                        for line in s.lines() {
                            if line.trim().is_empty() {
                                continue;
                            }
                            let mut fields: Vec<Value> = Vec::new();
                            let mut field = String::new();
                            let mut in_quotes = false;
                            let mut chars = line.chars().peekable();
                            while let Some(c) = chars.next() {
                                match c {
                                    '"' if !in_quotes => {
                                        in_quotes = true;
                                    }
                                    '"' if in_quotes => {
                                        if chars.peek() == Some(&'"') {
                                            chars.next();
                                            field.push('"');
                                        } else {
                                            in_quotes = false;
                                        }
                                    }
                                    ',' if !in_quotes => {
                                        fields.push(Value::String(Arc::from(field.clone())));
                                        field.clear();
                                    }
                                    _ => {
                                        field.push(c);
                                    }
                                }
                            }
                            fields.push(Value::String(Arc::from(field)));
                            rows.push(Value::Array(fields));
                        }
                        Ok(Value::Array(rows))
                    }
                    _ => Err(RuntimeError::new(
                        "csv_decode requires a string".to_string(),
                    )),
                }
            }

            // xml_decode is the canonical name; xml_parse is kept as a legacy alias.
            "xml_decode" | "xml_parse" => {
                #[cfg(not(feature = "stdlib-full"))]
                return Err(RuntimeError::new(
                    "xml_decode requires the 'stdlib-full' feature. \
                     Rebuild with: cargo build --features stdlib-full"
                        .to_string(),
                ));
                #[cfg(feature = "stdlib-full")]
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "xml_decode requires 1 argument (str)".to_string(),
                    ));
                }
                #[cfg(feature = "stdlib-full")]
                match &args[0] {
                    Value::String(s) => {
                        use quick_xml::events::Event;
                        use quick_xml::Reader;
                        let mut reader = Reader::from_str(s);
                        reader.trim_text(true);
                        let mut stack: Vec<(String, indexmap::IndexMap<String, Value>)> =
                            Vec::new();
                        let mut root: Option<Value> = None;
                        let mut buf = Vec::new();
                        loop {
                            match reader.read_event_into(&mut buf) {
                                Ok(Event::Start(e)) => {
                                    let tag = std::str::from_utf8(e.name().as_ref())
                                        .unwrap_or("unknown")
                                        .to_string();
                                    let mut node: indexmap::IndexMap<String, Value> =
                                        indexmap::IndexMap::new();
                                    node.insert("_tag".to_string(), Value::String(Arc::from(tag.clone())));
                                    for attr in e.attributes().flatten() {
                                        let key = std::str::from_utf8(attr.key.as_ref())
                                            .unwrap_or("")
                                            .to_string();
                                        let val = std::str::from_utf8(&attr.value)
                                            .unwrap_or("")
                                            .to_string();
                                        node.insert(key, Value::String(Arc::from(val)));
                                    }
                                    stack.push((tag, node));
                                }
                                Ok(Event::Text(e)) => {
                                    let text = e.unescape().unwrap_or_default().to_string();
                                    if !text.trim().is_empty() {
                                        if let Some((_, node)) = stack.last_mut() {
                                            node.insert("_text".to_string(), Value::String(Arc::from(text)));
                                        }
                                    }
                                }
                                Ok(Event::End(_)) => {
                                    if let Some((_, node)) = stack.pop() {
                                        let val = Value::Map(node);
                                        if let Some((_, parent)) = stack.last_mut() {
                                            let tag = if let Value::Map(ref m) = val {
                                                m.get("_tag")
                                                    .and_then(|v| {
                                                        if let Value::String(s) = v {
                                                            Some(s.to_string())
                                                        } else {
                                                            None
                                                        }
                                                    })
                                                    .map(|s| s.to_string()).unwrap_or("child".to_string())
                                            } else {
                                                "child".to_string()
                                            };
                                            let children = parent
                                                .entry("_children".to_string())
                                                .or_insert_with(|| Value::Array(Vec::new()));
                                            if let Value::Array(arr) = children {
                                                arr.push(val);
                                            }
                                            let _ = tag;
                                        } else {
                                            root = Some(val);
                                        }
                                    }
                                }
                                Ok(Event::Eof) => break,
                                Err(e) => {
                                    return Err(RuntimeError::new(format!(
                                        "xml_decode error: {}",
                                        e
                                    )))
                                }
                                _ => {}
                            }
                            buf.clear();
                        }
                        Ok(root.unwrap_or(Value::Null))
                    }
                    _ => Err(RuntimeError::new("xml_decode requires a string".to_string())),
                }
            }

            // xml_stringify: convert a Txtcode Value (map/array) back to an XML string.
            // Map keys: "_tag" → element name, "_text" → text content, others → attributes.
            // Arrays → repeated sibling elements (use "_tag" in each element).
            "xml_stringify" | "xml_encode" => {
                #[cfg(not(feature = "stdlib-full"))]
                return Err(RuntimeError::new(
                    "xml_stringify requires the 'stdlib-full' feature. \
                     Rebuild with: cargo build --features stdlib-full"
                        .to_string(),
                ));
                #[cfg(feature = "stdlib-full")]
                {
                    if args.len() != 1 {
                        return Err(RuntimeError::new(
                            "xml_stringify requires 1 argument".to_string(),
                        ));
                    }
                    fn value_to_xml(v: &Value) -> String {
                        match v {
                            Value::Map(m) => {
                                let tag: String = match m.get("_tag") {
                                    Some(Value::String(t)) => t.to_string(),
                                    _ => "element".to_string(),
                                };
                                let text: String = match m.get("_text") {
                                    Some(Value::String(t)) => t.to_string(),
                                    _ => String::new(),
                                };
                                let attrs: String = m
                                    .iter()
                                    .filter(|(k, _)| k.as_str() != "_tag" && k.as_str() != "_text" && k.as_str() != "_children")
                                    .map(|(k, v)| {
                                        let val: String = match v {
                                            Value::String(s) => s.to_string(),
                                            other => format!("{:?}", other),
                                        };
                                        // Escape XML attribute values
                                        let escaped = val
                                            .replace('&', "&amp;")
                                            .replace('"', "&quot;")
                                            .replace('<', "&lt;")
                                            .replace('>', "&gt;");
                                        format!(" {}=\"{}\"", k, escaped)
                                    })
                                    .collect();
                                let children = match m.get("_children") {
                                    Some(Value::Array(arr)) => arr
                                        .iter()
                                        .map(value_to_xml)
                                        .collect::<Vec<_>>()
                                        .join(""),
                                    _ => String::new(),
                                };
                                let inner = if children.is_empty() {
                                    
                                    text
                                        .replace('&', "&amp;")
                                        .replace('<', "&lt;")
                                        .replace('>', "&gt;")
                                } else {
                                    children
                                };
                                if inner.is_empty() {
                                    format!("<{}{}/>", tag, attrs)
                                } else {
                                    format!("<{}{}>{}</{}>", tag, attrs, inner, tag)
                                }
                            }
                            Value::Array(arr) => arr
                                .iter()
                                .map(value_to_xml)
                                .collect::<Vec<_>>()
                                .join(""),
                            Value::String(s) => {
                                s.replace('&', "&amp;")
                                    .replace('<', "&lt;")
                                    .replace('>', "&gt;")
                            }
                            Value::Integer(n) => n.to_string(),
                            Value::Float(f) => f.to_string(),
                            Value::Boolean(b) => b.to_string(),
                            Value::Null => String::new(),
                            _ => String::new(),
                        }
                    }
                    Ok(Value::String(Arc::from(value_to_xml(&args[0]))))
                }
            }

            "yaml_encode" => {
                #[cfg(not(feature = "stdlib-full"))]
                return Err(RuntimeError::new(
                    "yaml_encode requires the 'stdlib-full' feature. \
                     Rebuild with: cargo build --features stdlib-full"
                        .to_string(),
                ));
                #[cfg(feature = "stdlib-full")]
                {
                    if args.len() != 1 {
                        return Err(RuntimeError::new(
                            "yaml_encode requires 1 argument".to_string(),
                        ));
                    }
                    fn value_to_yaml(v: &Value) -> serde_yaml::Value {
                        match v {
                            Value::Null => serde_yaml::Value::Null,
                            Value::Boolean(b) => serde_yaml::Value::Bool(*b),
                            Value::Integer(n) => serde_yaml::Value::Number((*n).into()),
                            Value::Float(f) => {
                                serde_yaml::Value::Number(serde_yaml::Number::from(*f))
                            }
                            Value::String(s) => serde_yaml::Value::String(s.to_string()),
                            Value::Array(arr) => serde_yaml::Value::Sequence(
                                arr.iter().map(value_to_yaml).collect(),
                            ),
                            Value::Map(map) => {
                                let mut m = serde_yaml::Mapping::new();
                                for (k, v) in map {
                                    m.insert(
                                        serde_yaml::Value::String(k.to_string()),
                                        value_to_yaml(v),
                                    );
                                }
                                serde_yaml::Value::Mapping(m)
                            }
                            other => serde_yaml::Value::String(other.to_string()),
                        }
                    }
                    let yaml_val = value_to_yaml(&args[0]);
                    serde_yaml::to_string(&yaml_val)
                        .map(|s| Value::String(Arc::from(s)))
                        .map_err(|e| RuntimeError::new(format!("yaml_encode error: {}", e)))
                }
            }
            "yaml_decode" => {
                #[cfg(not(feature = "stdlib-full"))]
                return Err(RuntimeError::new(
                    "yaml_decode requires the 'stdlib-full' feature. \
                     Rebuild with: cargo build --features stdlib-full"
                        .to_string(),
                ));
                #[cfg(feature = "stdlib-full")]
                {
                    if args.len() != 1 {
                        return Err(RuntimeError::new(
                            "yaml_decode requires 1 argument (str)".to_string(),
                        ));
                    }
                    match &args[0] {
                        Value::String(s) => {
                            fn yaml_to_value(y: serde_yaml::Value) -> Value {
                                match y {
                                    serde_yaml::Value::Null => Value::Null,
                                    serde_yaml::Value::Bool(b) => Value::Boolean(b),
                                    serde_yaml::Value::Number(n) => {
                                        if let Some(i) = n.as_i64() {
                                            Value::Integer(i)
                                        } else if let Some(f) = n.as_f64() {
                                            Value::Float(f)
                                        } else {
                                            Value::String(Arc::from(n.to_string()))
                                        }
                                    }
                                    serde_yaml::Value::String(s) => Value::String(Arc::from(s)),
                                    serde_yaml::Value::Sequence(seq) => {
                                        Value::Array(seq.into_iter().map(yaml_to_value).collect())
                                    }
                                    serde_yaml::Value::Mapping(map) => {
                                        let mut m = indexmap::IndexMap::new();
                                        for (k, v) in map {
                                            let key = match k {
                                                serde_yaml::Value::String(s) => s,
                                                other => format!("{:?}", other),
                                            };
                                            m.insert(key, yaml_to_value(v));
                                        }
                                        Value::Map(m)
                                    }
                                    serde_yaml::Value::Tagged(t) => yaml_to_value(t.value),
                                }
                            }
                            let yaml_val: serde_yaml::Value = serde_yaml::from_str(s)
                                .map_err(|e| {
                                    RuntimeError::new(format!("yaml_decode error: {}", e))
                                })?;
                            Ok(yaml_to_value(yaml_val))
                        }
                        _ => Err(RuntimeError::new(
                            "yaml_decode requires a string argument".to_string(),
                        )),
                    }
                }
            }

            "int" => match args.first() {
                Some(Value::Integer(i)) => Ok(Value::Integer(*i)),
                Some(Value::Float(f)) => Ok(Value::Integer(*f as i64)),
                Some(Value::Boolean(b)) => Ok(Value::Integer(if *b { 1 } else { 0 })),
                Some(Value::String(s)) => s.trim().parse::<i64>().map(Value::Integer).map_err(|_| {
                    RuntimeError::new(format!("Cannot convert string to int: {:?}", s))
                }),
                _ => Err(RuntimeError::new("int() requires one argument".to_string())),
            },
            "float" => match args.first() {
                Some(Value::Float(f)) => Ok(Value::Float(*f)),
                Some(Value::Integer(i)) => Ok(Value::Float(*i as f64)),
                Some(Value::Boolean(b)) => Ok(Value::Float(if *b { 1.0 } else { 0.0 })),
                Some(Value::String(s)) => s.trim().parse::<f64>().map(Value::Float).map_err(|_| {
                    RuntimeError::new(format!("Cannot convert string to float: {:?}", s))
                }),
                _ => Err(RuntimeError::new("float() requires one argument".to_string())),
            },
            "string" => match args.first() {
                Some(v) => Ok(Value::String(Arc::from(v.to_string()))),
                None => Err(RuntimeError::new("string() requires one argument".to_string())),
            },
            "bool" => match args.first() {
                Some(Value::Boolean(b)) => Ok(Value::Boolean(*b)),
                Some(Value::Integer(i)) => Ok(Value::Boolean(*i != 0)),
                Some(Value::Null) => Ok(Value::Boolean(false)),
                Some(Value::String(s)) => Ok(Value::Boolean(!s.is_empty())),
                Some(_) => Ok(Value::Boolean(true)),
                None => Err(RuntimeError::new("bool() requires one argument".to_string())),
            },
            // ── str_format(template, arg0, arg1, ...) ────────────────────────
            // Supports `{}` (sequential) and `{N}` (positional) placeholders.
            "str_format" | "format" => {
                if args.is_empty() {
                    return Err(RuntimeError::new(
                        "str_format requires at least one argument (template)".to_string(),
                    ));
                }
                let template = match &args[0] {
                    Value::String(s) => s.to_string(),
                    _ => {
                        return Err(RuntimeError::new(
                            "str_format: template (arg 1) must be a string".to_string(),
                        ))
                    }
                };
                let fmt_args: Vec<String> = args[1..].iter().map(|v| v.to_string()).collect();
                let mut result = String::new();
                let mut chars = template.chars().peekable();
                let mut seq_idx = 0usize;
                while let Some(c) = chars.next() {
                    if c == '{' {
                        match chars.peek() {
                            Some('}') => {
                                chars.next();
                                let s = fmt_args.get(seq_idx).map(|s| s.as_ref()).unwrap_or("");
                                result.push_str(s);
                                seq_idx += 1;
                            }
                            Some(&d) if d.is_ascii_digit() => {
                                let mut num_str = String::new();
                                while chars.peek().is_some_and(|c| c.is_ascii_digit()) {
                                    num_str.push(chars.next().unwrap());
                                }
                                if chars.peek() == Some(&'}') {
                                    chars.next();
                                    let idx: usize = num_str.parse().unwrap_or(0);
                                    let s = fmt_args.get(idx).map(|s| s.as_ref()).unwrap_or("");
                                    result.push_str(s);
                                } else {
                                    result.push('{');
                                    result.push_str(&num_str);
                                }
                            }
                            _ => result.push(c),
                        }
                    } else {
                        result.push(c);
                    }
                }
                Ok(Value::String(Arc::from(result)))
            }

            // ── str_repeat(s, n) ──────────────────────────────────────────────
            "str_repeat" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "str_repeat requires 2 arguments (str, count)".to_string(),
                    ));
                }
                let s = match &args[0] {
                    Value::String(s) => s.to_string(),
                    _ => {
                        return Err(RuntimeError::new(
                            "str_repeat: first argument must be a string".to_string(),
                        ))
                    }
                };
                let n = match &args[1] {
                    Value::Integer(i) => *i,
                    _ => {
                        return Err(RuntimeError::new(
                            "str_repeat: second argument must be an integer".to_string(),
                        ))
                    }
                };
                if n < 0 {
                    return Err(RuntimeError::new(
                        "str_repeat: count must be non-negative".to_string(),
                    ));
                }
                Ok(Value::String(Arc::from(s.repeat(n as usize))))
            }

            // ── str_contains(s, substr) ───────────────────────────────────────
            "str_contains" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "str_contains requires 2 arguments (str, substr)".to_string(),
                    ));
                }
                match (&args[0], &args[1]) {
                    (Value::String(s), Value::String(sub)) => {
                        Ok(Value::Boolean(s.contains(sub.as_ref())))
                    }
                    _ => Err(RuntimeError::new(
                        "str_contains requires string arguments".to_string(),
                    )),
                }
            }

            // ── str_chars(s) — split into single-character array ──────────────
            "str_chars" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "str_chars requires 1 argument (str)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(s) => Ok(Value::Array(
                        s.chars().map(|c| Value::String(Arc::from(c.to_string()))).collect(),
                    )),
                    _ => Err(RuntimeError::new(
                        "str_chars requires a string argument".to_string(),
                    )),
                }
            }

            // ── str_reverse(s) ────────────────────────────────────────────────
            "str_reverse" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "str_reverse requires 1 argument (str)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(s) => Ok(Value::String(Arc::from(s.chars().rev().collect::<String>()))),
                    _ => Err(RuntimeError::new(
                        "str_reverse requires a string argument".to_string(),
                    )),
                }
            }

            // ── str_center(s, width, pad_char?) ───────────────────────────────
            "str_center" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(RuntimeError::new(
                        "str_center requires 2-3 arguments (str, width, pad_char?)".to_string(),
                    ));
                }
                let s = match &args[0] {
                    Value::String(s) => s.to_string(),
                    _ => {
                        return Err(RuntimeError::new(
                            "str_center: first argument must be a string".to_string(),
                        ))
                    }
                };
                let width = match &args[1] {
                    Value::Integer(i) => *i as usize,
                    _ => {
                        return Err(RuntimeError::new(
                            "str_center: width must be an integer".to_string(),
                        ))
                    }
                };
                let pad_char = if args.len() == 3 {
                    match &args[2] {
                        Value::String(p) if !p.is_empty() => {
                            p.chars().next().unwrap()
                        }
                        _ => ' ',
                    }
                } else {
                    ' '
                };
                let len = s.chars().count();
                if len >= width {
                    return Ok(Value::String(Arc::from(s)));
                }
                let total_pad = width - len;
                let left_pad = total_pad / 2;
                let right_pad = total_pad - left_pad;
                let result = format!(
                    "{}{}{}",
                    pad_char.to_string().repeat(left_pad),
                    s,
                    pad_char.to_string().repeat(right_pad)
                );
                Ok(Value::String(Arc::from(result)))
            }

            // ── array_sum(arr) ────────────────────────────────────────────────
            "array_sum" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "array_sum requires 1 argument (array)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::Array(arr) => {
                        let mut int_sum: i64 = 0;
                        let mut float_sum: f64 = 0.0;
                        let mut has_float = false;
                        for v in arr {
                            match v {
                                Value::Integer(i) => {
                                    int_sum = int_sum.checked_add(*i).ok_or_else(|| {
                                        RuntimeError::new("array_sum: integer overflow".to_string())
                                    })?;
                                }
                                Value::Float(f) => {
                                    float_sum += f;
                                    has_float = true;
                                }
                                _ => {
                                    return Err(RuntimeError::new(
                                        "array_sum: all elements must be numeric".to_string(),
                                    ))
                                }
                            }
                        }
                        if has_float {
                            Ok(Value::Float(int_sum as f64 + float_sum))
                        } else {
                            Ok(Value::Integer(int_sum))
                        }
                    }
                    _ => Err(RuntimeError::new(
                        "array_sum requires an array argument".to_string(),
                    )),
                }
            }

            // ── array_flatten(arr) — one level deep ───────────────────────────
            "array_flatten" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "array_flatten requires 1 argument (array)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::Array(arr) => {
                        let mut result = Vec::new();
                        for v in arr {
                            match v {
                                Value::Array(inner) => result.extend(inner.iter().cloned()),
                                other => result.push(other.clone()),
                            }
                        }
                        Ok(Value::Array(result))
                    }
                    _ => Err(RuntimeError::new(
                        "array_flatten requires an array argument".to_string(),
                    )),
                }
            }

            // ── array_enumerate(arr) → [[0,v0],[1,v1],...] ───────────────────
            "array_enumerate" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "array_enumerate requires 1 argument (array)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::Array(arr) => Ok(Value::Array(
                        arr.iter()
                            .enumerate()
                            .map(|(i, v)| Value::Array(vec![Value::Integer(i as i64), v.clone()]))
                            .collect(),
                    )),
                    _ => Err(RuntimeError::new(
                        "array_enumerate requires an array argument".to_string(),
                    )),
                }
            }

            // ── array_zip(arr1, arr2) → [[a0,b0],[a1,b1],...] ────────────────
            "array_zip" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "array_zip requires 2 arguments (arr1, arr2)".to_string(),
                    ));
                }
                match (&args[0], &args[1]) {
                    (Value::Array(a), Value::Array(b)) => {
                        let pairs: Vec<Value> = a
                            .iter()
                            .zip(b.iter())
                            .map(|(av, bv)| Value::Array(vec![av.clone(), bv.clone()]))
                            .collect();
                        Ok(Value::Array(pairs))
                    }
                    _ => Err(RuntimeError::new(
                        "array_zip requires two array arguments".to_string(),
                    )),
                }
            }

            // ── array_contains(arr, val) ──────────────────────────────────────
            "array_contains" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "array_contains requires 2 arguments (array, value)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::Array(arr) => Ok(Value::Boolean(arr.contains(&args[1]))),
                    _ => Err(RuntimeError::new(
                        "array_contains requires an array as first argument".to_string(),
                    )),
                }
            }

            // ── array_push(arr, val) — returns new array ──────────────────────
            "array_push" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "array_push requires 2 arguments (array, value)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::Array(arr) => {
                        let mut new_arr = arr.clone();
                        new_arr.push(args[1].clone());
                        Ok(Value::Array(new_arr))
                    }
                    _ => Err(RuntimeError::new(
                        "array_push requires an array as first argument".to_string(),
                    )),
                }
            }

            // ── array_pop(arr) — returns [new_arr, last_element] ─────────────
            "array_pop" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "array_pop requires 1 argument (array)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::Array(arr) if arr.is_empty() => Err(RuntimeError::new(
                        "array_pop: cannot pop from empty array".to_string(),
                    )),
                    Value::Array(arr) => {
                        let mut new_arr = arr.clone();
                        let last = new_arr.pop().unwrap();
                        Ok(Value::Array(vec![Value::Array(new_arr), last]))
                    }
                    _ => Err(RuntimeError::new(
                        "array_pop requires an array argument".to_string(),
                    )),
                }
            }

            // ── array_head(arr) — first element ──────────────────────────────
            "array_head" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "array_head requires 1 argument (array)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::Array(arr) if arr.is_empty() => Ok(Value::Null),
                    Value::Array(arr) => Ok(arr[0].clone()),
                    _ => Err(RuntimeError::new(
                        "array_head requires an array argument".to_string(),
                    )),
                }
            }

            // ── array_tail(arr) — all but first ──────────────────────────────
            "array_tail" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "array_tail requires 1 argument (array)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::Array(arr) if arr.is_empty() => Ok(Value::Array(vec![])),
                    Value::Array(arr) => Ok(Value::Array(arr[1..].to_vec())),
                    _ => Err(RuntimeError::new(
                        "array_tail requires an array argument".to_string(),
                    )),
                }
            }

            // ── Task 14.3: Iterator constructors ─────────────────────────────

            // range(start, end) or range(start, end, step) — lazy integer range
            "range" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(RuntimeError::new("range requires 2 or 3 arguments: range(start, end) or range(start, end, step)".to_string()));
                }
                let start = match &args[0] { Value::Integer(i) => *i, _ => return Err(RuntimeError::new("range: start must be integer".to_string())) };
                let end   = match &args[1] { Value::Integer(i) => *i, _ => return Err(RuntimeError::new("range: end must be integer".to_string())) };
                let step  = if args.len() == 3 {
                    match &args[2] { Value::Integer(i) if *i != 0 => *i, Value::Integer(_) => return Err(RuntimeError::new("range: step cannot be zero".to_string())), _ => return Err(RuntimeError::new("range: step must be integer".to_string())) }
                } else if start <= end { 1_i64 } else { -1_i64 };
                let mut fields = std::collections::HashMap::new();
                fields.insert("current".to_string(), Value::Integer(start));
                fields.insert("end".to_string(),     Value::Integer(end));
                fields.insert("step".to_string(),    Value::Integer(step));
                Ok(Value::Struct("__Range__".to_string(), fields))
            }

            // enumerate(iter) — wraps any iterable, yields [index, value] on each step
            "enumerate" => {
                if args.len() != 1 { return Err(RuntimeError::new("enumerate requires 1 argument".to_string())); }
                let mut fields = std::collections::HashMap::new();
                fields.insert("iter".to_string(),  args[0].clone());
                fields.insert("index".to_string(), Value::Integer(0));
                Ok(Value::Struct("__Enumerate__".to_string(), fields))
            }

            // zip(iter1, iter2) — pairs elements from two iterables
            "zip" => {
                if args.len() != 2 { return Err(RuntimeError::new("zip requires 2 arguments".to_string())); }
                let mut fields = std::collections::HashMap::new();
                fields.insert("iter1".to_string(), args[0].clone());
                fields.insert("iter2".to_string(), args[1].clone());
                Ok(Value::Struct("__Zip__".to_string(), fields))
            }

            // chain(iter1, iter2) — all of iter1, then all of iter2
            "chain" => {
                if args.len() != 2 { return Err(RuntimeError::new("chain requires 2 arguments".to_string())); }
                let mut fields = std::collections::HashMap::new();
                fields.insert("iter1".to_string(),      args[0].clone());
                fields.insert("iter2".to_string(),      args[1].clone());
                fields.insert("first_done".to_string(), Value::Boolean(false));
                Ok(Value::Struct("__Chain__".to_string(), fields))
            }

            _ => Err(RuntimeError::new(format!(
                "Unknown core function: {}",
                name
            ))),
        }
    }
}
