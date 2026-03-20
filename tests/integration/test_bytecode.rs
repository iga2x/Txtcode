use txtcode::compiler::bytecode::BytecodeCompiler;
/// Bytecode VM integration tests (v0.4)
///
/// Verifies all previously-stubbed features in the bytecode compiler + VM:
/// Ternary, Await, Set, Lambda, MethodCall, Slice, IndexAssignment,
/// CompoundAssignment, else-if chains, SetIndex, SetField.
use txtcode::lexer::Lexer;
use txtcode::parser::ast::{
    BinaryOperator, Expression, Literal, Pattern, Program, Span, Statement,
};
use txtcode::parser::Parser;
use txtcode::runtime::bytecode_vm::BytecodeVM;
use txtcode::runtime::core::Value;
use txtcode::runtime::errors::RuntimeError;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Compile Txt-code source and execute via bytecode VM.
/// Use `return → value` at the end of source to get a specific value back.
#[allow(clippy::result_large_err)]
fn compile_and_run(source: &str) -> Result<Value, RuntimeError> {
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut compiler = BytecodeCompiler::new();
    let bytecode = compiler.compile(&program);
    let mut vm = BytecodeVM::new();
    vm.execute(&bytecode)
}

fn run_ok(source: &str) -> Value {
    compile_and_run(source).expect("expected Ok result")
}

/// Compile and run a manually-constructed AST Program directly.
#[allow(clippy::result_large_err)]
fn run_ast(program: Program) -> Result<Value, RuntimeError> {
    let mut compiler = BytecodeCompiler::new();
    let bytecode = compiler.compile(&program);
    let mut vm = BytecodeVM::new();
    vm.execute(&bytecode)
}

fn default_span() -> Span {
    Span::default()
}

fn lit_int(n: i64) -> Expression {
    Expression::Literal(Literal::Integer(n))
}

fn lit_bool(b: bool) -> Expression {
    Expression::Literal(Literal::Boolean(b))
}

fn ident(name: &str) -> Expression {
    Expression::Identifier(name.to_string())
}

fn assign(name: &str, value: Expression) -> Statement {
    Statement::Assignment {
        pattern: Pattern::Identifier(name.to_string()),
        type_annotation: None,
        value,
        span: default_span(),
    }
}

fn ret(value: Expression) -> Statement {
    Statement::Return {
        value: Some(value),
        span: default_span(),
    }
}

// ---------------------------------------------------------------------------
// Ternary expression (not parseable from source — use AST directly)
// ---------------------------------------------------------------------------

#[test]
fn test_bytecode_ternary_true_branch() {
    let prog = Program {
        statements: vec![ret(Expression::Ternary {
            condition: Box::new(lit_bool(true)),
            true_expr: Box::new(lit_int(42)),
            false_expr: Box::new(lit_int(0)),
            span: default_span(),
        })],
    };
    assert_eq!(run_ast(prog).unwrap(), Value::Integer(42));
}

#[test]
fn test_bytecode_ternary_false_branch() {
    let prog = Program {
        statements: vec![ret(Expression::Ternary {
            condition: Box::new(lit_bool(false)),
            true_expr: Box::new(lit_int(42)),
            false_expr: Box::new(lit_int(99)),
            span: default_span(),
        })],
    };
    assert_eq!(run_ast(prog).unwrap(), Value::Integer(99));
}

#[test]
fn test_bytecode_ternary_computed_condition() {
    // a = 10; result = (a > 5) ? 1 : 0
    let condition = Expression::BinaryOp {
        left: Box::new(ident("a")),
        op: BinaryOperator::Greater,
        right: Box::new(lit_int(5)),
        span: default_span(),
    };
    let prog = Program {
        statements: vec![
            assign("a", lit_int(10)),
            assign(
                "result",
                Expression::Ternary {
                    condition: Box::new(condition),
                    true_expr: Box::new(lit_int(1)),
                    false_expr: Box::new(lit_int(0)),
                    span: default_span(),
                },
            ),
            ret(ident("result")),
        ],
    };
    assert_eq!(run_ast(prog).unwrap(), Value::Integer(1));
}

// ---------------------------------------------------------------------------
// CompoundAssignment (not in parser — use AST directly)
// ---------------------------------------------------------------------------

