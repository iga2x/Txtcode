// Array, Map, Set, and Slice evaluation

use super::ExpressionVM;
use crate::parser::ast::Expression;
use crate::runtime::core::Value;
use crate::runtime::errors::RuntimeError;
use std::collections::HashMap;

pub fn evaluate_array<VM: ExpressionVM>(
    vm: &mut VM,
    elements: &[Expression],
) -> Result<Value, RuntimeError> {
    let mut result: Vec<Value> = Vec::new();
    for e in elements {
        match e {
            Expression::Spread { value, .. } => {
                // ...arr — flatten the spread array into the result
                let spread_val = super::ExpressionEvaluator::evaluate(vm, value)?;
                match spread_val {
                    Value::Array(arr) => result.extend(arr),
                    other => {
                        return Err(RuntimeError::new(format!(
                            "Spread operator requires an array, got: {}",
                            other
                        )))
                    }
                }
            }
            other => {
                result.push(super::ExpressionEvaluator::evaluate(vm, other)?);
            }
        }
    }
    let arr = Value::Array(result);
    vm.gc_register_allocation(&arr);
    Ok(arr)
}

pub fn evaluate_map<VM: ExpressionVM>(
    vm: &mut VM,
    entries: &[(Expression, Expression)],
) -> Result<Value, RuntimeError> {
    let mut map = HashMap::new();
    for (key_expr, value_expr) in entries {
        let key_val = super::ExpressionEvaluator::evaluate(vm, key_expr)?;
        let value_val = super::ExpressionEvaluator::evaluate(vm, value_expr)?;
        let key = match key_val {
            Value::String(s) => s,
            _ => return Err(vm.create_error("Map keys must be strings".to_string())),
        };
        map.insert(key, value_val);
    }
    let map_val = Value::Map(map);
    // Register with GC
    vm.gc_register_allocation(&map_val);
    Ok(map_val)
}

pub fn evaluate_set<VM: ExpressionVM>(
    vm: &mut VM,
    elements: &[Expression],
) -> Result<Value, RuntimeError> {
    let mut set = Vec::new();
    for elem_expr in elements {
        let elem_val = super::ExpressionEvaluator::evaluate(vm, elem_expr)?;
        // Only add if not already in set (maintain uniqueness)
        if !set.contains(&elem_val) {
            set.push(elem_val);
        }
    }
    let set_val = Value::Set(set);
    // Register with GC
    vm.gc_register_allocation(&set_val);
    Ok(set_val)
}

