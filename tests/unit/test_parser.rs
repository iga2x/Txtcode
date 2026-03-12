use txtcode::lexer::Lexer;
use txtcode::parser::Parser;

#[test]
fn test_parser_hello_world() {
    let source = "print → \"Hello, World!\"".to_string();
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    assert_eq!(program.statements.len(), 1);

    // Verify it's a function call
    if let txtcode::parser::ast::Statement::Expression(expr) = &program.statements[0] {
        if let txtcode::parser::ast::Expression::FunctionCall {
            name, arguments, ..
        } = expr
        {
            assert_eq!(name, "print");
            assert_eq!(arguments.len(), 1);
            if let txtcode::parser::ast::Expression::Literal(
                txtcode::parser::ast::Literal::String(s),
            ) = &arguments[0]
            {
                assert_eq!(s, "Hello, World!");
            } else {
                panic!("Expected string literal as argument");
            }
        } else {
            panic!("Expected FunctionCall expression");
        }
    } else {
        panic!("Expected Expression statement");
    }
}

#[test]
fn test_parser_assignment() {
    let source = "store → x → 42".to_string();
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    assert_eq!(program.statements.len(), 1);
}

#[test]
fn test_parser_function() {
    let source = r#"
define → add → (a, b)
  return → a + b
end
"#
    .to_string();
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    assert_eq!(program.statements.len(), 1);
}
