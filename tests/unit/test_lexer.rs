use txtcode::lexer::{Lexer, TokenKind};

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

// Task 2.1 — Hex / binary / octal literals
#[test]
fn test_hex_literal() {
    let source = "0xFF".to_string();
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let int_tok = tokens.iter().find(|t| t.kind == TokenKind::Integer).unwrap();
    assert_eq!(int_tok.value, "255");
}

#[test]
fn test_binary_literal() {
    let source = "0b1010".to_string();
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let int_tok = tokens.iter().find(|t| t.kind == TokenKind::Integer).unwrap();
    assert_eq!(int_tok.value, "10");
}

#[test]
fn test_octal_literal() {
    let source = "0o777".to_string();
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let int_tok = tokens.iter().find(|t| t.kind == TokenKind::Integer).unwrap();
    assert_eq!(int_tok.value, "511");
}