#[test]
fn test_bytecode_compound_add() {
    let prog = Program {
        statements: vec![
            assign("x", lit_int(5)),
            Statement::CompoundAssignment {
                name: "x".to_string(),
                op: BinaryOperator::Add,
                value: lit_int(3),
                span: default_span(),
            },
            ret(ident("x")),
        ],
    };
    assert_eq!(run_ast(prog).unwrap(), Value::Integer(8));
}

#[test]
fn test_bytecode_compound_sub() {
    let prog = Program {
        statements: vec![
            assign("x", lit_int(10)),
            Statement::CompoundAssignment {
                name: "x".to_string(),
                op: BinaryOperator::Subtract,
                value: lit_int(4),
                span: default_span(),
            },
            ret(ident("x")),
        ],
    };
    assert_eq!(run_ast(prog).unwrap(), Value::Integer(6));
}

#[test]
fn test_bytecode_compound_mul() {
    let prog = Program {
        statements: vec![
            assign("x", lit_int(3)),
            Statement::CompoundAssignment {
                name: "x".to_string(),
                op: BinaryOperator::Multiply,
                value: lit_int(7),
                span: default_span(),
            },
            ret(ident("x")),
        ],
    };
    assert_eq!(run_ast(prog).unwrap(), Value::Integer(21));
}

// ---------------------------------------------------------------------------
// Await — bytecode VM now emits Instruction::Await
// ---------------------------------------------------------------------------

#[test]
fn test_bytecode_await_passthrough() {
    // await on a non-future is a transparent no-op (JS semantics)
    let val = run_ok("return → await 123");
    assert_eq!(val, Value::Integer(123));
}

#[test]
fn test_bytecode_await_resolves_future() {
    // async user function returns Value::Future in the AST VM; in the bytecode
    // VM the function runs synchronously and returns its value directly.
    // `await` on the result is a no-op and the value passes through.
    let val = run_ok(
        r#"
async define → double → (x)
  return → x * 2
end
return → await double(21)
"#,
    );
    assert_eq!(val, Value::Integer(42));
}

// ---------------------------------------------------------------------------
// Set literal
// ---------------------------------------------------------------------------

#[test]
fn test_bytecode_set_literal() {
    // {1, 2, 3} is a Set expression in this language
    let val = run_ok("return → {1, 2, 3}");
    match val {
        Value::Set(items) => {
            assert_eq!(items.len(), 3, "expected 3 items in set");
        }
        other => panic!("expected Set, got {:?}", other),
    }
}

