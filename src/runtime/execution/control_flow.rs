use crate::parser::ast::*;
use crate::runtime::core::Value;
use crate::runtime::errors::RuntimeError;
use crate::runtime::operators::OperatorRegistry;

/// Maximum number of iterations for `repeat → N` loops.
/// Prevents runaway loops from freezing the process.
const MAX_REPEAT_COUNT: i64 = 10_000_000;

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

        let iter_val_raw = vm.evaluate_expression(iterable)?;
        // Task 15.2: Auto-resolve futures so `for`/`async for` can consume async streams.
        let iter_val = match iter_val_raw {
            Value::Future(handle) => handle
                .resolve()
                .map_err(|e| RuntimeError::new(e))?,
            other => other,
        };
        log_debug(&format!(
            "For loop: variable='{}', iterable type={:?}, value={:?}",
            variable, iter_val, iter_val
        ));
        // ── Iterator protocol: struct with next() method ─────────────────
        if let Value::Struct(ref type_name, ref fields) = iter_val {
            // Built-in lazy range iterator
            if type_name == "__Range__" {
                let current = match fields.get("current") { Some(Value::Integer(i)) => *i, _ => return Err(RuntimeError::new("__Range__: missing 'current' field".to_string())) };
                let end     = match fields.get("end")     { Some(Value::Integer(i)) => *i, _ => return Err(RuntimeError::new("__Range__: missing 'end' field".to_string())) };
                let step    = match fields.get("step")    { Some(Value::Integer(i)) => *i, _ => 1 };
                vm.push_scope();
                let mut i = current;
                let mut outer_err: Option<RuntimeError> = None;
                'range_outer: loop {
                    if step > 0 && i >= end { break; }
                    if step < 0 && i <= end { break; }
                    if step == 0 { break; }
                    if let Err(e) = vm.set_variable(variable.to_string(), Value::Integer(i)) {
                        outer_err = Some(e); break;
                    }
                    for stmt in body {
                        match vm.execute_statement(stmt) {
                            Ok(_) => {}
                            Err(e) if e.is_break_signal() => { break 'range_outer; }
                            Err(e) if e.is_continue_signal() => { break; }
                            Err(e) => { outer_err = Some(e); break 'range_outer; }
                        }
                    }
                    i = match i.checked_add(step) { Some(v) => v, None => break };
                }
                vm.pop_scope();
                return if let Some(e) = outer_err { Err(e) } else { Ok(Value::Null) };
            }

            // Built-in enumerate iterator: yields [index, element] pairs
            if type_name == "__Enumerate__" {
                let inner_iter = match fields.get("iter") { Some(v) => v.clone(), None => return Err(RuntimeError::new("__Enumerate__: missing 'iter' field".to_string())) };
                let index_start = match fields.get("index") { Some(Value::Integer(i)) => *i, _ => 0 };
                // Materialize inner iter to an array then wrap with index
                let inner_items: Vec<Value> = match inner_iter {
                    Value::Array(arr) => arr,
                    Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(),
                    Value::Set(s) => s,
                    _ => return Err(RuntimeError::new("enumerate: inner iterator must be an array, string, or set".to_string())),
                };
                vm.push_scope();
                let mut outer_err: Option<RuntimeError> = None;
                'enum_outer: for (i, item) in inner_items.into_iter().enumerate() {
                    let pair = Value::Array(vec![Value::Integer(index_start + i as i64), item]);
                    if let Err(e) = vm.set_variable(variable.to_string(), pair) {
                        outer_err = Some(e); break;
                    }
                    for stmt in body {
                        match vm.execute_statement(stmt) {
                            Ok(_) => {}
                            Err(e) if e.is_break_signal() => { break 'enum_outer; }
                            Err(e) if e.is_continue_signal() => { break; }
                            Err(e) => { outer_err = Some(e); break 'enum_outer; }
                        }
                    }
                }
                vm.pop_scope();
                return if let Some(e) = outer_err { Err(e) } else { Ok(Value::Null) };
            }

            // Built-in zip iterator: yields [a, b] pairs
            if type_name == "__Zip__" {
                let iter1 = match fields.get("iter1") { Some(v) => v.clone(), None => return Err(RuntimeError::new("__Zip__: missing 'iter1'".to_string())) };
                let iter2 = match fields.get("iter2") { Some(v) => v.clone(), None => return Err(RuntimeError::new("__Zip__: missing 'iter2'".to_string())) };
                let items1: Vec<Value> = match iter1 { Value::Array(a) => a, Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(), _ => return Err(RuntimeError::new("zip: iter1 must be an array or string".to_string())) };
                let items2: Vec<Value> = match iter2 { Value::Array(a) => a, Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(), _ => return Err(RuntimeError::new("zip: iter2 must be an array or string".to_string())) };
                vm.push_scope();
                let mut outer_err: Option<RuntimeError> = None;
                'zip_outer: for (a, b) in items1.into_iter().zip(items2.into_iter()) {
                    let pair = Value::Array(vec![a, b]);
                    if let Err(e) = vm.set_variable(variable.to_string(), pair) {
                        outer_err = Some(e); break;
                    }
                    for stmt in body {
                        match vm.execute_statement(stmt) {
                            Ok(_) => {}
                            Err(e) if e.is_break_signal() => { break 'zip_outer; }
                            Err(e) if e.is_continue_signal() => { break; }
                            Err(e) => { outer_err = Some(e); break 'zip_outer; }
                        }
                    }
                }
                vm.pop_scope();
                return if let Some(e) = outer_err { Err(e) } else { Ok(Value::Null) };
            }

            // Built-in chain iterator: iter1 then iter2
            if type_name == "__Chain__" {
                let iter1 = match fields.get("iter1") { Some(v) => v.clone(), None => return Err(RuntimeError::new("__Chain__: missing 'iter1'".to_string())) };
                let iter2 = match fields.get("iter2") { Some(v) => v.clone(), None => return Err(RuntimeError::new("__Chain__: missing 'iter2'".to_string())) };
                let arr1: Vec<Value> = match iter1 { Value::Array(a) => a, Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(), _ => return Err(RuntimeError::new("chain: iter1 must be an array or string".to_string())) };
                let arr2: Vec<Value> = match iter2 { Value::Array(a) => a, Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(), _ => return Err(RuntimeError::new("chain: iter2 must be an array or string".to_string())) };
                let chained: Vec<Value> = arr1.into_iter().chain(arr2.into_iter()).collect();
                vm.push_scope();
                let mut outer_err: Option<RuntimeError> = None;
                'chain_outer: for item in chained {
                    if let Err(e) = vm.set_variable(variable.to_string(), item) {
                        outer_err = Some(e); break;
                    }
                    for stmt in body {
                        match vm.execute_statement(stmt) {
                            Ok(_) => {}
                            Err(e) if e.is_break_signal() => { break 'chain_outer; }
                            Err(e) if e.is_continue_signal() => { break; }
                            Err(e) => { outer_err = Some(e); break 'chain_outer; }
                        }
                    }
                }
                vm.pop_scope();
                return if let Some(e) = outer_err { Err(e) } else { Ok(Value::Null) };
            }

            // User-defined iterator: next(self) → [value, new_state] | null
            if vm.call_struct_method(iter_val.clone(), "next").is_some() {
                vm.push_scope();
                let mut state = iter_val.clone();
                let mut outer_err: Option<RuntimeError> = None;
                'iter_outer: loop {
                    let result = match vm.call_struct_method(state.clone(), "next") {
                        Some(Ok(v)) => v,
                        Some(Err(e)) => { outer_err = Some(e); break; }
                        None => break,
                    };
                    match result {
                        Value::Null => break,
                        Value::Array(ref pair) if pair.len() == 2 => {
                            let item = pair[0].clone();
                            state = pair[1].clone();
                            if let Err(e) = vm.set_variable(variable.to_string(), item) {
                                outer_err = Some(e); break 'iter_outer;
                            }
                            for stmt in body {
                                match vm.execute_statement(stmt) {
                                    Ok(_) => {}
                                    Err(e) if e.is_break_signal() => { break 'iter_outer; }
                                    Err(e) if e.is_continue_signal() => { break; }
                                    Err(e) => { outer_err = Some(e); break 'iter_outer; }
                                }
                            }
                        }
                        other => {
                            outer_err = Some(RuntimeError::new(format!(
                                "Iterator next() must return [value, new_state] or null, got {:?}", other
                            )));
                            break;
                        }
                    }
                }
                vm.pop_scope();
                return if let Some(e) = outer_err { Err(e) } else { Ok(Value::Null) };
            }

            log_warn(&format!("For loop: struct '{}' has no next() method", type_name));
            return Err(RuntimeError::new(format!(
                "Struct '{}' is not iterable: no next() method found", type_name
            )));
        }
        // ──────────────────────────────────────────────────────────────────

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
        if n > MAX_REPEAT_COUNT {
            return Err(RuntimeError::new(format!(
                "repeat count {} exceeds maximum allowed iterations ({})",
                n, MAX_REPEAT_COUNT
            )));
        }
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
    /// Call a user-defined method on a struct value (iterator protocol).
    /// Returns None if the struct type has no registered method with this name.
    fn call_struct_method(&mut self, obj: Value, method: &str) -> Option<Result<Value, RuntimeError>>;
}
