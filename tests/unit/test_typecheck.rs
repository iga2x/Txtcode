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

// Task 10.2 — Collection element type enforcement
#[test]
fn test_array_element_type_mismatch() {
    // store → nums: Array<int> → [1, 2, "three"] → warning for "three"
    let result = check_source("store → nums: array[int] → [1, 2, \"three\"]");
    assert!(result.is_err(), "string element in array[int] should be a type error: {:?}", result);
    let errors = result.unwrap_err();
    assert!(errors.iter().any(|e| e.contains("Array element type mismatch")));
}

#[test]
fn test_array_element_type_match() {
    let result = check_source("store → nums: array[int] → [1, 2, 3]");
    assert!(result.is_ok(), "all int elements in array[int] should pass: {:?}", result);
}

// Task 10.3 — Return type checking
#[test]
fn test_return_type_mismatch() {
    let src = "define → greet → () → int\n  return → \"hello\"\nend";
    let result = check_source(src);
    assert!(result.is_err(), "returning string from int function should be a type error");
    let errors = result.unwrap_err();
    assert!(errors.iter().any(|e| e.contains("Return type mismatch")));
}

#[test]
fn test_return_type_match() {
    let src = "define → add → (a: int, b: int) → int\n  return → 42\nend";
    let result = check_source(src);
    assert!(result.is_ok(), "returning int from int function should pass: {:?}", result);
}

// Task 10.3 — Arity checking
#[test]
fn test_arity_mismatch_warning() {
    // Define f(a, b) then call f(a, b, c)
    let src = "define → f → (a: int, b: int) → int\n  return → 0\nend\nf(1, 2, 3)";
    let result = check_source(src);
    assert!(result.is_err(), "calling f with wrong arity should be a type error");
    let errors = result.unwrap_err();
    assert!(errors.iter().any(|e| e.contains("Arity mismatch")));
}

#[test]
fn test_arity_correct_no_error() {
    let src = "define → f → (a: int, b: int) → int\n  return → 0\nend\nf(1, 2)";
    let result = check_source(src);
    assert!(result.is_ok(), "calling f with correct arity should pass: {:?}", result);
}

// Task 10.3 — Null arithmetic warning
#[test]
fn test_null_arithmetic_warning() {
    let result = check_source("store → x → null + 5");
    assert!(result.is_err(), "null arithmetic should produce a warning/error");
    let errors = result.unwrap_err();
    assert!(errors.iter().any(|e| e.contains("null")));
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
