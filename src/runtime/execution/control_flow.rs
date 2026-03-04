use crate::parser::ast::*;
use crate::runtime::core::Value;
use crate::runtime::errors::RuntimeError;
use crate::runtime::operators::OperatorRegistry;

/// Control flow execution (if, while, for, match, try)
pub struct ControlFlowExecutor;

impl ControlFlowExecutor {
    /// Execute an if statement
    pub fn execute_if(
        vm: &mut impl ControlFlowVM,
        condition: &Expression,
        then_branch: &[Statement],
        else_branch: &Option<Vec<Statement>>,
    ) -> Result<Value, RuntimeError> {
        let cond = vm.evaluate_expression(condition)?;
        if OperatorRegistry::is_truthy(&cond) {
            vm.push_scope();
            for stmt in then_branch {
                vm.execute_statement(stmt)?;
            }
            vm.pop_scope();
        } else if let Some(else_body) = else_branch {
            vm.push_scope();
            for stmt in else_body {
                vm.execute_statement(stmt)?;
            }
            vm.pop_scope();
        }
        Ok(Value::Null)
    }

    /// Execute a while loop
    pub fn execute_while(
        vm: &mut impl ControlFlowVM,
        condition: &Expression,
        body: &[Statement],
    ) -> Result<Value, RuntimeError> {
        loop {
            let cond_val = vm.evaluate_expression(condition)?;
            if !OperatorRegistry::is_truthy(&cond_val) {
                break;
            }
            vm.push_scope();
            for stmt in body {
                vm.execute_statement(stmt)?;
            }
            vm.pop_scope();
        }
        Ok(Value::Null)
    }

    /// Execute a do-while loop
    pub fn execute_do_while(
        vm: &mut impl ControlFlowVM,
        body: &[Statement],
        condition: &Expression,
    ) -> Result<Value, RuntimeError> {
        loop {
            vm.push_scope();
            for stmt in body {
                vm.execute_statement(stmt)?;
            }
            vm.pop_scope();
            
            let cond_val = vm.evaluate_expression(condition)?;
            if !OperatorRegistry::is_truthy(&cond_val) {
                break;
            }
        }
        Ok(Value::Null)
    }

    /// Execute a for loop
    pub fn execute_for(
        vm: &mut impl ControlFlowVM,
        variable: &str,
        iterable: &Expression,
        body: &[Statement],
    ) -> Result<Value, RuntimeError> {
        use crate::tools::logger::log_debug;
        use crate::tools::logger::log_warn;

        let iter_val = vm.evaluate_expression(iterable)?;
        log_debug(&format!(
            "For loop: variable='{}', iterable type={:?}, value={:?}",
            variable, iter_val, iter_val
        ));
        let items = match iter_val {
            Value::Array(arr) => {
                log_debug(&format!("For loop: array has {} items", arr.len()));
                arr
            },
            Value::Map(map) => {
                log_debug(&format!("For loop: iterating map with {} keys", map.len()));
                map.keys().map(|k| Value::String(k.clone())).collect()
            },
            Value::Set(set) => {
                log_debug(&format!("For loop: iterating set with {} elements", set.len()));
                set.into_iter().collect()
            },
            Value::String(s) => {
                log_debug(&format!("For loop: iterating string with {} chars", s.len()));
                s.chars().map(|c| Value::String(c.to_string())).collect()
            },
            _ => {
                log_warn(&format!(
                    "For loop: cannot iterate over {:?}",
                    iter_val
                ));
                return Err(RuntimeError::new(format!(
                    "Cannot iterate over this value. Expected array, map, set, or string."
                )));
            },
        };
        // Push scope for loop variable
        vm.push_scope();
        for item in items {
            vm.set_variable(variable.to_string(), item)?;
            for stmt in body {
                vm.execute_statement(stmt)?;
            }
        }
        vm.pop_scope();
        Ok(Value::Null)
    }

    /// Execute a match statement
    pub fn execute_match(
        vm: &mut impl ControlFlowVM,
        value: &Expression,
        cases: &[(Pattern, Option<Expression>, Vec<Statement>)],
        default: &Option<Vec<Statement>>,
    ) -> Result<Value, RuntimeError> {
        let match_val = vm.evaluate_expression(value)?;
        let mut matched = false;
        
        // Try each case
        for (pattern, guard, body) in cases {
            // Try to bind pattern to match value
            // Push scope for pattern binding
            vm.push_scope();
            let pattern_match = vm.bind_pattern(pattern, &match_val);
            
            if pattern_match.is_ok() {
                // Pattern matched - check guard if present
                let guard_passed = if let Some(guard_expr) = guard {
                    let guard_val = vm.evaluate_expression(guard_expr)?;
                    OperatorRegistry::is_truthy(&guard_val)
                } else {
                    true
                };
                
                if guard_passed {
                    // Execute case body
                    for stmt in body {
                        vm.execute_statement(stmt)?;
                    }
                    vm.pop_scope(); // Pop pattern binding scope
                    matched = true;
                    break;
                }
            }
            
            // Pattern didn't match or guard failed - pop scope and try next
            vm.pop_scope();
        }
        
        // Execute default case if no match
        if !matched {
            if let Some(default_body) = default {
                vm.push_scope();
                for stmt in default_body {
                    vm.execute_statement(stmt)?;
                }
                vm.pop_scope();
            }
        }
        
        Ok(Value::Null)
    }

    /// Execute a try-catch-finally statement
    pub fn execute_try(
        vm: &mut impl ControlFlowVM,
        body: &[Statement],
        catch: &Option<(String, Vec<Statement>)>,
        finally: &Option<Vec<Statement>>,
    ) -> Result<Value, RuntimeError> {
        // Execute try block
        let result = (|| -> Result<Value, RuntimeError> {
            for stmt in body {
                vm.execute_statement(stmt)?;
            }
            Ok(Value::Null)
        })();
        
        // Handle catch if error occurred
        if let Err(error) = result {
            if let Some((error_var, catch_body)) = catch {
                // Store error in variable
                vm.push_scope();
                vm.set_variable(error_var.clone(), Value::String(error.message().to_string()))?;
                for stmt in catch_body {
                    vm.execute_statement(stmt)?;
                }
                vm.pop_scope(); // Clean up catch scope
            } else {
                // No catch block, re-throw
                return Err(error);
            }
        }
        
        // Execute finally if present
        if let Some(finally_body) = finally {
            for stmt in finally_body {
                vm.execute_statement(stmt)?;
            }
        }
        
        Ok(Value::Null)
    }

    /// Execute a repeat statement
    pub fn execute_repeat(
        vm: &mut impl ControlFlowVM,
        count: &Expression,
        body: &[Statement],
    ) -> Result<Value, RuntimeError> {
        let count_val = vm.evaluate_expression(count)?;
        let n = match count_val {
            Value::Integer(i) => i,
            _ => return Err(RuntimeError::new("Repeat requires an integer".to_string())),
        };
        for _ in 0..n {
            for stmt in body {
                vm.execute_statement(stmt)?;
            }
        }
        Ok(Value::Null)
    }
}

/// Trait for VM methods needed by control flow execution
pub trait ControlFlowVM {
    fn evaluate_expression(&mut self, expr: &Expression) -> Result<Value, RuntimeError>;
    fn execute_statement(&mut self, stmt: &Statement) -> Result<Value, RuntimeError>;
    fn push_scope(&mut self);
    fn pop_scope(&mut self);
    fn set_variable(&mut self, name: String, value: Value) -> Result<(), RuntimeError>;
    fn bind_pattern(&mut self, pattern: &Pattern, value: &Value) -> Result<(), RuntimeError>;
}

