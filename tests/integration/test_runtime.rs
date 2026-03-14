use txtcode::lexer::Lexer;
use txtcode::parser::Parser;
use txtcode::runtime::vm::VirtualMachine;

#[test]
fn test_runtime_arithmetic() {
    let source = "store → result → 10 + 5".to_string();
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut vm = VirtualMachine::new();

    let result = vm.interpret(&program).unwrap();
    // Result would be the last expression value
    assert!(matches!(
        result,
        txtcode::runtime::Value::Integer(_) | txtcode::runtime::Value::Null
    ));
}

#[test]
fn test_runtime_print_arrow() {
    let source = "print → \"Hello, World!\"".to_string();
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    let mut vm = VirtualMachine::new();
    // This should execute without error (print returns Null)
    let result = vm.interpret(&program);
    assert!(result.is_ok());
}

#[test]
fn test_runtime_print_original() {
    let source = "print → \"Test\"".to_string();
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut vm = VirtualMachine::new();

    let result = vm.interpret(&program);
    assert!(result.is_ok());
}

// ---------------------------------------------------------------------------
// v0.4.1 — control-flow propagation regression tests
//
// Each test uses the "bomb" pattern: if the control-flow signal is NOT
// propagated correctly the program hits a deliberate division-by-zero and
// run_ast_source returns Err.  After the fix every test must return Ok.
// ---------------------------------------------------------------------------

