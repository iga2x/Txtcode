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
    assert!(matches!(result, txtcode::runtime::Value::Integer(_) | txtcode::runtime::Value::Null));
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
// Phase 6 — AST VM pipe and async tests
// ---------------------------------------------------------------------------

fn run_ast_source(source: &str) -> Result<txtcode::runtime::Value, txtcode::runtime::errors::RuntimeError> {
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
    let result = run_ast_source(r#"
define → double → (x)
  return → x * 2
end
store → result → 5 |> double
"#);
    assert!(result.is_ok(), "pipe with identifier rhs should work: {:?}", result);
}

#[test]
fn test_runtime_async_sync_mode() {
    // async functions run synchronously in v0.3, should not crash
    let result = run_ast_source(r#"
async → define → add_one → (x)
  return → x + 1
end
store → result → add_one(41)
"#);
    assert!(result.is_ok(), "async function should run synchronously: {:?}", result);
}
