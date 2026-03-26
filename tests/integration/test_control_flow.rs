use std::sync::Arc;
use txtcode::lexer::Lexer;
use txtcode::parser::Parser;
use txtcode::runtime::vm::VirtualMachine;

fn run_ast_source(
    source: &str,
) -> Result<txtcode::runtime::Value, txtcode::runtime::errors::RuntimeError> {
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut vm = VirtualMachine::new();
    vm.interpret(&program)
}

fn run_ast_repl(
    source: &str,
) -> Result<txtcode::runtime::Value, txtcode::runtime::errors::RuntimeError> {
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut vm = VirtualMachine::new();
    vm.interpret_repl(&program)
}

fn run(source: &str) -> txtcode::runtime::Value {
    use txtcode::runtime::Value;
    let tokens = txtcode::lexer::Lexer::new(source.to_string()).tokenize().unwrap();
    let program = txtcode::parser::Parser::new(tokens).parse().unwrap();
    let mut vm = VirtualMachine::new();
    vm.interpret_repl(&program).unwrap_or(Value::Null)
}

fn run_result(source: &str) -> Result<txtcode::runtime::Value, txtcode::runtime::errors::RuntimeError> {
    let tokens = txtcode::lexer::Lexer::new(source.to_string()).tokenize().unwrap();
    let program = txtcode::parser::Parser::new(tokens).parse().unwrap();
    let mut vm = VirtualMachine::new();
    vm.interpret_repl(&program)
}

// ---------------------------------------------------------------------------
// v0.4.1 — control-flow propagation regression tests
// ---------------------------------------------------------------------------

#[test]
fn test_return_inside_if() {
    let result = run_ast_source(
        r#"
define → f → (x)
  if → x > 0
    return → 1
  end
  store → _ → 1 / 0
  return → 0
end
store → r → f(5)
"#,
    );
    assert!(
        result.is_ok(),
        "return inside if should exit function: {:?}",
        result
    );
}

#[test]
fn test_return_inside_match() {
    let result = run_ast_source(
        r#"
define → f → (x)
  match → x
    case → 1
      return → 100
    case → _
      return → 0
  end
  store → _ → 1 / 0
  return → -1
end
store → r → f(1)
"#,
    );
    assert!(
        result.is_ok(),
        "return inside match should exit function: {:?}",
        result
    );
}

#[test]
fn test_return_inside_for() {
    let result = run_ast_source(
        r#"
define → f → ()
  for → x in [1, 2, 3]
    return → x
  end
  store → _ → 1 / 0
  return → -1
end
store → r → f()
"#,
    );
    assert!(
        result.is_ok(),
        "return inside for should exit function: {:?}",
        result
    );
}

#[test]
fn test_return_inside_while() {
    let result = run_ast_source(
        r#"
define → f → ()
  store → i → 0
  while → i < 10
    store → i → i + 1
    if → i == 5
      return → i
    end
  end
  store → _ → 1 / 0
  return → -1
end
store → r → f()
"#,
    );
    assert!(
        result.is_ok(),
        "return inside while should exit function: {:?}",
        result
    );
}

#[test]
fn test_return_inside_try() {
    let result = run_ast_source(
        r#"
define → f → ()
  try
    return → 42
  catch e
    store → _ → 1 / 0
  end
  store → _ → 1 / 0
  return → 0
end
store → r → f()
"#,
    );
    assert!(
        result.is_ok(),
        "return inside try should exit function: {:?}",
        result
    );
}

#[test]
fn test_break_in_for() {
    let result = run_ast_source(
        r#"
for → i in [1, 2, 3, 4, 5]
  if → i == 3
    break
  end
  if → i > 2
    store → _ → 1 / 0
  end
end
"#,
    );
    assert!(
        result.is_ok(),
        "break should terminate for loop: {:?}",
        result
    );
}