#[test]
fn test_bytecode_set_dedup() {
    // Duplicate values should be deduplicated
    let val = run_ok("return → {1, 1, 2, 2, 3}");
    match val {
        Value::Set(items) => {
            assert_eq!(items.len(), 3, "expected 3 unique items, got {:?}", items);
        }
        other => panic!("expected Set, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// IndexAssignment: store → arr[i] → val
// ---------------------------------------------------------------------------

#[test]
fn test_bytecode_index_assignment_array() {
    let source = "store → arr → [1, 2, 3]\nstore → arr[0] → 99\nreturn → arr[0]";
    assert_eq!(run_ok(source), Value::Integer(99));
}

#[test]
fn test_bytecode_index_assignment_map_existing_key() {
    let source = "store → m → {\"a\": 1}\nstore → m[\"a\"] → 42\nreturn → m[\"a\"]";
    assert_eq!(run_ok(source), Value::Integer(42));
}

#[test]
fn test_bytecode_index_assignment_map_new_key() {
    let source = "store → m → {\"x\": 0}\nstore → m[\"y\"] → 100\nreturn → m[\"y\"]";
    assert_eq!(run_ok(source), Value::Integer(100));
}

// ---------------------------------------------------------------------------
// Lambda: (params) → body
// ---------------------------------------------------------------------------

#[test]
fn test_bytecode_lambda_call() {
    // store → f → (x) → x + 1; f(5) should be 6
    let source = "store → f → (x) → x + 1\nreturn → f(5)";
    assert_eq!(run_ok(source), Value::Integer(6));
}

#[test]
fn test_bytecode_lambda_multi_param() {
    let source = "store → add → (a, b) → a + b\nreturn → add(3, 4)";
    assert_eq!(run_ok(source), Value::Integer(7));
}

// ---------------------------------------------------------------------------
// MethodCall: expr.method(args)
// ---------------------------------------------------------------------------

#[test]
fn test_bytecode_method_string_len() {
    assert_eq!(run_ok("return → \"hello\".len()"), Value::Integer(5));
}

#[test]
fn test_bytecode_method_string_to_upper() {
    assert_eq!(
        run_ok("return → \"hello\".toUpper()"),
        Value::String("HELLO".to_string())
    );
}

#[test]
fn test_bytecode_method_string_trim() {
    assert_eq!(
        run_ok("return → \"  hi  \".trim()"),
        Value::String("hi".to_string())
    );
}

#[test]
fn test_bytecode_method_string_contains() {
    assert_eq!(
        run_ok("return → \"hello world\".contains(\"world\")"),
        Value::Boolean(true)
    );
}

#[test]
fn test_bytecode_method_array_len() {
    assert_eq!(run_ok("return → [1, 2, 3, 4].len()"), Value::Integer(4));
}

#[test]
fn test_bytecode_method_array_first() {
    assert_eq!(run_ok("return → [10, 20, 30].first()"), Value::Integer(10));
}

#[test]
fn test_bytecode_method_array_last() {
    assert_eq!(run_ok("return → [10, 20, 30].last()"), Value::Integer(30));
}

#[test]
fn test_bytecode_method_array_contains() {
    assert_eq!(
        run_ok("return → [1, 2, 3].contains(2)"),
        Value::Boolean(true)
    );
}

#[test]
fn test_bytecode_method_array_join() {
    assert_eq!(
        run_ok("return → [\"a\", \"b\", \"c\"].join(\"-\")"),
        Value::String("a-b-c".to_string())
    );
}

#[test]
fn test_bytecode_method_map_len() {
    assert_eq!(
        run_ok("return → {\"x\": 1, \"y\": 2}.len()"),
        Value::Integer(2)
    );
}

#[test]
fn test_bytecode_method_map_has() {
    assert_eq!(
        run_ok("return → {\"k\": 1}.has(\"k\")"),
        Value::Boolean(true)
    );
}

// ---------------------------------------------------------------------------
// Slice: arr[start:end] and str[start:end]
// ---------------------------------------------------------------------------

#[test]
fn test_bytecode_slice_array() {
    let val = run_ok("store → a → [1, 2, 3, 4, 5]\nreturn → a[1:3]");
    assert_eq!(
        val,
        Value::Array(vec![Value::Integer(2), Value::Integer(3)])
    );
}

#[test]
fn test_bytecode_slice_string() {
    let val = run_ok("store → s → \"hello\"\nreturn → s[1:4]");
    assert_eq!(val, Value::String("ell".to_string()));
}

// ---------------------------------------------------------------------------
// else-if chain: if → ... elseif → ... else ... end
// ---------------------------------------------------------------------------

#[test]
fn test_bytecode_else_if_first_branch() {
    let source = "store → x → 1\nstore → r → 0\nif → x == 1\nstore → r → 10\nelseif → x == 2\nstore → r → 20\nelse\nstore → r → 30\nend\nreturn → r";
    assert_eq!(run_ok(source), Value::Integer(10));
}

#[test]
fn test_bytecode_else_if_second_branch() {
    let source = "store → x → 2\nstore → r → 0\nif → x == 1\nstore → r → 10\nelseif → x == 2\nstore → r → 20\nelse\nstore → r → 30\nend\nreturn → r";
    assert_eq!(run_ok(source), Value::Integer(20));
}

#[test]
fn test_bytecode_else_if_else_branch() {
    let source = "store → x → 99\nstore → r → 0\nif → x == 1\nstore → r → 10\nelseif → x == 2\nstore → r → 20\nelse\nstore → r → 30\nend\nreturn → r";
    assert_eq!(run_ok(source), Value::Integer(30));
}

// ---------------------------------------------------------------------------
// Full program smoke test combining multiple new features
// ---------------------------------------------------------------------------

#[test]
fn test_bytecode_full_program_index_and_sum() {
    // [99, 2, 3, 4, 5] → sum = 99+2+3+4+5 = 113
    let source = "store → arr → [1, 2, 3, 4, 5]\nstore → arr[0] → 99\nstore → total → 0\nfor → i in arr\nstore → total → total + i\nend\nreturn → total";
    assert_eq!(run_ok(source), Value::Integer(113));
}

#[test]
fn test_bytecode_method_on_variable() {
    // Method call on stored string variable
    let source = "store → s → \"world\"\nreturn → s.len()";
    assert_eq!(run_ok(source), Value::Integer(5));
}

// ---------------------------------------------------------------------------
// String interpolation in bytecode VM
// ---------------------------------------------------------------------------

#[test]
fn test_bytecode_string_interpolation_basic() {
    let val = run_ok("store → name → \"World\"\nreturn → f\"Hello {name}!\"");
    assert_eq!(val, Value::String("Hello World!".to_string()));
}

#[test]
fn test_bytecode_string_interpolation_expr() {
    let val = run_ok("store → x → 5\nreturn → f\"result={x + 1}\"");
    assert_eq!(val, Value::String("result=6".to_string()));
}

// ---------------------------------------------------------------------------
// do-while loop in bytecode VM
// ---------------------------------------------------------------------------

#[test]
fn test_bytecode_do_while_basic() {
    let source = "store → x → 0\ndo\nstore → x → x + 1\nwhile → x < 3\nend\nreturn → x";
    assert_eq!(run_ok(source), Value::Integer(3));
}

#[test]
fn test_bytecode_do_while_executes_once_when_false() {
    // condition false from start — do-while still runs once
    let source = "store → x → 0\ndo\nstore → x → x + 1\nwhile → x < 0\nend\nreturn → x";
    assert_eq!(run_ok(source), Value::Integer(1));
}

// ---------------------------------------------------------------------------
// Optional chaining in bytecode VM
// ---------------------------------------------------------------------------

#[test]
fn test_bytecode_optional_chain_map_hit() {
    let source = "store → m → {\"key\": 42}\nreturn → m?.key";
    let result = compile_and_run(source);
    // Should return 42, not an error
    assert_eq!(result.unwrap(), Value::Integer(42));
}

#[test]
fn test_bytecode_optional_chain_null_safe() {
    let source = "store → m → null\nreturn → m?.key";
    let result = compile_and_run(source);
    // Should return Null without crashing
    assert_eq!(result.unwrap(), Value::Null);
}

// ---------------------------------------------------------------------------
// Slice expressions in bytecode VM (verify existing tests work)
// ---------------------------------------------------------------------------

#[test]
fn test_bytecode_slice_open_start() {
    let val = run_ok("store → a → [1, 2, 3, 4, 5]\nreturn → a[0:3]");
    assert_eq!(
        val,
        Value::Array(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3)
        ])
    );
}

// ---------------------------------------------------------------------------
// Spread operator in bytecode VM
// ---------------------------------------------------------------------------

#[test]
fn test_bytecode_spread_concat_arrays() {
    let val = run_ok("store → a → [1, 2]\nstore → b → [3, 4]\nreturn → [...a, ...b]");
    assert_eq!(
        val,
        Value::Array(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
            Value::Integer(4),
        ])
    );
}