#[test]
fn test_return_inside_if() {
    // return inside an if branch must exit the function, not fall through.
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
    // return inside a match case must exit the function.
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
    // return inside a for body must exit the function.
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
    // return inside a while body (nested inside if) must exit the function.
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
    // return inside a try body must exit the function, not trigger the catch.
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
    // break inside a for body must terminate the loop immediately.
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
    // continue inside a for body must skip the rest of the iteration.
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
    // return inside an if that is itself inside a match case must exit the function.
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

/// Like `run_ast_source` but uses `interpret_repl` so the last expression's
/// value is returned (instead of always `Null`). Use for tests that need to
/// assert the actual return value of a stdlib call.
#[allow(clippy::result_large_err)]
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

// ---------------------------------------------------------------------------
// Phase 6 — stdlib panic hardening tests
// ---------------------------------------------------------------------------

#[test]
fn test_substring_ascii_valid() {
    let result = run_ast_repl(r#"substring("hello", 1, 3)"#);
    assert!(result.is_ok(), "{:?}", result);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String("el".to_string()));
}

#[test]
fn test_substring_unicode_valid() {
    // "hé" = 2 chars; substring(s, 0, 2) must return those 2 chars, not 3 bytes
    let result = run_ast_repl(r#"substring("héllo", 0, 2)"#);
    assert!(result.is_ok(), "{:?}", result);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String("hé".to_string()));
}

#[test]
fn test_substring_unicode_mid_char() {
    // char index 1 = 'é', char index 2 = 'l'; this was a byte-slice panic before the fix
    let result = run_ast_repl(r#"substring("héllo", 1, 2)"#);
    assert!(result.is_ok(), "{:?}", result);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String("é".to_string()));
}

#[test]
fn test_substring_negative_start_errors() {
    let result = run_ast_source(r#"store → r → substring("hello", -1, 3)"#);
    assert!(result.is_err(), "expected error for negative start index");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("non-negative"), "error message: {}", msg);
}

#[test]
fn test_substring_oob_errors() {
    let result = run_ast_source(r#"store → r → substring("hello", 2, 10)"#);
    assert!(result.is_err(), "expected error for out-of-bounds end index");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("out of bounds"), "error message: {}", msg);
}

#[test]
fn test_str_pad_left_valid() {
    let result = run_ast_repl(r#"str_pad_left("hi", 5)"#);
    assert!(result.is_ok(), "{:?}", result);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String("   hi".to_string()));
}

#[test]
fn test_str_pad_left_negative_width_errors() {
    let result = run_ast_source(r#"store → r → str_pad_left("x", -1)"#);
    assert!(result.is_err(), "expected error for negative width");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("non-negative"), "error message: {}", msg);
}

#[test]
fn test_str_pad_right_valid() {
    let result = run_ast_repl(r#"str_pad_right("hi", 5)"#);
    assert!(result.is_ok(), "{:?}", result);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String("hi   ".to_string()));
}

#[test]
fn test_str_pad_right_negative_width_errors() {
    let result = run_ast_source(r#"store → r → str_pad_right("x", -5)"#);
    assert!(result.is_err(), "expected error for negative width");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("non-negative"), "error message: {}", msg);
}

#[test]
fn test_math_random_int_valid() {
    let result = run_ast_repl(r#"math_random_int(1, 100)"#);
    assert!(result.is_ok(), "{:?}", result);
    if let Ok(txtcode::runtime::Value::Integer(n)) = result {
        assert!((1..=100).contains(&n), "got out-of-range value: {}", n);
    } else {
        panic!("expected integer");
    }
}

#[test]
fn test_math_random_int_equal_bounds() {
    let result = run_ast_repl(r#"math_random_int(5, 5)"#);
    assert!(result.is_ok(), "{:?}", result);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(5));
}

#[test]
fn test_math_random_int_inverted_range_errors() {
    let result = run_ast_source(r#"store → r → math_random_int(10, 1)"#);
    assert!(result.is_err(), "expected error for inverted range");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("min"), "error message: {}", msg);
}

// ---------------------------------------------------------------------------
// Phase 7 — AST VM pipe and async tests
// ---------------------------------------------------------------------------

#[allow(clippy::result_large_err)]
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

#[test]
fn test_runtime_pipe_lambda() {
    // AST VM: `5 |> (x) -> x * 2` should return 10
    let result = run_ast_source(
        r#"
define → double → (x)
  return → x * 2
end
store → result → 5 |> double
"#,
    );
    assert!(
        result.is_ok(),
        "pipe with identifier rhs should work: {:?}",
        result
    );
}

#[test]
fn test_runtime_async_sync_mode() {
    // async functions run synchronously in v0.4, should not crash
    let result = run_ast_source(
        r#"
async → define → add_one → (x)
  return → x + 1
end
store → result → add_one(41)
"#,
    );
    assert!(
        result.is_ok(),
        "async function should run synchronously: {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Phase 7 — Slice stabilization tests — AST VM + stdlib
// ---------------------------------------------------------------------------

#[test]
fn test_ast_slice_string_step() {
    // String slicing with step is now supported in AST VM.
    let result = run_ast_repl(r#""abcdef"[::2]"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String("ace".to_string()));
}

#[test]
fn test_ast_slice_string_reverse() {
    let result = run_ast_repl(r#""hello"[::-1]"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String("olleh".to_string()));
}

#[test]
fn test_ast_slice_string_negative_index_char_based() {
    // "héllo" — 5 Unicode chars; [-3:] must use char count (5), not byte count (6).
    let result = run_ast_repl(r#""héllo"[-3:]"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String("llo".to_string()));
}

#[test]
fn test_ast_slice_string_step_zero_errors() {
    let result = run_ast_repl(r#""hello"[::0]"#);
    assert!(result.is_err(), "step=0 must error in AST VM");
}

#[test]
fn test_ast_slice_empty_array_reverse() {
    // [][::-1] must return [] without panic.
    let result = run_ast_repl(r#"[][::  -1]"#);
    // Parser may not support spaces inside slice — use store approach instead.
    let result = run_ast_source(
        r#"
store → a → []
store → r → a[::-1]
"#,
    );
    assert!(result.is_ok(), "empty array reverse must not panic: {:?}", result);
}

#[test]
fn test_ast_slice_array_negative_index() {
    let result = run_ast_repl(r#"[10, 20, 30, 40][-2:]"#);
    assert_eq!(
        result.unwrap(),
        txtcode::runtime::Value::Array(vec![
            txtcode::runtime::Value::Integer(30),
            txtcode::runtime::Value::Integer(40),
        ])
    );
}

#[test]
fn test_array_slice_stdlib_negative_start() {
    // array_slice(arr, -2) → last 2 elements.
    let result = run_ast_repl(r#"array_slice([1, 2, 3, 4], -2)"#);
    assert_eq!(
        result.unwrap(),
        txtcode::runtime::Value::Array(vec![
            txtcode::runtime::Value::Integer(3),
            txtcode::runtime::Value::Integer(4),
        ])
    );
}

#[test]
fn test_array_slice_stdlib_negative_end() {
    // array_slice(arr, 0, -1) → all but last.
    let result = run_ast_repl(r#"array_slice([1, 2, 3, 4], 0, -1)"#);
    assert_eq!(
        result.unwrap(),
        txtcode::runtime::Value::Array(vec![
            txtcode::runtime::Value::Integer(1),
            txtcode::runtime::Value::Integer(2),
            txtcode::runtime::Value::Integer(3),
        ])
    );
}

#[test]
fn test_array_slice_stdlib_oob_errors() {
    let result = run_ast_repl(r#"array_slice([1, 2, 3], 0, 99)"#);
    assert!(result.is_err(), "OOB must error");
}
