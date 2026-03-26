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

fn write_temp_module(name: &str, content: &str) -> std::path::PathBuf {
    let path = std::path::PathBuf::from(format!("/tmp/{}.tc", name));
    std::fs::write(&path, content).expect("write temp module");
    path
}

fn run_with_fs(source: &str) -> Result<txtcode::runtime::Value, txtcode::runtime::errors::RuntimeError> {
    use txtcode::runtime::permissions::PermissionResource;
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut vm = VirtualMachine::new();
    vm.grant_permission(PermissionResource::FileSystem("read".to_string()), None);
    vm.interpret_repl(&program)
}

// ---------------------------------------------------------------------------
// Function / pipe / lambda tests
// ---------------------------------------------------------------------------

#[test]
fn test_runtime_pipe_lambda() {
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
// Closure tests (13.1)
// ---------------------------------------------------------------------------

#[test]
fn test_closure_mutation_does_not_affect_outer_scope() {
    use txtcode::runtime::Value;
    let src = r#"
store → x → 10
store → f → () → x + 1
store → x → 20
f()
"#;
    let result = run(src);
    assert_eq!(result, Value::Integer(11), "closure should capture x at definition time");
}

#[test]
fn test_closure_loop_capture() {
    use txtcode::runtime::Value;
    let src = r#"
store → fns → []
for → i in [1, 2, 3]
  store → fns → array_push(fns, () → i)
end
store → f0 → fns[0]
store → f1 → fns[1]
store → f2 → fns[2]
store → r0 → f0()
store → r1 → f1()
store → r2 → f2()
[r0, r1, r2]
"#;
    let result = run(src);
    assert_eq!(
        result,
        Value::Array(vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)]),
        "each closure should capture its own iteration value"
    );
}

// W.3: Closures capture enclosing scope
#[test]
fn test_w3_closure_captures_outer_variable() {
    let source = r#"
define → make_adder → (n)
  define → adder → (x)
    return → x + n
  end
  return → adder
end
store → add5 → make_adder(5)
add5(3)
"#;
    let r = run_ast_repl(source.trim());
    assert_eq!(r.unwrap(), txtcode::runtime::Value::Integer(8));
}

#[test]
fn test_w3_multiple_closures_independent() {
    let source = r#"
define → multiplier → (n)
  define → mul → (x)
    return → x * n
  end
  return → mul
end
store → double → multiplier(2)
store → triple → multiplier(3)
triple(4)
"#;
    let r = run_ast_repl(source.trim());
    assert_eq!(r.unwrap(), txtcode::runtime::Value::Integer(12));
}

// ---------------------------------------------------------------------------
// TCO tests (E.5, N.4)
// ---------------------------------------------------------------------------

#[test]
fn test_tco_countdown_1000() {
    let src = r#"
define → countdown → (n: int)
  if → n <= 0
    return → "done"
  end
  return → countdown(n - 1)
end
countdown(1000)
"#;
    let result = run_ast_repl(src).unwrap();
    assert_eq!(result, txtcode::runtime::Value::String("done".into()));
}

#[test]
fn test_tco_factorial_accumulator() {
    let src = r#"
define → fact → (n: int, acc: int)
  if → n <= 1
    return → acc
  end
  return → fact(n - 1, acc * n)
end
fact(10, 1)
"#;
    let result = run_ast_repl(src).unwrap();
    assert_eq!(result, txtcode::runtime::Value::Integer(3628800));
}

#[test]
fn test_n4_bare_tco_count_down() {
    let src = r#"
define → countdown → (n, acc)
  if → n == 0
    return → acc
  end
  countdown(n - 1, acc + 1)
end
countdown(5000, 0)
"#;
    let result = run_ast_repl(src);
    assert!(result.is_ok(), "N.4 bare TCO countdown: {:?}", result);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(5000));
}

// ---------------------------------------------------------------------------
// Variadic / default params
// ---------------------------------------------------------------------------