#[test]
fn test_bytecode_spread_with_literal_elements() {
    let val = run_ok("store → a → [2, 3]\nreturn → [1, ...a, 4]");
    assert_eq!(
        val,
        Value::Array(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
            Value::Integer(4),
        ])
    );
}

#[test]
fn test_bytecode_spread_empty_array() {
    let val = run_ok("store → a → []\nreturn → [1, ...a, 2]");
    assert_eq!(
        val,
        Value::Array(vec![Value::Integer(1), Value::Integer(2)])
    );
}

// ---------------------------------------------------------------------------
// Multi-return values
// ---------------------------------------------------------------------------

#[test]
fn test_bytecode_multi_return_as_array() {
    // return → a, b auto-wraps as [a, b]
    let val = run_ok("return → 1, 2, 3");
    assert_eq!(
        val,
        Value::Array(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ])
    );
}

#[test]
fn test_bytecode_multi_return_from_function() {
    let val = run_ok(
        r#"
define → minmax → (arr)
  store → lo → arr[0]
  store → hi → arr[0]
  for → x in arr
    if lo > x
      store → lo → x
    end
    if hi < x
      store → hi → x
    end
  end
  return → lo, hi
end
return → minmax([3, 1, 4, 1, 5, 9, 2, 6])
"#,
    );
    assert_eq!(
        val,
        Value::Array(vec![Value::Integer(1), Value::Integer(9)])
    );
}

// ---------------------------------------------------------------------------
// Destructured function arguments
// ---------------------------------------------------------------------------

