// Array, Map, Set, and Slice evaluation

use crate::parser::ast::Expression;
use crate::runtime::core::Value;
use crate::runtime::errors::RuntimeError;
use std::collections::HashMap;
use super::ExpressionVM;

pub fn evaluate_array<VM: ExpressionVM>(
    vm: &mut VM,
    elements: &[Expression],
) -> Result<Value, RuntimeError> {
    let values: Result<Vec<Value>, RuntimeError> = elements.iter()
        .map(|e| super::ExpressionEvaluator::evaluate(vm, e))
        .collect();
    let arr = Value::Array(values?);
    // Register with GC
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
        let elem_val = super::ExpressionEvaluator::evaluate(vm, &elem_expr)?;
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
    target: &Box<Expression>,
    start: &Option<Box<Expression>>,
    end: &Option<Box<Expression>>,
    step: &Option<Box<Expression>>,
) -> Result<Value, RuntimeError> {
    let obj = super::ExpressionEvaluator::evaluate(vm, target.as_ref())?;
    match obj {
        Value::Array(arr) => {
            // Handle step parameter first to determine direction
            let step_val = if let Some(s) = step {
                match super::ExpressionEvaluator::evaluate(vm, s.as_ref())? {
                    Value::Integer(i) if i > 0 => i as usize,
                    Value::Integer(i) if i < 0 => {
                        // Negative step - reverse slicing
                        let abs_step = (-i) as usize;
                        
                        let start_idx = if let Some(s) = start {
                            match super::ExpressionEvaluator::evaluate(vm, s)? {
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
                                _ => return Err(RuntimeError::new("Slice end must be integer".to_string())),
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
                    _ => return Err(RuntimeError::new("Slice step must be an integer".to_string())),
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
            if step.is_some() {
                return Err(RuntimeError::new("String slicing with step not yet supported".to_string()));
            }
            
            let start_idx = if let Some(s_expr) = start {
                match super::ExpressionEvaluator::evaluate(vm, s_expr.as_ref())? {
                    Value::Integer(i) => {
                        if i < 0 {
                            (s.len() as i64 + i) as usize
                        } else {
                            i as usize
                        }
                    }
                    _ => return Err(RuntimeError::new("String slice start must be integer".to_string())),
                }
            } else {
                0
            };
            
            let end_idx = if let Some(e_expr) = end {
                match super::ExpressionEvaluator::evaluate(vm, e_expr.as_ref())? {
                    Value::Integer(i) => {
                        if i < 0 {
                            (s.len() as i64 + i) as usize
                        } else {
                            i as usize
                        }
                    }
                    _ => return Err(RuntimeError::new("String slice end must be integer".to_string())),
                }
            } else {
                s.len()
            };
            
            if start_idx > s.len() || end_idx > s.len() || start_idx > end_idx {
                return Err(RuntimeError::new("Invalid string slice indices".to_string()));
            }
            
            let result = s.chars().skip(start_idx).take(end_idx - start_idx).collect::<String>();
            Ok(Value::String(result))
        }
        _ => Err(RuntimeError::new("Slice only works on arrays and strings".to_string())),
    }
}