pub fn evaluate_slice<VM: ExpressionVM>(
    vm: &mut VM,
    target: &Expression,
    start: &Option<Box<Expression>>,
    end: &Option<Box<Expression>>,
    step: &Option<Box<Expression>>,
) -> Result<Value, RuntimeError> {
    let obj = super::ExpressionEvaluator::evaluate(vm, target)?;
    match obj {
        Value::Array(arr) => {
            // Handle step parameter first to determine direction
            let step_val = if let Some(s) = step {
                match super::ExpressionEvaluator::evaluate(vm, s.as_ref())? {
                    Value::Integer(i) if i > 0 => i as usize,
                    Value::Integer(i) if i < 0 => {
                        // Negative step - reverse slicing
                        let abs_step = (-i) as usize;

                        if arr.is_empty() {
                            return Ok(Value::Array(vec![]));
                        }

                        let start_idx = if let Some(s) = start {
                            match super::ExpressionEvaluator::evaluate(vm, s)? {
                                Value::Integer(i) => {
                                    if i < 0 {
                                        (arr.len() as i64 + i) as usize
                                    } else {
                                        i as usize
                                    }
                                }
                                _ => {
                                    return Err(RuntimeError::new(
                                        "Slice start must be integer".to_string(),
                                    ))
                                }
                            }
                        } else {
                            arr.len() - 1
                        };

                        let end_idx = if let Some(e) = end {
                            match super::ExpressionEvaluator::evaluate(vm, e.as_ref())? {
                                Value::Integer(i) => {
                                    if i < 0 {
                                        (arr.len() as i64 + i) as usize
                                    } else {
                                        i as usize
                                    }
                                }
                                _ => {
                                    return Err(RuntimeError::new(
                                        "Slice end must be integer".to_string(),
                                    ))
                                }
                            }
                        } else {
                            0
                        };

                        if start_idx >= arr.len() || end_idx >= arr.len() {
                            return Err(RuntimeError::new("Invalid slice indices".to_string()));
                        }

                        let mut result = Vec::new();
                        let mut idx = start_idx;
                        while idx > end_idx {
                            result.push(arr[idx].clone());
                            if idx < abs_step {
                                break;
                            }
                            idx -= abs_step;
                        }
                        if idx == end_idx {
                            result.push(arr[idx].clone());
                        }
                        return Ok(Value::Array(result));
                    }
                    Value::Integer(0) => {
                        return Err(RuntimeError::new("Slice step cannot be zero".to_string()));
                    }
                    _ => {
                        return Err(RuntimeError::new(
                            "Slice step must be an integer".to_string(),
                        ))
                    }
                }
            } else {
                1
            };

            // Positive step (forward slicing)
            let start_idx = if let Some(s) = start {
                match super::ExpressionEvaluator::evaluate(vm, s.as_ref())? {
                    Value::Integer(i) => {
                        if i < 0 {
                            (arr.len() as i64 + i) as usize
                        } else {
                            i as usize
                        }
                    }
                    _ => return Err(RuntimeError::new("Slice start must be integer".to_string())),
                }
            } else {
                0
            };

            let end_idx = if let Some(e) = end {
                match super::ExpressionEvaluator::evaluate(vm, e.as_ref())? {
                    Value::Integer(i) => {
                        if i < 0 {
                            (arr.len() as i64 + i) as usize
                        } else {
                            i as usize
                        }
                    }
                    _ => return Err(RuntimeError::new("Slice end must be integer".to_string())),
                }
            } else {
                arr.len()
            };

            if start_idx > arr.len() || end_idx > arr.len() {
                return Err(RuntimeError::new("Invalid slice indices".to_string()));
            }

            let mut result = Vec::new();
            let mut idx = start_idx;
            while idx < end_idx {
                result.push(arr[idx].clone());
                idx += step_val;
            }
            Ok(Value::Array(result))
        }
        Value::String(s) => {
            // Evaluate step (Null/None → 1). Zero is a runtime error.
            let step_raw: i64 = if let Some(step_expr) = step {
                match super::ExpressionEvaluator::evaluate(vm, step_expr.as_ref())? {
                    Value::Integer(i) => i,
                    _ => {
                        return Err(RuntimeError::new(
                            "String slice step must be an integer".to_string(),
                        ))
                    }
                }
            } else {
                1
            };
            if step_raw == 0 {
                return Err(RuntimeError::new("Slice step cannot be zero".to_string()));
            }

            let chars: Vec<char> = s.chars().collect();
            let char_count = chars.len();

            // Resolve a char index: negative counts from end (using char count, not byte length).
            // Defined as a macro to avoid borrow-checker conflict with `vm`.
            macro_rules! resolve_str_idx {
                ($opt_expr:expr, $default:expr) => {
                    if let Some(expr) = $opt_expr {
                        match super::ExpressionEvaluator::evaluate(vm, (expr as &Box<Expression>).as_ref())? {
                            Value::Integer(i) => {
                                if i < 0 {
                                    let r = char_count as i64 + i;
                                    if r < 0 {
                                        return Err(RuntimeError::new(format!(
                                            "String slice index {} out of bounds for string of length {}",
                                            i, char_count
                                        )));
                                    }
                                    r as usize
                                } else {
                                    i as usize
                                }
                            }
                            _ => {
                                return Err(RuntimeError::new(
                                    "String slice index must be an integer".to_string(),
                                ))
                            }
                        }
                    } else {
                        $default
                    }
                };
            }

            if step_raw < 0 {
                if char_count == 0 {
                    return Ok(Value::String(String::new()));
                }
                let abs_step = (-step_raw) as usize;
                let start_idx = resolve_str_idx!(start, char_count - 1);
                let end_idx = resolve_str_idx!(end, 0);
                if start_idx >= char_count || end_idx >= char_count {
                    return Err(RuntimeError::new(
                        "Invalid string slice indices".to_string(),
                    ));
                }
                let mut result = Vec::new();
                let mut idx = start_idx;
                while idx > end_idx {
                    result.push(chars[idx]);
                    if idx < abs_step {
                        break;
                    }
                    idx -= abs_step;
                }
                if idx == end_idx {
                    result.push(chars[idx]);
                }
                return Ok(Value::String(result.into_iter().collect()));
            }

            // Positive step (forward).
            let abs_step = step_raw as usize;
            let start_idx = resolve_str_idx!(start, 0);
            let end_idx = resolve_str_idx!(end, char_count);
            if start_idx > char_count || end_idx > char_count || start_idx > end_idx {
                return Err(RuntimeError::new(
                    "Invalid string slice indices".to_string(),
                ));
            }
            let result: String = chars[start_idx..end_idx].iter().step_by(abs_step).collect();
            Ok(Value::String(result))
        }
        _ => Err(RuntimeError::new(
            "Slice only works on arrays and strings".to_string(),
        )),
    }
}