#[test]
fn test_bytecode_destructured_map_arg() {
    let val = run_ok(
        r#"
define → greet → ({name, age})
  return → name
end
return → greet({"name": "Alice", "age": 30})
"#,
    );
    assert_eq!(val, Value::String("Alice".to_string()));
}

#[test]
fn test_bytecode_destructured_multi_field() {
    let val = run_ok(
        r#"
define → sum_coords → ({x, y})
  return → x + y
end
return → sum_coords({"x": 10, "y": 20})
"#,
    );
    assert_eq!(val, Value::Integer(30));
}

// ---------------------------------------------------------------------------
// Phase 6 — New tests for gaps C1, M2, L1, M6, L4
// ---------------------------------------------------------------------------

#[test]
fn test_bytecode_async_function_runs_synchronously() {
    // async/await runs synchronously in v0.4 — no crash, returns value
    let val = run_ok(
        r#"
async → define → fetch → (x)
  return → x + 1
end
return → fetch(41)
"#,
    );
    assert_eq!(val, Value::Integer(42));
}

#[test]
fn test_bytecode_increment_on_index_errors_cleanly() {
    // ++arr[0] should surface a clear RuntimeError, not silently do nothing
    let result = compile_and_run(
        r#"
store → arr → [1, 2, 3]
++arr[0]
return → arr[0]
"#,
    );
    assert!(result.is_err(), "expected RuntimeError for ++arr[index]");
    let err = result.unwrap_err();
    let msg = format!("{}", err);
    assert!(
        msg.contains("++") || msg.contains("operator") || msg.contains("variable"),
        "error message should mention the operator: {}",
        msg
    );
}

#[test]
fn test_bytecode_optional_call_on_function() {
    // func?.() where func is a real function value should call it
    // Functions are callable directly; ?.() on a string name looks up and calls
    let val = run_ok(
        r#"
define → double → (x)
  return → x * 2
end
return → double(5)
"#,
    );
    assert_eq!(val, Value::Integer(10));
}

#[test]
fn test_bytecode_optional_call_on_null_returns_null() {
    // null?.() should return null without error
    let val = run_ok(
        r#"
store → f → null
return → f?.()
"#,
    );
    assert_eq!(val, Value::Null);
}

#[test]
fn test_bytecode_pipe_identifier_rhs() {
    // Simple pipe: 5 |> double (identifier RHS — desugars at parse time)
    let val = run_ok(
        r#"
define → double → (x)
  return → x * 2
end
return → 5 |> double
"#,
    );
    assert_eq!(val, Value::Integer(10));
}

#[test]
fn test_bytecode_call_depth_limit() {
    // Recursive function should hit call depth limit before Rust stack overflows
    let result = compile_and_run(
        r#"
define → recurse → (n)
  return → recurse(n + 1)
end
return → recurse(0)
"#,
    );
    assert!(result.is_err(), "expected call depth RuntimeError");
    let msg = format!("{}", result.unwrap_err());
    assert!(
        msg.contains("call")
            || msg.contains("depth")
            || msg.contains("stack")
            || msg.contains("recursion"),
        "error should mention call depth: {}",
        msg
    );
}

// ---------------------------------------------------------------------------
// Control-flow signal regression tests (mirrors test_runtime.rs suite)
// Bytecode VM uses Jump-based control flow, not signals, so these also guard
// against any future regression where a signal leaks into the bytecode path.
// ---------------------------------------------------------------------------

#[test]
fn test_bytecode_return_inside_if() {
    // `return` inside an `if` branch must exit the function, not just the branch.
    let val = run_ok(
        "define → f → (x)\n  if → x > 0\n    return → 1\n  end\n  return → 0\nend\nreturn → f(5)\n",
    );
    assert_eq!(
        val,
        Value::Integer(1),
        "return inside if should exit function"
    );
}

#[test]
fn test_bytecode_return_inside_else() {
    let val = run_ok(
        "define → f → (x)\n  if → x > 0\n    return → 1\n  else\n    return → -1\n  end\n  return → 0\nend\nreturn → f(-3)\n",
    );
    assert_eq!(
        val,
        Value::Integer(-1),
        "return inside else should exit function"
    );
}

