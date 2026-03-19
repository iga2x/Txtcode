use txtcode::lexer::Lexer;
use txtcode::parser::Parser;
use txtcode::typecheck::TypeChecker;
use txtcode::typecheck::types::Type;

fn check_source(source: &str) -> Result<(), Vec<String>> {
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut checker = TypeChecker::new();
    checker.check(&program)
}

// Task 3.1 — Strict-types blocking mode
#[test]
fn test_typecheck_annotation_mismatch() {
    // store → x: int → "hello" should be a type error
    let result = check_source("store → x: int → \"hello\"");
    assert!(result.is_err(), "expected type error for int annotation with string value");
    let errors = result.unwrap_err();
    assert!(!errors.is_empty());
    assert!(errors[0].contains("mismatch") || errors[0].contains("type"));
}

#[test]
fn test_typecheck_annotation_match_no_error() {
    // store → x: int → 42 should have no type errors
    let result = check_source("store → x: int → 42");
    assert!(result.is_ok(), "valid int annotation should pass: {:?}", result);
}

#[test]
fn test_typecheck_annotation_string_match() {
    let result = check_source("store → s: string → \"hello\"");
    assert!(result.is_ok(), "valid string annotation should pass: {:?}", result);
}

// Task 3.3 — Null safety mode
#[test]
fn test_null_assigned_to_non_nullable_is_error() {
    // store → x: int → null should be a type error
    let result = check_source("store → x: int → null");
    assert!(result.is_err(), "null assigned to int should be type error");
}

#[test]
fn test_null_assigned_to_nullable_is_ok() {
    // store → x: int? → null should be fine
    let result = check_source("store → x: int? → null");
    assert!(result.is_ok(), "null assigned to int? should pass: {:?}", result);
}

#[test]
fn test_int_assigned_to_nullable_is_ok() {
    // store → x: int? → 42 should be fine
    let result = check_source("store → x: int? → 42");
    assert!(result.is_ok(), "int assigned to int? should pass: {:?}", result);
}
