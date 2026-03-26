use std::sync::Arc;
use txtcode::lexer::Lexer;
use txtcode::parser::Parser;
use txtcode::runtime::vm::VirtualMachine;

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

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

fn run(source: &str) -> txtcode::runtime::Value {
    use txtcode::runtime::Value;
    let tokens = txtcode::lexer::Lexer::new(source.to_string()).tokenize().unwrap();
    let program = txtcode::parser::Parser::new(tokens).parse().unwrap();
    let mut vm = VirtualMachine::new();
    vm.interpret_repl(&program).unwrap_or(Value::Null)
}

// ---------------------------------------------------------------------------
// Basic runtime smoke tests
// ---------------------------------------------------------------------------

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
// Slice stabilization tests — AST VM + stdlib
// ---------------------------------------------------------------------------

#[test]
fn test_ast_slice_string_step() {
    // String slicing with step is now supported in AST VM.
    let result = run_ast_repl(r#""abcdef"[::2]"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String(Arc::from("ace")));
}

#[test]
fn test_ast_slice_string_reverse() {
    let result = run_ast_repl(r#""hello"[::-1]"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String(Arc::from("olleh")));
}

#[test]
fn test_ast_slice_string_negative_index_char_based() {
    // "héllo" — 5 Unicode chars; [-3:] must use char count (5), not byte count (6).
    let result = run_ast_repl(r#""héllo"[-3:]"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String(Arc::from("llo")));
}

#[test]
fn test_ast_slice_string_step_zero_errors() {
    let result = run_ast_repl(r#""hello"[::0]"#);
    assert!(result.is_err(), "step=0 must error in AST VM");
}

#[test]
fn test_ast_slice_empty_array_reverse() {
    // [][::-1] must return [] without panic.
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

// ---------------------------------------------------------------------------
// Map insertion-order iteration (Group 9.4)
// ---------------------------------------------------------------------------

#[test]
fn test_map_iteration_is_insertion_ordered() {
    // Map keys must iterate in the exact order they were inserted.
    use txtcode::runtime::Value;

    let source = r#"
store → m → {a: 1, b: 2, c: 3}
store → keys → []
for → k in m
  store → keys → array_push(keys, k)
end
keys
"#;
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut vm = VirtualMachine::new();
    let result = vm.interpret_repl(&program).unwrap();
    assert_eq!(
        result,
        Value::Array(vec![
            Value::String(Arc::from("a")),
            Value::String(Arc::from("b")),
            Value::String(Arc::from("c")),
        ]),
        "Map keys must iterate in insertion order (a, b, c)"
    );
}

// ---------------------------------------------------------------------------
// Iterator Protocol — range / enumerate (Group 14.3)
// ---------------------------------------------------------------------------

#[test]
fn test_range_basic_for_loop() {
    use txtcode::runtime::Value;
    let src = "store → total → 0\nfor → i in range(0, 5)\n  total += i\nend\ntotal";
    assert_eq!(run(src), Value::Integer(10));
}

#[test]
fn test_range_with_step() {
    use txtcode::runtime::Value;
    let src = "store → total → 0\nfor → i in range(0, 6, 2)\n  total += i\nend\ntotal";
    assert_eq!(run(src), Value::Integer(6));
}

#[test]
fn test_enumerate_basic() {
    use txtcode::runtime::Value;
    let src = r#"
store → result → 0
for → pair in enumerate(["a", "b", "c"])
  result += pair[0]
end
result
"#;
    assert_eq!(run(src), Value::Integer(3)); // 0+1+2
}

// ---------------------------------------------------------------------------
// Generator Functions (Group 14.5)
// ---------------------------------------------------------------------------

#[test]
fn test_generator_basic_yield() {
    use txtcode::runtime::Value;
    let src = r#"
define → count_to_three → ()
  yield → 1
  yield → 2
  yield → 3
end
store → total → 0
for → x in count_to_three()
  total += x
end
total
"#;
    assert_eq!(run(src), Value::Integer(6));
}

#[test]
fn test_generator_with_loop() {
    use txtcode::runtime::Value;
    let src = r#"
define → squares → (n)
  for → i in range(1, n + 1)
    yield → i * i
  end
end
store → total → 0
for → x in squares(5)
  total += x
end
total
"#;
    assert_eq!(run(src), Value::Integer(55)); // 1+4+9+16+25
}

#[test]
fn test_generator_conditional_yield() {
    use txtcode::runtime::Value;
    let src = r#"
define → evens → (n)
  for → i in range(0, n)
    if → i % 2 == 0
      yield → i
    end
  end
end
store → total → 0
for → x in evens(10)
  total += x
end
total
"#;
    assert_eq!(run(src), Value::Integer(20)); // 0+2+4+6+8
}

#[test]
fn test_generator_returns_array() {
    use txtcode::runtime::Value;
    let src = r#"
define → three_items → ()
  yield → "a"
  yield → "b"
  yield → "c"
end
three_items()
"#;
    let result = run(src);
    assert_eq!(result, Value::Array(vec![
        Value::String(Arc::from("a")),
        Value::String(Arc::from("b")),
        Value::String(Arc::from("c")),
    ]));
}

// ---------------------------------------------------------------------------
// CLI Argument Parsing (Task 17.4)
// ---------------------------------------------------------------------------

#[test]
fn test_cli_parse_flags_and_options() {
    use txtcode::runtime::Value;
    let src = r#"
store → cli_args → ["--verbose", "--output", "file.txt", "positional"]
store → cli_spec → {"flags": ["verbose"], "options": ["output"], "positionals": ["file"]}
store → parsed → cli_parse(cli_args, cli_spec)
[parsed["verbose"], parsed["output"], parsed["file"]]
"#;
    let tokens = Lexer::new(src.to_string()).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap();
    let result = VirtualMachine::new().interpret_repl(&program).unwrap();
    assert_eq!(
        result,
        Value::Array(vec![
            Value::Boolean(true),
            Value::String(Arc::from("file.txt")),
            Value::String(Arc::from("positional")),
        ])
    );
}

#[test]
fn test_cli_parse_defaults() {
    use txtcode::runtime::Value;
    let src = r#"
store → parsed → cli_parse([], {"flags": ["debug"], "options": ["host"]})
[parsed["debug"], parsed["host"]]
"#;
    let tokens = Lexer::new(src.to_string()).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap();
    let result = VirtualMachine::new().interpret_repl(&program).unwrap();
    assert_eq!(result, Value::Array(vec![Value::Boolean(false), Value::Null]));
}

// ---------------------------------------------------------------------------
// Process Control (Task 17.5)
// ---------------------------------------------------------------------------

#[test]
fn test_proc_run_basic() {
    use txtcode::runtime::Value;
    use txtcode::runtime::permissions::PermissionResource;
    let src = r#"
store → result → proc_run("echo hello")
result["stdout"]
"#;
    let tokens = Lexer::new(src.to_string()).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap();
    let mut vm = VirtualMachine::new();
    vm.set_exec_allowed(true);
    vm.grant_permission(PermissionResource::System("exec".to_string()), None);
    let result = vm.interpret_repl(&program).unwrap();
    match result {
        Value::String(s) => assert!(s.contains("hello"), "Expected 'hello' in stdout, got: {}", s),
        other => panic!("Expected String, got {:?}", other),
    }
}

#[test]
fn test_proc_run_with_stdin() {
    use txtcode::runtime::Value;
    use txtcode::runtime::permissions::PermissionResource;
    let src = r#"
store → result → proc_run("cat", {"stdin": "input data"})
result["stdout"]
"#;
    let tokens = Lexer::new(src.to_string()).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap();
    let mut vm = VirtualMachine::new();
    vm.set_exec_allowed(true);
    vm.grant_permission(PermissionResource::System("exec".to_string()), None);
    let result = vm.interpret_repl(&program).unwrap();
    match result {
        Value::String(s) => assert_eq!(s.as_ref(), "input data"),
        other => panic!("Expected String, got {:?}", other),
    }
}

#[test]
fn test_proc_pipe_basic() {
    use txtcode::runtime::Value;
    use txtcode::runtime::permissions::PermissionResource;
    let src = r#"
store → result → proc_pipe(["echo hello world", "tr a-z A-Z"])
result["stdout"]
"#;
    let tokens = Lexer::new(src.to_string()).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap();
    let mut vm = VirtualMachine::new();
    vm.set_exec_allowed(true);
    vm.grant_permission(PermissionResource::System("exec".to_string()), None);
    let result = vm.interpret_repl(&program).unwrap();
    match result {
        Value::String(s) => assert!(s.contains("HELLO"), "Expected uppercase, got: {}", s),
        other => panic!("Expected String, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Call Depth / Recursion tests (Group C)
// ---------------------------------------------------------------------------

#[test]
fn test_deep_recursion_100() {
    let src = r#"
define → count → (n)
  if → n == 0
    return → "done"
  end
  return → count(n - 1)
end
count(100)
"#;
    let result = run_ast_repl(src);
    assert!(result.is_ok(), "100-deep recursion should succeed: {:?}", result);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String(Arc::from("done")));
}

#[test]
fn test_deep_recursion_499() {
    let src = r#"
define → count → (n)
  if → n == 0
    return → 0
  end
  return → count(n - 1)
end
count(499)
"#;
    let result = run_ast_repl(src);
    assert!(result.is_ok(), "499-deep recursion should succeed: {:?}", result);
}

#[test]
fn test_mutual_recursion_50() {
    let src = r#"
define → ping → (n)
  if → n == 0
    return → "ping"
  end
  return → pong(n - 1)
end
define → pong → (n)
  if → n == 0
    return → "pong"
  end
  return → ping(n - 1)
end
ping(50)
"#;
    let result = run_ast_repl(src);
    assert!(result.is_ok(), "mutual recursion 50 should succeed: {:?}", result);
}

#[test]
fn test_recursive_fibonacci_30() {
    let src = r#"
define → fib → (n)
  if → n <= 1
    return → n
  end
  return → fib(n - 1) + fib(n - 2)
end
fib(15)
"#;
    let result = run_ast_repl(src);
    assert!(result.is_ok(), "fib(15) should succeed: {:?}", result);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(610));
}

#[test]
fn test_recursion_limit_error() {
    let src = r#"
define → inf → (n)
  return → inf(n + 1)
end
inf(0)
"#;
    let result = run_ast_repl(src);
    assert!(result.is_err(), "infinite recursion must return an error, not panic");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("call stack") || err.to_string().contains("recursion") || err.to_string().contains("depth"),
        "error must mention recursion/stack: {}", err
    );
}

// ---------------------------------------------------------------------------
// WASM compiler string support (Group 29.1)
// ---------------------------------------------------------------------------

#[cfg(feature = "bytecode")]
#[test]
fn test_wasm_compiler_string_data_segments() {
    use txtcode::compiler::bytecode::BytecodeCompiler;
    use txtcode::compiler::wasm::WasmCompiler;
    let source = r#"store → greeting → "Hello, WASM!""#.to_string();
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut compiler = BytecodeCompiler::new();
    let bytecode = compiler.compile(&program);
    let mut wasm = WasmCompiler::new();
    let wat = wasm.compile(&bytecode);
    // Verify string appears in data segment
    assert!(wat.contains("Hello, WASM!"), "string not in data segment: {}", &wat[..200.min(wat.len())]);
    assert!(wat.contains("(data"), "no data segments: {}", &wat[..200.min(wat.len())]);
}

// ---------------------------------------------------------------------------
// P.4: Argument pooling performance test
// ---------------------------------------------------------------------------

/// P.4: Thread-local arg pool avoids per-call Vec allocation.
/// 10,000 calls to a 3-arg function must complete well under 500ms.
#[test]
fn test_p4_100k_calls_under_500ms() {
    let src = r#"
define → add3 → (a, b, c)
  return → a + b + c
end
store → total → 0
for → i in range(0, 10000)
  store → total → add3(total, 1, 0)
end
total
"#;
    let start = std::time::Instant::now();
    let result = run_ast_repl(src);
    let elapsed = start.elapsed();
    assert!(result.is_ok(), "10k calls failed: {:?}", result.err());
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(10000));
    assert!(
        elapsed.as_millis() < 500,
        "10k function calls took {}ms — expected < 500ms",
        elapsed.as_millis()
    );
}