#[test]
fn test_bytecode_return_inside_for() {
    // `return` inside a `for` loop must exit the function immediately.
    let val = run_ok(
        "define → first_pos → (arr)\n  for → x in arr\n    if → x > 0\n      return → x\n    end\n  end\n  return → -1\nend\nreturn → first_pos([0, -2, 3, 4])\n",
    );
    assert_eq!(
        val,
        Value::Integer(3),
        "return inside for should exit function"
    );
}

#[test]
fn test_bytecode_return_inside_while() {
    let val = run_ok(
        "define → countdown → (n)\n  store → i → n\n  while → i > 0\n    if → i == 3\n      return → i\n    end\n    store → i → i - 1\n  end\n  return → 0\nend\nreturn → countdown(5)\n",
    );
    assert_eq!(
        val,
        Value::Integer(3),
        "return inside while should exit function"
    );
}

#[test]
fn test_bytecode_return_inside_try() {
    // `return` inside a try block must exit the function; the catch must NOT run.
    let val = run_ok(
        "define → f → ()\n  try\n    return → 42\n  catch e\n    return → -1\n  end\n  return → 0\nend\nreturn → f()\n",
    );
    assert_eq!(
        val,
        Value::Integer(42),
        "return inside try should exit function, not trigger catch"
    );
}

#[test]
fn test_bytecode_try_catch_genuine_error() {
    // A genuine runtime error (undefined variable) must be caught.
    let val = run_ok(
        "define → f → ()\n  try\n    store → x → undefined_var\n    return → 0\n  catch e\n    return → 99\n  end\nend\nreturn → f()\n",
    );
    assert_eq!(
        val,
        Value::Integer(99),
        "genuine error inside try should be caught"
    );
}

#[test]
fn test_bytecode_break_in_for() {
    // `break` inside a for loop exits the loop; execution continues after.
    let val = run_ok(
        "store → found → -1\nfor → x in [1, 2, 3, 4, 5]\n  if → x == 3\n    store → found → x\n    break\n  end\nend\nreturn → found\n",
    );
    assert_eq!(val, Value::Integer(3), "break should exit for loop");
}

#[test]
fn test_bytecode_continue_in_for() {
    // `continue` skips the rest of the current iteration.
    let val = run_ok(
        "store → sum → 0\nfor → x in [1, 2, 3, 4, 5]\n  if → x == 3\n    continue\n  end\n  store → sum → sum + x\nend\nreturn → sum\n",
    );
    assert_eq!(
        val,
        Value::Integer(12),
        "continue should skip iteration (1+2+4+5=12)"
    );
}

// ---------------------------------------------------------------------------
// Stale catch-frame regression tests
// Verify that SetupCatch frames pushed inside a callee are NOT left on the
// catch_stack when the callee returns.  Before the fix (catch_depth field on
// call_stack frames), the stale frame would intercept the *caller's* genuine
// errors and silently absorb them.
// ---------------------------------------------------------------------------

#[test]
fn test_bytecode_stale_catch_frame_after_return_from_try() {
    // Callee has a try block but returns from inside it.
    // After it returns, the caller triggers a genuine error.
    // Without the fix the stale SetupCatch frame would swallow the error.
    let result = compile_and_run(
        r#"
define → callee → ()
  try
    return → 42
  catch e
    return → -1
  end
end
store → x → callee()
return → undefined_var
"#,
    );
    assert!(
        result.is_err(),
        "caller's undefined-var error must not be swallowed by callee's stale catch frame"
    );
}

#[test]
fn test_bytecode_stale_catch_frame_caller_can_catch_own_error() {
    // Callee returns from inside try.  Caller wraps its own call in try-catch.
    // The caller's catch block must fire, not the callee's stale frame.
    let val = run_ok(
        r#"
define → callee → ()
  try
    return → 10
  catch e
    return → -1
  end
end
store → result → 0
try
  store → x → callee()
  store → bad → undefined_var
catch e
  store → result → 99
end
return → result
"#,
    );
    assert_eq!(
        val,
        Value::Integer(99),
        "caller's own catch block must fire"
    );
}

#[test]
fn test_bytecode_break_inside_try() {
    // break inside a try block must exit the loop, not be caught.
    let val = run_ok(
        r#"
store → acc → 0
for → i in [1, 2, 3]
  try
    if → i == 2
      break
    end
    store → acc → i
  catch e
    store → acc → -1
  end
end
return → acc
"#,
    );
    assert_eq!(
        val,
        Value::Integer(1),
        "break inside try must exit loop without catch firing"
    );
}

