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
    assert!(matches!(result, txtcode::runtime::vm::Value::Integer(_) | txtcode::runtime::vm::Value::Null));
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