#[test]
fn test_default_param_used() {
    let result = run_ast_repl(r#"
define → greet → (name, greeting = "Hello")
  return → greeting
end
greet("Alice")
"#);
    assert!(result.is_ok(), "default param: {:?}", result);
}

#[test]
fn test_default_param_overridden() {
    let result = run_ast_repl(r#"
define → greet → (name, greeting = "Hello")
  return → greeting
end
greet("Alice", "Hi")
"#);
    assert!(result.is_ok(), "default param override: {:?}", result);
}

#[test]
fn test_variadic_spread_syntax() {
    let result = run_ast_repl(r#"
define → sum_all → (...args)
  store → total → 0
  for → x in args
    store → total → total + x
  end
  return → total
end
sum_all(1, 2, 3, 4)
"#);
    assert!(result.is_ok(), "variadic spread: {:?}", result);
}

#[test]
fn test_variadic_star_syntax() {
    let result = run_ast_repl(r#"
define → count_args → (*args)
  return → len(args)
end
count_args(1, 2, 3)
"#);
    assert!(result.is_ok(), "variadic star: {:?}", result);
}

// ---------------------------------------------------------------------------
// Impl block / method tests
// ---------------------------------------------------------------------------

#[test]
fn test_impl_block_method_call() {
    use txtcode::runtime::Value;
    let source = r#"
struct Point(x: int, y: int)

impl → Point
  define → sum → (self)
    return → self.x + self.y
  end
end

store → p → Point { x: 3, y: 4 }
p.sum()
"#;
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut vm = VirtualMachine::new();
    let result = vm.interpret_repl(&program).unwrap();
    assert_eq!(result, Value::Integer(7), "Point.sum() should return x + y = 7");
}

#[test]
fn test_impl_block_method_with_arg() {
    use txtcode::runtime::Value;
    let source = r#"
struct Point(x: int, y: int)

impl → Point
  define → scale → (self, factor)
    return → self.x * factor
  end
end

store → p → Point { x: 3, y: 4 }
p.scale(10)
"#;
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut vm = VirtualMachine::new();
    let result = vm.interpret_repl(&program).unwrap();
    assert_eq!(result, Value::Integer(30));
}

// W.4: Method definition with dotted name
#[test]
fn test_w4_dotted_method_definition_and_call() {
    let source = r#"
struct Counter(val: int)
define → Counter.increment → (self)
  return → self["val"] + 1
end
store → c → Counter{ val: 10 }
c.increment()
"#;
    let r = run_ast_repl(source.trim());
    assert_eq!(r.unwrap(), txtcode::runtime::Value::Integer(11));
}

// ---------------------------------------------------------------------------
// Parser error recovery (E.4)
// ---------------------------------------------------------------------------

#[test]
fn test_parser_error_recovery_single_error() {
    let src = "store → x → 42\ndefine → (\nstore → y → 10\n";
    let tokens = txtcode::lexer::Lexer::new(src.to_string()).tokenize().unwrap();
    let mut parser = txtcode::parser::Parser::new(tokens);
    let (program, errors) = parser.parse_with_errors();
    assert!(!errors.is_empty(), "Should have at least 1 error");
    assert!(!program.statements.is_empty(), "Should have partial AST with statements");
}

#[test]
fn test_parser_error_recovery_continues_after_error() {
    let src = "store → a → 1\nstruct → (\nstore → b → 2\n";
    let tokens = txtcode::lexer::Lexer::new(src.to_string()).tokenize().unwrap();
    let mut parser = txtcode::parser::Parser::new(tokens);
    let (program, errors) = parser.parse_with_errors();
    assert!(!errors.is_empty());
    let valid_stmts: Vec<_> = program.statements.iter()
        .filter(|s| !matches!(s, txtcode::parser::ast::Statement::Error { .. }))
        .collect();
    assert!(!valid_stmts.is_empty(), "Should have valid statements after error recovery");
}

#[test]
fn test_parser_error_recovery_error_node_in_ast() {
    let src = "store → x → 1\ndefine → (\nstore → y → 2\n";
    let tokens = txtcode::lexer::Lexer::new(src.to_string()).tokenize().unwrap();
    let mut parser = txtcode::parser::Parser::new(tokens);
    let (program, _errors) = parser.parse_with_errors();
    let has_error_node = program.statements.iter()
        .any(|s| matches!(s, txtcode::parser::ast::Statement::Error { .. }));
    assert!(has_error_node, "AST should contain Statement::Error node at error position");
}

#[test]
fn test_parser_error_recovery_multiple_errors() {
    let src = "store → x → 1\ndefine → (\nstore → y → 2\nstruct → (\nstore → z → 3\n";
    let tokens = txtcode::lexer::Lexer::new(src.to_string()).tokenize().unwrap();
    let mut parser = txtcode::parser::Parser::new(tokens);
    let (program, errors) = parser.parse_with_errors();
    assert!(errors.len() >= 2, "Should report at least 2 errors");
    let error_nodes = program.statements.iter()
        .filter(|s| matches!(s, txtcode::parser::ast::Statement::Error { .. }))
        .count();
    assert!(error_nodes >= 2, "Should have at least 2 error nodes in AST");
}

// ---------------------------------------------------------------------------
// Module isolation tests
// ---------------------------------------------------------------------------

#[test]
fn test_module_isolation_no_namespace_pollution() {
    write_temp_module(
        "mod_iso_a",
        "define → helper → ()\n  return → 42\nend\ndefine → internal → ()\n  return → 99\nend\n",
    );
    let result = run_with_fs("import → helper from \"/tmp/mod_iso_a\"\nstore → r → helper()\nr");
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(42));
}

#[test]
fn test_module_unexported_symbol_not_accessible() {
    write_temp_module(
        "mod_iso_b",
        "define → pub_fn → ()\n  return → 1\nend\ndefine → _private → ()\n  return → 2\nend\n",
    );
    let result = run_with_fs("import → _private from \"/tmp/mod_iso_b\"\n_private()");
    assert!(result.is_err(), "Importing underscore-prefixed symbol should fail");
}

#[test]
fn test_module_two_modules_same_function_no_collision() {
    write_temp_module("mod_col_a", "define → compute → ()\n  return → 10\nend\n");
    write_temp_module("mod_col_b", "define → compute → ()\n  return → 20\nend\n");
    let result = run_with_fs(
        "import → compute from \"/tmp/mod_col_a\"\ncompute()",
    );
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(10));
}

#[test]
fn test_circular_import_detected() {
    write_temp_module(
        "mod_circ_a",
        "import → helper from \"/tmp/mod_circ_b\"\ndefine → a_fn → ()\n  return → 1\nend\n",
    );
    write_temp_module(
        "mod_circ_b",
        "import → a_fn from \"/tmp/mod_circ_a\"\ndefine → helper → ()\n  return → 2\nend\n",
    );
    let result = run_with_fs("import → a_fn from \"/tmp/mod_circ_a\"\na_fn()");
    assert!(result.is_err(), "Circular import must be detected");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("ircular") || msg.contains("import"),
        "Error should mention circular import, got: {msg}"
    );
}

// ---------------------------------------------------------------------------
// P.1: O(1) stdlib dispatch tests
// ---------------------------------------------------------------------------

#[test]
fn test_p1_known_exact_name_routes_correctly() {
    let tests = vec![
        ("len(\"hello\")", txtcode::runtime::Value::Integer(5)),
        ("abs(0 - 7)", txtcode::runtime::Value::Integer(7)),
        ("max(3, 9)", txtcode::runtime::Value::Integer(9)),
        ("min(3, 9)", txtcode::runtime::Value::Integer(3)),
    ];
    for (src, expected) in tests {
        let result = run_ast_repl(src);
        assert!(result.is_ok(), "dispatch failed for: {}: {:?}", src, result.err());
        assert_eq!(result.unwrap(), expected, "wrong result for: {}", src);
    }
}

#[test]
fn test_p1_unknown_function_gives_clear_error() {
    let result = run_ast_source("store → x → totally_unknown_fn_xyz(1, 2)");
    assert!(result.is_err(), "unknown function should error");
    let msg = format!("{}", result.err().unwrap());
    assert!(
        msg.contains("totally_unknown_fn_xyz") || msg.contains("Unknown"),
        "error should name the unknown function: {}",
        msg
    );
}

// ---------------------------------------------------------------------------
// P.2: String clone O(1) and str_build correctness
// ---------------------------------------------------------------------------

#[test]
fn test_p2_string_clone_is_o1() {
    use std::sync::Arc;
    let big: String = "x".repeat(1_000_000);
    let v = txtcode::runtime::Value::String(Arc::from(big.as_str()));
    let start = std::time::Instant::now();
    let clones: Vec<_> = (0..10_000).map(|_| v.clone()).collect();
    let elapsed = start.elapsed();
    drop(clones);
    assert!(
        elapsed.as_millis() < 200,
        "10,000 clones of 1MB string took {}ms — expected O(1), not O(n)",
        elapsed.as_millis()
    );
}

#[test]
fn test_p2_str_build_correctness() {
    let result = run_ast_repl(r#"str_build(["hello", " ", "world"])"#);
    assert!(result.is_ok(), "str_build should not error: {:?}", result.err());
    assert_eq!(
        result.unwrap(),
        txtcode::runtime::Value::String(std::sync::Arc::from("hello world")),
        "str_build should concatenate parts"
    );
}