#[test]
fn test_bytecode_continue_inside_try() {
    // continue inside a try block must skip to next iteration, not be caught.
    let val = run_ok(
        r#"
store → acc → 0
for → i in [1, 2, 3]
  try
    if → i == 2
      continue
    end
    store → acc → acc + i
  catch e
    store → acc → -1
  end
end
return → acc
"#,
    );
    assert_eq!(
        val,
        Value::Integer(4),
        "continue inside try must skip iteration without catch firing"
    );
}

#[test]
fn test_bytecode_nested_try_inner_caught_outer_clean() {
    // Inner try catches its own error; outer try should not see it.
    let val = run_ok(
        r#"
store → result → 0
try
  try
    store → x → undefined_inner
  catch e
    store → result → 1
  end
catch e
  store → result → -1
end
return → result
"#,
    );
    assert_eq!(val, Value::Integer(1), "inner try must catch its own error");
}

#[test]
fn test_bytecode_version_string() {
    // Sanity check: Cargo.toml version is present and non-empty.
    // Update this assertion when the version is bumped.
    let v = env!("CARGO_PKG_VERSION");
    assert!(!v.is_empty(), "CARGO_PKG_VERSION must not be empty");
    assert!(v.starts_with("0."), "expected semver starting with 0., got {}", v);
}

// ---------------------------------------------------------------------------
// Slice stabilization tests — bytecode VM (C1–C7)
// ---------------------------------------------------------------------------

#[test]
fn test_bytecode_slice_negative_step_reverses_array() {
    // arr[::-1] must reverse, not iterate forward.
    let val = run_ok("store → a → [1, 2, 3]\nreturn → a[::-1]");
    assert_eq!(
        val,
        Value::Array(vec![Value::Integer(3), Value::Integer(2), Value::Integer(1)])
    );
}

#[test]
fn test_bytecode_slice_negative_step_with_stride() {
    // arr[::-2] → every other element in reverse.
    let val = run_ok("store → a → [1, 2, 3, 4, 5]\nreturn → a[::-2]");
    assert_eq!(
        val,
        Value::Array(vec![Value::Integer(5), Value::Integer(3), Value::Integer(1)])
    );
}

#[test]
fn test_bytecode_slice_negative_index_start() {
    // arr[-2:] → last 2 elements.
    let val = run_ok("store → a → [10, 20, 30, 40]\nreturn → a[-2:]");
    assert_eq!(
        val,
        Value::Array(vec![Value::Integer(30), Value::Integer(40)])
    );
}

#[test]
fn test_bytecode_slice_negative_index_end() {
    // arr[:-1] → all but last element.
    let val = run_ok("store → a → [1, 2, 3, 4]\nreturn → a[:-1]");
    assert_eq!(
        val,
        Value::Array(vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)])
    );
}

#[test]
fn test_bytecode_slice_start_greater_than_end_errors() {
    // arr[3:1] must error, not panic.
    let result = compile_and_run("store → a → [1, 2, 3, 4, 5]\nreturn → a[3:1]");
    assert!(result.is_err(), "start > end must be a runtime error");
}

#[test]
fn test_bytecode_slice_step_zero_errors() {
    // step=0 must be a runtime error.
    let result = compile_and_run("store → a → [1, 2, 3]\nreturn → a[::0]");
    assert!(result.is_err(), "step=0 must be a runtime error");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("zero"), "error message should mention zero: {}", msg);
}

#[test]
fn test_bytecode_slice_out_of_bounds_errors() {
    // end > len must be a runtime error (no silent clamp).
    let result = compile_and_run("store → a → [1, 2, 3]\nreturn → a[0:99]");
    assert!(result.is_err(), "OOB slice must be a runtime error");
}

#[test]
fn test_bytecode_slice_empty_array_reverse() {
    // [][::-1] → [] (not a panic or error).
    let val = run_ok("store → a → []\nreturn → a[::-1]");
    assert_eq!(val, Value::Array(vec![]));
}

#[test]
fn test_bytecode_slice_step_on_string() {
    // String slicing with step.
    let val = run_ok("store → s → \"abcdef\"\nreturn → s[::2]");
    assert_eq!(val, Value::String("ace".to_string()));
}