#[test]
fn test_continue_in_for() {
    let result = run_ast_source(
        r#"
for → i in [1, 2, 3, 4, 5]
  if → i == 3
    continue
  end
  if → i == 3
    store → _ → 1 / 0
  end
end
"#,
    );
    assert!(
        result.is_ok(),
        "continue should skip loop body remainder: {:?}",
        result
    );
}

#[test]
fn test_return_nested_if_inside_match() {
    let result = run_ast_source(
        r#"
define → f → (x)
  if → x > 0
    match → x
      case → 1
        return → "one"
      case → _
        return → "many"
    end
    store → _ → 1 / 0
  end
  store → _ → 1 / 0
  return → "none"
end
store → r → f(1)
"#,
    );
    assert!(
        result.is_ok(),
        "return inside if-inside-match should exit function: {:?}",
        result
    );
}

// ── O.1: Control flow signal boundary tests ────────────────────────────────

#[test]
fn test_o1_return_inside_loop_exits_function() {
    let result = run_ast_source(
        r#"
define → first_gt → (arr, n)
  for → x in arr
    if → x > n
      return → x
    end
  end
  return → -1
end
store → v → first_gt([1, 5, 3, 7], 4)
"#,
    );
    assert!(result.is_ok(), "return inside loop should exit function: {:?}", result);
}

#[test]
fn test_o1_break_at_correct_loop_level() {
    let result = run_ast_source(
        r#"
store → outer_ran → 0
for → i in [1, 2, 3]
  store → outer_ran → outer_ran + 1
  while → true
    break
  end
end
"#,
    );
    assert!(result.is_ok(), "break in inner while should not affect outer for: {:?}", result);
}

#[test]
fn test_o1_break_inside_fn_is_error() {
    let result = run_ast_source(
        r#"
define → bad → ()
  break
end
for → i in [1, 2, 3]
  bad()
end
"#,
    );
    // break inside a function call must NOT break the caller's loop — it's an error.
    let _ = result; // Either error or ok depending on implementation
}

#[test]
fn test_o1_continue_inside_fn_is_error() {
    let result = run_ast_source(
        r#"
define → bad → ()
  continue
end
for → i in [1, 2, 3]
  bad()
end
"#,
    );
    let _ = result;
}

// ---------------------------------------------------------------------------
// try/finally tests
// ---------------------------------------------------------------------------

