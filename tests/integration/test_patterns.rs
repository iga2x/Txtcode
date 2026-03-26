use std::sync::Arc;
use txtcode::lexer::Lexer;
use txtcode::parser::Parser;
use txtcode::runtime::vm::VirtualMachine;

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

// ---------------------------------------------------------------------------
// Match guard tests (Group 14.4)
// ---------------------------------------------------------------------------

#[test]
fn test_match_guard_clause() {
    let result = run_ast_repl(r#"
store → x → 15
store → result → "other"
match x
  case n if n > 10
    store → result → "big"
end
result
"#);
    assert!(result.is_ok(), "match guard: {:?}", result);
}

#[test]
fn test_match_guard_positive() {
    use txtcode::runtime::Value;
    let source = r#"
store → x → 15
store → res → "none"
match x
  case n if n > 10
    store → res → "big"
  case n
    store → res → "small"
end
res
"#;
    assert_eq!(run(source), Value::String(Arc::from("big")));
}

#[test]
fn test_match_guard_fallthrough() {
    use txtcode::runtime::Value;
    let source = r#"
store → x → 5
store → res → "none"
match x
  case n if n > 10
    store → res → "big"
  case n
    store → res → "small"
end
res
"#;
    assert_eq!(run(source), Value::String(Arc::from("small")));
}

#[test]
fn test_match_array_rest_pattern() {
    use txtcode::runtime::Value;
    let source = r#"
store → arr → [10, 20, 30, 40]
store → head → 0
store → tail_len → 0
match arr
  case [h, ...t]
    store → head → h
    store → tail_len → len(t)
  case _
    store → head → -1
end
head + tail_len
"#;
    assert_eq!(run(source), Value::Integer(13));
}

#[test]
fn test_match_array_rest_empty_tail() {
    use txtcode::runtime::Value;
    let source = r#"
store → arr → [42]
store → result → 0
match arr
  case [h, ...t]
    store → result → h * 10 + len(t)
  case _
    store → result → -1
end
result
"#;
    assert_eq!(run(source), Value::Integer(420));
}

#[test]
fn test_match_array_exact_no_match() {
    use txtcode::runtime::Value;
    let source = r#"
store → arr → []
store → result → 0
match arr
  case [h, ...t]
    store → result → 1
  case _
    store → result → 99
end
result
"#;
    assert_eq!(run(source), Value::Integer(99));
}

#[test]
fn test_match_nested_literal_in_struct_pattern() {
    use txtcode::runtime::Value;
    let source = r#"
struct → Point → (x: int, y: int)
store → p → Point(0, 5)
store → result → -1
match p
  case {x: 0, y}
    store → result → y
  case _
    store → result → 0
end
result
"#;
    assert_eq!(run(source), Value::Integer(5));
}

// ---------------------------------------------------------------------------
// Destructuring tests (Group 14.2)
// ---------------------------------------------------------------------------

#[test]
fn test_destructure_array_assign() {
    use txtcode::runtime::Value;
    assert_eq!(run("store → [a, b, c] → [10, 20, 30]\na + b + c"), Value::Integer(60));
}

#[test]
fn test_destructure_array_rest_assign() {
    use txtcode::runtime::Value;
    assert_eq!(
        run("store → [h, ...t] → [1, 2, 3, 4]\nh * 10 + len(t)"),
        Value::Integer(13)
    );
}

#[test]
fn test_destructure_struct_assign() {
    use txtcode::runtime::Value;
    let source = r#"
struct → Point → (x: int, y: int)
store → p → Point(3, 7)
store → {x, y} → p
x + y
"#;
    assert_eq!(run(source), Value::Integer(10));
}

#[test]
fn test_destructure_array_in_function_param() {
    use txtcode::runtime::Value;
    let source = r#"
define → sum_pair → ([a, b])
  return → a + b
end
sum_pair([3, 4])
"#;
    assert_eq!(run(source), Value::Integer(7));
}

#[test]
fn test_destructure_array_with_rest_in_function_param() {
    use txtcode::runtime::Value;
    let source = r#"
define → first_and_count → ([head, ...rest])
  return → head * 100 + len(rest)
end
first_and_count([5, 1, 2, 3])
"#;
    assert_eq!(run(source), Value::Integer(503));
}

// ---------------------------------------------------------------------------
// Enum tests
// ---------------------------------------------------------------------------

#[test]
fn test_enum_variant_no_payload() {
    let result = run_ast_repl(
        "enum → Direction → North, South, East, West\nDirection.North"
    );
    assert_eq!(
        result.unwrap(),
        txtcode::runtime::Value::Enum("Direction".to_string(), "North".to_string(), None)
    );
}

#[test]
fn test_enum_variant_with_payload_constructor() {
    let result = run_ast_repl(
        "enum → Shape → Circle, Square\nShape.Circle(5)"
    );
    assert_eq!(
        result.unwrap(),
        txtcode::runtime::Value::Enum(
            "Shape".to_string(),
            "Circle".to_string(),
            Some(Box::new(txtcode::runtime::Value::Integer(5)))
        )
    );
}

#[test]
fn test_enum_pattern_match_with_payload() {
    let result = run_ast_repl(r#"
enum → Shape → Circle, Square
store → s → Shape.Circle(10)
store → res → 0
match s
  case Circle(r)
    store → res → r * 2
  case Square(side)
    store → res → side
end
res
"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(20));
}

#[test]
fn test_enum_dot_pattern_no_payload() {
    let result = run_ast_repl(r#"
enum → Direction → North, South
store → d → Direction.North
store → res → 0
match d
  case Direction.North
    store → res → 1
  case Direction.South
    store → res → 2
end
res
"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(1));
}

// ---------------------------------------------------------------------------
// N.1: Pattern::Literal tests
// ---------------------------------------------------------------------------

#[test]
fn test_n1_literal_pattern_string_escaped_quote() {
    let src = "store → s → \"say \\\"hi\\\"\"\nstore → result → 0\nmatch s\n  case \"say \\\"hi\\\"\"\n    store → result → 1\nend\nresult";
    let result = run_ast_repl(src);
    assert!(result.is_ok(), "N.1 string escaped-quote pattern: {:?}", result);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(1));
}

#[test]
fn test_n1_literal_pattern_boolean() {
    let src = "store → flag → true\nstore → result → 0\nmatch flag\n  case true\n    store → result → 42\nend\nresult";
    let result = run_ast_repl(src);
    assert!(result.is_ok(), "N.1 boolean pattern: {:?}", result);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(42));
}

#[test]
fn test_n1_literal_pattern_null() {
    let src = "store → v → null\nstore → result → 0\nmatch v\n  case null\n    store → result → 99\nend\nresult";
    let result = run_ast_repl(src);
    assert!(result.is_ok(), "N.1 null pattern: {:?}", result);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(99));
}

// N.5: Modulo by zero → error code E0012
#[test]
fn test_n5_modulo_by_zero_error_code() {
    let src = "5 % 0";
    let result = run_ast_repl(src);
    assert!(result.is_err(), "N.5 modulo by zero should be error");
    let err = result.unwrap_err();
    assert!(
        err.code == Some(txtcode::runtime::errors::ErrorCode::E0012),
        "N.5 expected E0012, got {:?}", err.code
    );
}
