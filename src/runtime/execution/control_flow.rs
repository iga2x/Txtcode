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
            let mut err = None;
            for stmt in then_branch {
                if let Err(e) = vm.execute_statement(stmt) {
                    err = Some(e);
                    break;
                }
            }
            vm.pop_scope();
            if let Some(e) = err {
                return Err(e);
            }
        } else if let Some(else_body) = else_branch {
            vm.push_scope();
            let mut err = None;
            for stmt in else_body {
                if let Err(e) = vm.execute_statement(stmt) {
                    err = Some(e);
                    break;
                }
            }
            vm.pop_scope();
            if let Some(e) = err {
                return Err(e);
            }
        }
        Ok(Value::Null)
    }

    /// Execute a while loop
    pub fn execute_while(
        vm: &mut impl ControlFlowVM,
        condition: &Expression,
        body: &[Statement],
    ) -> Result<Value, RuntimeError> {
        'outer: loop {
            let cond_val = vm.evaluate_expression(condition)?;
            if !OperatorRegistry::is_truthy(&cond_val) {
                break;
            }
            vm.push_scope();
            let mut err: Option<RuntimeError> = None;
            let mut should_break = false;
            for stmt in body {
                match vm.execute_statement(stmt) {
                    Ok(_) => {}
                    Err(e) if e.is_break_signal() => {
                        should_break = true;
                        break;
                    }
                    Err(e) if e.is_continue_signal() => {
                        break;
                    }
                    Err(e) => {
                        err = Some(e);
                        break;
                    }
                }
            }
            vm.pop_scope();
            if should_break {
                break 'outer;
            }
            if let Some(e) = err {
                return Err(e);
            }
        }
        Ok(Value::Null)
    }

    /// Execute a do-while loop
    pub fn execute_do_while(
        vm: &mut impl ControlFlowVM,
        body: &[Statement],
        condition: &Expression,
    ) -> Result<Value, RuntimeError> {
        'outer: loop {
            vm.push_scope();
            let mut err: Option<RuntimeError> = None;
            let mut should_break = false;
            for stmt in body {
                match vm.execute_statement(stmt) {
                    Ok(_) => {}
                    Err(e) if e.is_break_signal() => {
                        should_break = true;
                        break;
                    }
                    Err(e) if e.is_continue_signal() => {
                        break;
                    }
                    Err(e) => {
                        err = Some(e);
                        break;
                    }
                }
            }
            vm.pop_scope();
            if should_break {
                break 'outer;
            }
            if let Some(e) = err {
                return Err(e);
            }

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
            }
            Value::Map(map) => {
                log_debug(&format!("For loop: iterating map with {} keys", map.len()));
                map.keys().map(|k| Value::String(k.clone())).collect()
            }
            Value::Set(set) => {
                log_debug(&format!(
                    "For loop: iterating set with {} elements",
                    set.len()
                ));
                set.into_iter().collect()
            }
            Value::String(s) => {
                log_debug(&format!(
                    "For loop: iterating string with {} chars",
                    s.len()
                ));
                s.chars().map(|c| Value::String(c.to_string())).collect()
            }
            _ => {
                log_warn(&format!("For loop: cannot iterate over {:?}", iter_val));
                return Err(RuntimeError::new(
                    "Cannot iterate over this value. Expected array, map, set, or string."
                        .to_string(),
                ));
            }
        };

        // Push scope for loop variable
        vm.push_scope();
        let mut outer_err: Option<RuntimeError> = None;
        'outer: for item in items {
            if let Err(e) = vm.set_variable(variable.to_string(), item) {
                outer_err = Some(e);
                break 'outer;
            }
            for stmt in body {
                match vm.execute_statement(stmt) {
                    Ok(_) => {}
                    Err(e) if e.is_break_signal() => {
                        break 'outer;
                    }
                    Err(e) if e.is_continue_signal() => {
                        break;
                    } // go to next item
                    Err(e) => {
                        outer_err = Some(e);
                        break 'outer;
                    }
                }
            }
        }
        vm.pop_scope();
        if let Some(e) = outer_err {
            return Err(e);
        }
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
            vm.push_scope();
            let pattern_match = vm.bind_pattern(pattern, &match_val);

            if pattern_match.is_ok() {
                // Pattern matched - check guard if present
                let guard_passed = if let Some(guard_expr) = guard {
                    match vm.evaluate_expression(guard_expr) {
                        Ok(v) => OperatorRegistry::is_truthy(&v),
                        Err(e) => {
                            vm.pop_scope();
                            return Err(e);
                        }
                    }
                } else {
                    true
                };

                if guard_passed {
                    // Execute case body
                    let mut err = None;
                    for stmt in body {
                        if let Err(e) = vm.execute_statement(stmt) {
                            err = Some(e);
                            break;
                        }
                    }
                    vm.pop_scope();
                    if let Some(e) = err {
                        return Err(e);
                    }
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
                let mut err = None;
                for stmt in default_body {
                    if let Err(e) = vm.execute_statement(stmt) {
                        err = Some(e);
                        break;
                    }
                }
                vm.pop_scope();
                if let Some(e) = err {
                    return Err(e);
                }
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
        let try_result: Result<Value, RuntimeError> = (|| -> Result<Value, RuntimeError> {
            for stmt in body {
                vm.execute_statement(stmt)?;
            }
            Ok(Value::Null)
        })();

        let is_signal = try_result
            .as_ref()
            .err()
            .is_some_and(|e| e.is_control_flow_signal());

        // Control-flow signals bypass the catch handler
        let mut outcome: Result<Value, RuntimeError> = if is_signal {
            try_result
        } else if let Err(error) = try_result {
            // Genuine runtime error — run catch handler
            if let Some((error_var, catch_body)) = catch {
                vm.push_scope();
                let set_res = vm.set_variable(
                    error_var.clone(),
                    Value::String(error.message().to_string()),
                );
                if let Err(e) = set_res {
                    vm.pop_scope();
                    Err(e)
                } else {
                    let mut catch_err = None;
                    for stmt in catch_body {
                        if let Err(e) = vm.execute_statement(stmt) {
                            catch_err = Some(e);
                            break;
                        }
                    }
                    vm.pop_scope();
                    match catch_err {
                        Some(e) => Err(e),
                        None => Ok(Value::Null),
                    }
                }
            } else {
                // No catch block, re-throw
                Err(error)
            }
        } else {
            Ok(Value::Null)
        };

        // Finally always runs; if finally itself errors, that error wins
        if let Some(finally_body) = finally {
            for stmt in finally_body {
                if let Err(e) = vm.execute_statement(stmt) {
                    outcome = Err(e);
                    break;
                }
            }
        }

        outcome
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
        'outer: for _ in 0..n {
            for stmt in body {
                match vm.execute_statement(stmt) {
                    Ok(_) => {}
                    Err(e) if e.is_break_signal() => {
                        break 'outer;
                    }
                    Err(e) if e.is_continue_signal() => {
                        break;
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
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