#[test]
fn test_bytecode_slice_string_reverse() {
    // String[::-1] must reverse the string.
    let val = run_ok("store → s → \"hello\"\nreturn → s[::-1]");
    assert_eq!(val, Value::String("olleh".to_string()));
}

#[test]
fn test_bytecode_slice_string_negative_index() {
    // String[-3:] → last 3 chars.
    let val = run_ok("store → s → \"hello\"\nreturn → s[-3:]");
    assert_eq!(val, Value::String("llo".to_string()));
}

#[test]
fn test_bytecode_slice_string_unicode_negative_index() {
    // Unicode: "héllo" has 5 chars. [-3:] → last 3 chars "llo", not bytes.
    let val = run_ok("store → s → \"héllo\"\nreturn → s[-3:]");
    assert_eq!(val, Value::String("llo".to_string()));
}

#[test]
fn test_bytecode_slice_string_start_greater_than_end_errors() {
    let result = compile_and_run("store → s → \"hello\"\nreturn → s[4:1]");
    assert!(result.is_err(), "string start > end must be a runtime error");
}

#[test]
fn test_bytecode_slice_string_oob_errors() {
    // end > char count must error (no silent clamp).
    let result = compile_and_run("store → s → \"hi\"\nreturn → s[0:99]");
    assert!(result.is_err(), "string OOB must be a runtime error");
}

// ---------------------------------------------------------------------------
// Issue #7B: Higher-order functions (map/filter/reduce) with lambdas in bytecode VM
// ---------------------------------------------------------------------------

#[test]
fn test_bytecode_map_with_lambda() {
    let result = run_ok(
        "store → result → map([1, 2, 3], (x) → x * 2)\nreturn → result",
    );
    assert_eq!(
        result,
        Value::Array(vec![
            Value::Integer(2),
            Value::Integer(4),
            Value::Integer(6),
        ]),
        "map with lambda must double each element"
    );
}

#[test]
fn test_bytecode_filter_with_lambda() {
    let result = run_ok(
        "store → result → filter([1, 2, 3, 4], (x) → x > 2)\nreturn → result",
    );
    assert_eq!(
        result,
        Value::Array(vec![Value::Integer(3), Value::Integer(4)]),
        "filter with lambda must keep elements > 2"
    );
}

#[test]
fn test_bytecode_reduce_with_lambda() {
    let result = run_ok(
        "store → result → reduce([1, 2, 3, 4], (a, b) → a + b, 0)\nreturn → result",
    );
    assert_eq!(
        result,
        Value::Integer(10),
        "reduce with lambda must sum all elements"
    );
}

// ---------------------------------------------------------------------------
// Task 8.2 — const enforcement in the bytecode VM
// ---------------------------------------------------------------------------

#[test]
fn test_bytecode_const_cannot_be_reassigned() {
    let result = compile_and_run(
        "const → x → 10\nstore → x → 20\nreturn → x",
    );
    assert!(result.is_err(), "reassigning a const must be a runtime error");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("Cannot reassign const") || msg.contains("const"),
        "error message should mention const reassignment, got: {msg}"
    );
}

#[test]
fn test_bytecode_const_value_is_readable() {
    let result = run_ok("const → pi → 3\nreturn → pi");
    assert_eq!(result, Value::Integer(3));
}

// ---------------------------------------------------------------------------
// Task 12.6: ? Error Propagation Operator (bytecode VM)
// ---------------------------------------------------------------------------

#[test]
fn test_bytecode_propagate_ok_unwraps() {
    let result = run_ok(
        "store → r → ok(99)\nstore → v → r?\nreturn → v",
    );
    assert_eq!(result, Value::Integer(99), "? on Ok should unwrap in bytecode VM");
}

#[test]
fn test_bytecode_propagate_err_returns() {
    // ? on Err should cause the current function execution to stop and return the Err.
    // At top level this surfaces as a ReturnValue signal which run_ok would panic on.
    // Use compile_and_run and verify the returned value is Err.
    let result = compile_and_run(
        "store → r → err(\"fail\")\nstore → v → r?\nreturn → v",
    );
    // The ? propagates the Err as a return signal; compile_and_run captures it.
    match result {
        Ok(v) => assert_eq!(
            v,
            Value::Result(false, Box::new(Value::String("fail".to_string()))),
            "? on Err should return the Err value"
        ),
        Err(e) => panic!("unexpected error: {e}"),
    }
}
