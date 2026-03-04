use crate::runtime::{Value, RuntimeError};
use crate::stdlib::FunctionExecutor;

/// Core standard library functions
pub struct CoreLib;

impl CoreLib {
    pub fn call_function<E: FunctionExecutor>(name: &str, args: &[Value], executor: Option<&mut E>) -> Result<Value, RuntimeError> {
        match name {
            "print" => {
                if let Some(val) = args.first() {
                    println!("{}", val.to_string());
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
                        _ => Err(RuntimeError::new("len() requires string, array, map, or set".to_string())),
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
                        Value::Enum(_, _) => "enum",
                    };
                    Ok(Value::String(type_name.to_string()))
                } else {
                    Err(RuntimeError::new("type() requires one argument".to_string()))
                }
            }
            "input" => {
                use std::io::{self, Write};
                print!("> ");
                io::stdout().flush().unwrap();
                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap();
                Ok(Value::String(input.trim().to_string()))
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
                        _ => return Err(RuntimeError::new("log() base must be a number".to_string())),
                    };
                    let x = match args.first().unwrap() {
                        Value::Float(x) => *x,
                        Value::Integer(x) => *x as f64,
                        _ => return Err(RuntimeError::new("log() requires a number".to_string())),
                    };
                    Ok(Value::Float(x.log(base)))
                } else {
                    Err(RuntimeError::new("log() requires 1 or 2 arguments".to_string()))
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
                    _ => return Err(RuntimeError::new("pow() exponent must be a number".to_string())),
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
                if args.len() < 1 || args.len() > 2 {
                    return Err(RuntimeError::new("split() requires 1 or 2 arguments".to_string()));
                }
                let s = match args.first().unwrap() {
                    Value::String(s) => s,
                    _ => return Err(RuntimeError::new("split() requires a string".to_string())),
                };
                let delimiter = if args.len() == 2 {
                    match args.get(1).unwrap() {
                        Value::String(d) => d.as_str(),
                        _ => return Err(RuntimeError::new("split() delimiter must be a string".to_string())),
                    }
                } else {
                    " "
                };
                let parts: Vec<Value> = s.split(delimiter).map(|p| Value::String(p.to_string())).collect();
                Ok(Value::Array(parts))
            }
            "str_join" | "join" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new("join() requires 2 arguments (array, separator)".to_string()));
                }
                let arr = match args.first().unwrap() {
                    Value::Array(a) => a,
                    _ => return Err(RuntimeError::new("join() first argument must be an array".to_string())),
                };
                let sep = match args.get(1).unwrap() {
                    Value::String(s) => s,
                    _ => return Err(RuntimeError::new("join() second argument must be a string".to_string())),
                };
                let strings: Vec<String> = arr.iter()
                    .map(|v| match v {
                        Value::String(s) => s.clone(),
                        _ => v.to_string(),
                    })
                    .collect();
                Ok(Value::String(strings.join(sep)))
            }
            "str_replace" | "replace" => {
                if args.len() != 3 {
                    return Err(RuntimeError::new("replace() requires 3 arguments (string, old, new)".to_string()));
                }
                let s = match args.first().unwrap() {
                    Value::String(s) => s,
                    _ => return Err(RuntimeError::new("replace() first argument must be a string".to_string())),
                };
                let old = match args.get(1).unwrap() {
                    Value::String(o) => o,
                    _ => return Err(RuntimeError::new("replace() second argument must be a string".to_string())),
                };
                let new = match args.get(2).unwrap() {
                    Value::String(n) => n,
                    _ => return Err(RuntimeError::new("replace() third argument must be a string".to_string())),
                };
                Ok(Value::String(s.replace(old, new)))
            }
            "str_trim" | "trim" => {
                if let Some(Value::String(s)) = args.first() {
                    Ok(Value::String(s.trim().to_string()))
                } else {
                    Err(RuntimeError::new("trim() requires a string".to_string()))
                }
            }
            "str_substring" | "substring" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(RuntimeError::new("substring() requires 2 or 3 arguments".to_string()));
                }
                let s = match args.first().unwrap() {
                    Value::String(s) => s,
                    _ => return Err(RuntimeError::new("substring() first argument must be a string".to_string())),
                };
                let start = match args.get(1).unwrap() {
                    Value::Integer(i) => *i as usize,
                    _ => return Err(RuntimeError::new("substring() start must be an integer".to_string())),
                };
                let end = if args.len() == 3 {
                    match args.get(2).unwrap() {
                        Value::Integer(i) => *i as usize,
                        _ => return Err(RuntimeError::new("substring() end must be an integer".to_string())),
                    }
                } else {
                    s.len()
                };
                if start > s.len() || end > s.len() || start > end {
                    return Err(RuntimeError::new("substring() indices out of bounds".to_string()));
                }
                Ok(Value::String(s[start..end].to_string()))
            }
            "str_indexOf" | "indexOf" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new("indexOf() requires 2 arguments".to_string()));
                }
                let s = match args.first().unwrap() {
                    Value::String(s) => s,
                    _ => return Err(RuntimeError::new("indexOf() first argument must be a string".to_string())),
                };
                let search = match args.get(1).unwrap() {
                    Value::String(ss) => ss,
                    _ => return Err(RuntimeError::new("indexOf() second argument must be a string".to_string())),
                };
                match s.find(search) {
                    Some(idx) => Ok(Value::Integer(idx as i64)),
                    None => Ok(Value::Integer(-1)),
                }
            }
            "str_startsWith" | "startsWith" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new("startsWith() requires 2 arguments".to_string()));
                }
                let s = match args.first().unwrap() {
                    Value::String(s) => s,
                    _ => return Err(RuntimeError::new("startsWith() first argument must be a string".to_string())),
                };
                let prefix = match args.get(1).unwrap() {
                    Value::String(p) => p,
                    _ => return Err(RuntimeError::new("startsWith() second argument must be a string".to_string())),
                };
                Ok(Value::Boolean(s.starts_with(prefix)))
            }
            "str_endsWith" | "endsWith" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new("endsWith() requires 2 arguments".to_string()));
                }
                let s = match args.first().unwrap() {
                    Value::String(s) => s,
                    _ => return Err(RuntimeError::new("endsWith() first argument must be a string".to_string())),
                };
                let suffix = match args.get(1).unwrap() {
                    Value::String(s) => s,
                    _ => return Err(RuntimeError::new("endsWith() second argument must be a string".to_string())),
                };
                Ok(Value::Boolean(s.ends_with(suffix)))
            }
            "str_toUpper" | "toUpper" => {
                if let Some(Value::String(s)) = args.first() {
                    Ok(Value::String(s.to_uppercase()))
                } else {
                    Err(RuntimeError::new("toUpper() requires a string".to_string()))
                }
            }
            "str_toLower" | "toLower" => {
                if let Some(Value::String(s)) = args.first() {
                    Ok(Value::String(s.to_lowercase()))
                } else {
                    Err(RuntimeError::new("toLower() requires a string".to_string()))
                }
            }
            // Array functions
            "array_map" | "map" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new("map() requires 2 arguments (array, function)".to_string()));
                }
                let arr = match args.first().unwrap() {
                    Value::Array(a) => a,
                    _ => return Err(RuntimeError::new("map() first argument must be an array".to_string())),
                };
                let func = args.get(1).unwrap();
                
                let executor = executor.ok_or_else(|| RuntimeError::new("map() requires function executor".to_string()))?;
                
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
                    return Err(RuntimeError::new("filter() requires 2 arguments (array, function)".to_string()));
                }
                let arr = match args.first().unwrap() {
                    Value::Array(a) => a,
                    _ => return Err(RuntimeError::new("filter() first argument must be an array".to_string())),
                };
                let func = args.get(1).unwrap();
                
                let executor = executor.ok_or_else(|| RuntimeError::new("filter() requires function executor".to_string()))?;
                
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
                    return Err(RuntimeError::new("reduce() requires 2 or 3 arguments".to_string()));
                }
                let arr = match args.first().unwrap() {
                    Value::Array(a) => a,
                    _ => return Err(RuntimeError::new("reduce() first argument must be an array".to_string())),
                };
                let func = args.get(1).unwrap();
                
                let executor = executor.ok_or_else(|| RuntimeError::new("reduce() requires function executor".to_string()))?;
                
                let mut accumulator = if args.len() == 3 {
                    args.get(2).unwrap().clone()
                } else if arr.is_empty() {
                    return Err(RuntimeError::new("reduce() on empty array requires initial value".to_string()));
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
                    return Err(RuntimeError::new("find() requires 2 arguments (array, function)".to_string()));
                }
                let arr = match args.first().unwrap() {
                    Value::Array(a) => a,
                    _ => return Err(RuntimeError::new("find() first argument must be an array".to_string())),
                };
                let func = args.get(1).unwrap();
                
                let executor = executor.ok_or_else(|| RuntimeError::new("find() requires function executor".to_string()))?;
                
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
                    return Err(RuntimeError::new("sort() requires 0 or 1 argument".to_string()));
                }
                if let Some(Value::Array(mut arr)) = args.first().cloned() {
                    // Simple numeric/string sort
                    arr.sort_by(|a, b| {
                        match (a, b) {
                            (Value::Integer(i1), Value::Integer(i2)) => i1.cmp(i2),
                            (Value::Float(f1), Value::Float(f2)) => f1.partial_cmp(f2).unwrap_or(std::cmp::Ordering::Equal),
                            (Value::String(s1), Value::String(s2)) => s1.cmp(s2),
                            _ => std::cmp::Ordering::Equal,
                        }
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
                if args.len() < 1 {
                    return Err(RuntimeError::new("concat() requires at least 1 argument".to_string()));
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
                if args.len() < 1 || args.len() > 3 {
                    return Err(RuntimeError::new("array_slice() requires 1-3 arguments".to_string()));
                }
                let arr = match args.first().unwrap() {
                    Value::Array(a) => a,
                    _ => return Err(RuntimeError::new("array_slice() first argument must be an array".to_string())),
                };
                let start = if args.len() >= 2 {
                    match args.get(1).unwrap() {
                        Value::Integer(i) => *i as usize,
                        _ => return Err(RuntimeError::new("array_slice() start must be an integer".to_string())),
                    }
                } else {
                    0
                };
                let end = if args.len() == 3 {
                    match args.get(2).unwrap() {
                        Value::Integer(i) => *i as usize,
                        _ => return Err(RuntimeError::new("array_slice() end must be an integer".to_string())),
                    }
                } else {
                    arr.len()
                };
                if start > arr.len() || end > arr.len() || start > end {
                    return Err(RuntimeError::new("array_slice() indices out of bounds".to_string()));
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
            _ => Err(RuntimeError::new(format!("Unknown core function: {}", name))),
        }
    }
}