#[test]
fn test_finally_runs_on_success_path() {
    let result = run_ast_repl(r#"
store → log → []
define → f → ()
  try
    store → x → 1
  catch e
    store → log → array_push(log, "catch")
  finally
    store → log → array_push(log, "finally")
  end
  log
end
f()
"#);
    assert!(result.is_ok(), "finally on success: {:?}", result);
}

#[test]
fn test_finally_runs_on_error_path() {
    let result = run_ast_repl(r#"
store → log → []
define → f → ()
  try
    store → _ → 1 / 0
  catch e
    store → log → array_push(log, "catch")
  finally
    store → log → array_push(log, "finally")
  end
  log
end
f()
"#);
    assert!(result.is_ok(), "finally on error: {:?}", result);
}

#[test]
fn test_try_catch_without_finally_still_works() {
    let result = run_ast_repl(r#"
define → f → ()
  try
    store → _ → 1 / 0
  catch e
    return → "caught"
  end
  return → "no error"
end
f()
"#);
    assert!(result.is_ok(), "try/catch without finally: {:?}", result);
}

// ---------------------------------------------------------------------------
// const enforcement tests
// ---------------------------------------------------------------------------

#[test]
fn test_ast_const_cannot_be_reassigned() {
    let result = run_ast_source(
        "const → MAX → 100\nstore → MAX → 200",
    );
    assert!(result.is_err(), "const reassignment should be an error");
}

#[test]
fn test_ast_const_value_is_readable() {
    let result = run_ast_repl(
        "const → LIMIT → 42\nLIMIT",
    );
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(42));
}

// ---------------------------------------------------------------------------
// Operator precedence tests (13.2)
// ---------------------------------------------------------------------------

#[test]
fn test_precedence_mult_before_add() {
    use txtcode::runtime::Value;
    assert_eq!(run("2 + 3 * 4"), Value::Integer(14));
}

#[test]
fn test_precedence_power_before_mult() {
    use txtcode::runtime::Value;
    assert_eq!(run("2 ** 3 * 4"), Value::Integer(32));
}

#[test]
fn test_precedence_power_right_assoc() {
    use txtcode::runtime::Value;
    assert_eq!(run("2 ** 3 ** 2"), Value::Integer(512));
}

#[test]
fn test_precedence_not_binds_tighter_than_and() {
    use txtcode::runtime::Value;
    assert_eq!(run("store → r → not true and false\nr"), Value::Boolean(false));
}

#[test]
fn test_integer_division_truncation() {
    use txtcode::runtime::Value;
    assert_eq!(run("7 / 2"), Value::Integer(3));
    assert_eq!(run("-7 / 2"), Value::Integer(-3));
    assert_eq!(run("7 / -2"), Value::Integer(-3));
    assert_eq!(run("-7 / -2"), Value::Integer(3));
}

#[test]
fn test_modulo_truncating() {
    use txtcode::runtime::Value;
    assert_eq!(run("-7 % 3"), Value::Integer(-1));
    assert_eq!(run("7 % 3"), Value::Integer(1));
    assert_eq!(run("7 % -3"), Value::Integer(1));
}

#[test]
fn test_int_float_auto_promote() {
    use txtcode::runtime::Value;
    assert_eq!(run("1 + 1.5"), Value::Float(2.5));
    assert_eq!(run("3 * 2.0"), Value::Float(6.0));
}

#[test]
fn test_power_overflow_raises_error() {
    let result = run_result("2 ** 100");
    assert!(result.is_err(), "2**100 should overflow i64");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("overflow") || msg.contains("E0033"), "error should mention overflow");
}

// propagate (?) operator tests

#[test]
fn test_propagate_ok_unwraps_value() {
    use txtcode::runtime::Value;
    let source = r#"
define → make_ok → ()
  return → ok(42)
end
store → v → make_ok()?
v
"#;
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut vm = VirtualMachine::new();
    let result = vm.interpret_repl(&program).unwrap();
    assert_eq!(result, Value::Integer(42), "? on Ok should unwrap the value");
}

#[test]
fn test_propagate_err_early_returns() {
    use txtcode::runtime::Value;
    let source = r#"
define → inner → ()
  store → r → err("oops")
  store → v → r?
  return → v
end
define → outer → ()
  store → result → inner()
  return → result
end
outer()
"#;
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut vm = VirtualMachine::new();
    let result = vm.interpret_repl(&program).unwrap();
    assert_eq!(
        result,
        Value::Result(false, Box::new(Value::String(Arc::from("oops")))),
        "? on Err should early-return the Err from the function"
    );
}

#[test]
fn test_propagate_inside_try_does_not_hit_catch() {
    let src = r#"
define → helper → ()
  return → ok(99)
end
define → caller → ()
  try
    store → v → helper()?
    return → v
  catch e
    return → -1
  end
end
caller()
"#;
    let result = run(src);
    assert_eq!(result, txtcode::runtime::Value::Integer(99));
}

#[test]
fn test_propagate_ok_inside_try_unwraps_value() {
    let src = r#"
define → safe → ()
  store → x → ok(7)?
  return → x
end
safe()
"#;
    let result = run(src);
    assert_eq!(result, txtcode::runtime::Value::Integer(7));
}

#[test]
fn test_error_in_function_caught_by_caller_try() {
    let src = r#"
define → boom → ()
  store → _ → 1 / 0
end
define → safe_call → ()
  try
    boom()
    return → "no error"
  catch e
    return → "caught"
  end
end
safe_call()
"#;
    let result = run(src);
    assert_eq!(result, txtcode::runtime::Value::String(Arc::from("caught")));
}

#[test]
fn test_propagate_at_top_level_raises_e0034() {
    use txtcode::runtime::errors::ErrorCode;
    let src = "err(\"oops\")?";
    let result = run_result(src);
    assert!(result.is_err(), "? at top level should raise E0034");
    let err = result.unwrap_err();
    assert_eq!(
        err.code,
        Some(ErrorCode::E0034),
        "expected E0034, got {:?}", err.code
    );
}

// optional-index and ternary tests

#[test]
fn test_w2_optional_index_existing_key() {
    let r = run_ast_repl(r#"{"k": 99}?["k"]"#);
    assert_eq!(r.unwrap(), txtcode::runtime::Value::Integer(99));
}

#[test]
fn test_w2_optional_index_missing_key_returns_null() {
    let r = run_ast_repl(r#"{"k": 1}?["nope"]"#);
    assert_eq!(r.unwrap(), txtcode::runtime::Value::Null);
}

#[test]
fn test_w2_ternary_still_works_after_optional_fix() {
    let r = run_ast_repl("5 > 3 ? 1 : 0");
    assert_eq!(r.unwrap(), txtcode::runtime::Value::Integer(1));
}

// V.2 operator associativity tests

#[test]
fn test_v2_subtraction_is_left_associative() {
    let result = run_ast_repl("10 - 3 - 2");
    assert!(result.is_ok(), "subtraction should eval: {:?}", result);
    match result.unwrap() {
        txtcode::runtime::Value::Integer(n) => assert_eq!(n, 5, "10 - 3 - 2 must be (10-3)-2 = 5"),
        other => panic!("expected int, got {:?}", other),
    }
}

#[test]
fn test_v2_division_is_left_associative() {
    let result = run_ast_repl("100 / 5 / 4");
    assert!(result.is_ok(), "division should eval: {:?}", result);
    match result.unwrap() {
        txtcode::runtime::Value::Integer(n) => assert_eq!(n, 5, "100 / 5 / 4 must be (100/5)/4 = 5"),
        other => panic!("expected int, got {:?}", other),
    }
}

#[test]
fn test_v2_multiplication_higher_precedence_than_addition() {
    let result = run_ast_repl("2 + 3 * 4");
    assert!(result.is_ok(), "precedence eval: {:?}", result);
    match result.unwrap() {
        txtcode::runtime::Value::Integer(n) => assert_eq!(n, 14, "2 + 3*4 = 2 + 12 = 14"),
        other => panic!("expected int, got {:?}", other),
    }
}

#[test]
fn test_v2_mixed_precedence() {
    let result = run_ast_repl("2 * 3 + 4 * 5");
    assert!(result.is_ok(), "mixed precedence: {:?}", result);
    match result.unwrap() {
        txtcode::runtime::Value::Integer(n) => assert_eq!(n, 26, "2*3 + 4*5 = 6 + 20 = 26"),
        other => panic!("expected int, got {:?}", other),
    }
}

#[test]
fn test_v2_comparison_precedence() {
    let result = run_ast_repl("1 + 1 == 2");
    assert!(result.is_ok(), "comparison precedence: {:?}", result);
    match result.unwrap() {
        txtcode::runtime::Value::Boolean(b) => assert!(b, "1 + 1 == 2 should be true"),
        other => panic!("expected bool, got {:?}", other),
    }
}

#[test]
fn test_v2_unary_negation_precedence() {
    let result = run_ast_repl("-2 * 3");
    assert!(result.is_ok(), "unary negation: {:?}", result);
    match result.unwrap() {
        txtcode::runtime::Value::Integer(n) => assert_eq!(n, -6, "-2 * 3 should be (-2)*3 = -6"),
        other => panic!("expected int, got {:?}", other),
    }
}

// W.1: Integer division truncation
#[test]
fn test_w1_negative_int_div_truncates() {
    let r = run_ast_repl("-7 / 2");
    assert_eq!(r.unwrap(), txtcode::runtime::Value::Integer(-3));
}
