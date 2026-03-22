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

// V.1 — Unicode escape sequences
#[test]
fn test_v1_unicode_escape_lowercase_u() {
    // \u0041 = 'A' (U+0041)
    let source = r#""\u0041""#.to_string();
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let str_tok = tokens.iter().find(|t| t.kind == TokenKind::String).unwrap();
    assert_eq!(str_tok.value, "A", "\\u0041 should decode to 'A'");
}

#[test]
fn test_v1_unicode_escape_uppercase_u() {
    // \U0001F600 = emoji (U+1F600)
    let source = r#""\U0001F600""#.to_string();
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let str_tok = tokens.iter().find(|t| t.kind == TokenKind::String).unwrap();
    assert_eq!(str_tok.value, "\u{1F600}", "\\U0001F600 should decode to emoji");
}

#[test]
fn test_v1_unicode_escape_null_char() {
    // \u0000 = null character
    let source = r#""\u0000""#.to_string();
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let str_tok = tokens.iter().find(|t| t.kind == TokenKind::String).unwrap();
    assert_eq!(str_tok.value, "\u{0000}");
}

#[test]
fn test_v1_unicode_escape_mixed_with_regular() {
    // "A\u0042C" = "ABC"
    let source = r#""A\u0042C""#.to_string();
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let str_tok = tokens.iter().find(|t| t.kind == TokenKind::String).unwrap();
    assert_eq!(str_tok.value, "ABC");
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
