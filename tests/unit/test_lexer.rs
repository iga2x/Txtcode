use txtcode::lexer::Lexer;

#[test]
fn test_lexer_basic() {
    let source = "print → \"Hello\"".to_string();
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    assert!(!tokens.is_empty());
}

#[test]
fn test_lexer_numbers() {
    let source = "42 3.14 0xFF 0b1010".to_string();
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    assert!(tokens.len() >= 4);
}

#[test]
fn test_lexer_strings() {
    let source = "\"Hello\" 'World' \"Multi\\nline\"".to_string();
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    assert!(tokens.len() >= 3);
}
