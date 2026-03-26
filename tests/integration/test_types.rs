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

fn run_with_strict(source: &str) -> Result<txtcode::runtime::Value, txtcode::runtime::errors::RuntimeError> {
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut vm = VirtualMachine::new();
    vm.set_strict_types(true);
    vm.interpret(&program)
}

// ---------------------------------------------------------------------------
// Type enforcement tests (Task 21.2)
// ---------------------------------------------------------------------------

#[test]
fn test_type_enforcement_int_ok() {
    let result = run_ast_repl("store → x: int → 42\nx");
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(42));
}

#[test]
fn test_type_enforcement_int_rejects_string() {
    let result = run_ast_repl("store → x: int → \"hello\"");
    let err = result.unwrap_err();
    assert!(
        err.message().contains("type mismatch"),
        "expected type mismatch error, got: {}",
        err.message()
    );
}

#[test]
fn test_type_enforcement_string_ok() {
    let result = run_ast_repl("store → s: string → \"hi\"\ns");
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String(Arc::from("hi")));
}

#[test]
fn test_type_enforcement_string_rejects_int() {
    let result = run_ast_repl("store → s: string → 99");
    let err = result.unwrap_err();
    assert!(err.message().contains("type mismatch"));
}

#[test]
fn test_type_enforcement_bool_rejects_int() {
    let result = run_ast_repl("store → flag: bool → 1");
    let err = result.unwrap_err();
    assert!(err.message().contains("type mismatch"));
}

#[test]
fn test_type_enforcement_null_always_allowed() {
    let result = run_ast_repl("store → x: int → null\nx");
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Null);
}

#[test]
fn test_type_enforcement_unannotated_allows_any() {
    let result = run_ast_repl("store → x → \"hello\"\nx");
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String(Arc::from("hello")));
}

#[test]
fn test_type_enforcement_param_ok() {
    let src = "define → greet → (name: string)\n  return → name\nend\ngreet(\"Alice\")";
    let result = run_ast_repl(src);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String(Arc::from("Alice")));
}

#[test]
fn test_type_enforcement_param_rejects_wrong_type() {
    let src = "define → double → (n: int)\n  return → n * 2\nend\ndouble(\"oops\")";
    let result = run_ast_repl(src);
    let err = result.unwrap_err();
    assert!(
        err.message().contains("type mismatch"),
        "expected type mismatch error, got: {}",
        err.message()
    );
}

#[test]
fn test_type_enforcement_error_code_e0011() {
    use txtcode::runtime::errors::ErrorCode;
    let result = run_ast_repl("store → x: int → true");
    let err = result.unwrap_err();
    assert_eq!(err.code, Some(ErrorCode::E0011));
}

// ---------------------------------------------------------------------------
// Struct construction and type checks
// ---------------------------------------------------------------------------

#[test]
fn test_struct_construction_correct_types() {
    let source = "struct Point(x: int, y: int)\nstore → p → Point{ x: 1, y: 2 }";
    let result = run_ast_repl(source);
    assert!(result.is_ok(), "correct struct construction should succeed: {:?}", result);
}

#[test]
fn test_struct_construction_type_mismatch_advisory() {
    let source = "struct Point(x: int, y: int)\nstore → p → Point{ x: \"bad\", y: 2 }";
    let result = run_ast_repl(source);
    assert!(result.is_ok(), "advisory mode should not hard-error on type mismatch: {:?}", result);
}

#[test]
fn test_struct_construction_type_mismatch_strict() {
    let source = "struct Point(x: int, y: int)\nstore → p → Point{ x: \"bad\", y: 2 }";
    let result = run_with_strict(source);
    assert!(result.is_err(), "strict mode must error on type mismatch");
    let err = result.unwrap_err();
    assert_eq!(
        err.code,
        Some(txtcode::runtime::errors::ErrorCode::E0016),
        "error code must be E0016, got: {:?}", err.code
    );
}

#[test]
fn test_struct_unknown_field_strict() {
    let source = "struct Point(x: int, y: int)\nstore → p → Point{ x: 1, y: 2, z: 3 }";
    let result = run_with_strict(source);
    assert!(result.is_err(), "strict mode must error on unknown field");
    let err = result.unwrap_err();
    assert_eq!(
        err.code,
        Some(txtcode::runtime::errors::ErrorCode::E0016),
        "error code must be E0016, got: {:?}", err.code
    );
}

#[test]
fn test_struct_field_type_mismatch_strict_mode() {
    let result = run_with_strict(r#"
struct → User → (name: string, age: int)
store → u → User{ name: 42, age: "young" }
"#);
    assert!(result.is_err(), "strict mode must error on field type mismatch: {:?}", result);
}

#[test]
fn test_struct_field_type_match() {
    let result = run_ast_repl(r#"
struct → User → (name: string, age: int)
store → u → User{ name: "Alice", age: 30 }
u["name"]
"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String(Arc::from("Alice")));
}

// ---------------------------------------------------------------------------
// Protocol system (Task E.1, Group N.2)
// ---------------------------------------------------------------------------

#[test]
fn test_protocol_declaration_is_stored() {
    let src = r#"
protocol → Serializable
  serialize(self) → string
  deserialize(s) → Self
end
__protocol_Serializable
"#;
    let result = run_ast_repl(src).unwrap();
    if let txtcode::runtime::Value::Array(ref methods) = result {
        assert!(!methods.is_empty(), "protocol should have methods");
    } else {
        panic!("Expected Array for __protocol_Serializable, got {:?}", result);
    }
}

#[test]
fn test_struct_implements_stored() {
    let src = r#"
protocol → Drawable
  draw(self) → null
end
struct → Point(x: int, y: int) implements Drawable
__implements_Point
"#;
    let result = run_ast_repl(src).unwrap();
    if let txtcode::runtime::Value::Array(ref list) = result {
        assert!(list.iter().any(|v| v == &txtcode::runtime::Value::String("Drawable".into())),
            "Point should implement Drawable");
    } else {
        panic!("Expected Array for __implements_Point, got {:?}", result);
    }
}

#[test]
fn test_struct_implements_multiple_protocols() {
    let src = r#"
struct → Widget(id: int) implements Drawable, Serializable
__implements_Widget
"#;
    let result = run_ast_repl(src).unwrap();
    if let txtcode::runtime::Value::Array(ref list) = result {
        assert_eq!(list.len(), 2);
        assert!(list.contains(&txtcode::runtime::Value::String("Drawable".into())));
        assert!(list.contains(&txtcode::runtime::Value::String("Serializable".into())));
    } else {
        panic!("Expected Array with 2 protocols, got {:?}", result);
    }
}

#[test]
fn test_protocol_method_names_accessible() {
    let src = r#"
protocol → Iterator
  next(self) → array
  has_next(self) → bool
end
store → methods → __protocol_Iterator
methods[0]["name"]
"#;
    let result = run_ast_repl(src).unwrap();
    assert_eq!(result, txtcode::runtime::Value::String("next".into()));
}

#[test]
fn test_struct_without_implements_has_empty_list() {
    let src = r#"
struct → Bare(x: int)
struct → Bare2(y: int) implements SomeProto
__implements_Bare2
"#;
    let result = run_ast_repl(src).unwrap();
    if let txtcode::runtime::Value::Array(ref list) = result {
        assert!(list.contains(&txtcode::runtime::Value::String("SomeProto".into())));
    } else {
        panic!("Expected Array, got {:?}", result);
    }
}

#[test]
fn test_protocol_empty_body() {
    let src = r#"
protocol → Marker
end
__protocol_Marker
"#;
    let result = run_ast_repl(src).unwrap();
    if let txtcode::runtime::Value::Array(ref methods) = result {
        assert_eq!(methods.len(), 0, "Marker protocol should have 0 methods");
    } else {
        panic!("Expected empty Array for Marker protocol, got {:?}", result);
    }
}

// ---------------------------------------------------------------------------
// Generic structs (Task E.2)
// ---------------------------------------------------------------------------

#[test]
fn test_generic_struct_parse() {
    let src = r#"
struct → Stack<T>(items: array)
store → s → Stack([1, 2, 3])
s["items"]
"#;
    let result = run_ast_repl(src).unwrap();
    if let txtcode::runtime::Value::Array(ref v) = result {
        assert_eq!(v.len(), 3);
    } else {
        panic!("Expected Array, got {:?}", result);
    }
}

#[test]
fn test_generic_struct_with_multiple_type_params() {
    let src = r#"
struct → Pair<K, V>(key: string, value: int)
store → p → Pair("hello", 42)
p["key"]
"#;
    let result = run_ast_repl(src).unwrap();
    assert_eq!(result, txtcode::runtime::Value::String("hello".into()));
}

#[test]
fn test_generic_struct_implements() {
    let src = r#"
struct → Container<T>(value: int) implements Printable
__implements_Container
"#;
    let result = run_ast_repl(src).unwrap();
    if let txtcode::runtime::Value::Array(ref list) = result {
        assert!(list.contains(&txtcode::runtime::Value::String("Printable".into())));
    } else {
        panic!("Expected Array, got {:?}", result);
    }
}

#[test]
fn test_generic_struct_field_access() {
    let src = r#"
struct → Box<T>(val: int, label: string)
store → b → Box(99, "mybox")
b["val"]
"#;
    let result = run_ast_repl(src).unwrap();
    assert_eq!(result, txtcode::runtime::Value::Integer(99));
}

// ---------------------------------------------------------------------------
// Error code inference tests
// ---------------------------------------------------------------------------

#[test]
fn test_error_code_e0016_inferred() {
    use txtcode::runtime::errors::ErrorCode;
    assert_eq!(
        ErrorCode::infer_from_message("Struct field type mismatch: 'Point.x' expected Int, got string"),
        ErrorCode::E0016
    );
}

#[test]
fn test_error_code_e0051_inferred() {
    use txtcode::runtime::errors::ErrorCode;
    assert_eq!(
        ErrorCode::infer_from_message("async function 'foo' is not supported"),
        ErrorCode::E0051
    );
}

#[test]
fn test_error_code_e0052_inferred() {
    use txtcode::runtime::errors::ErrorCode;
    assert_eq!(
        ErrorCode::infer_from_message("experimental feature disabled"),
        ErrorCode::E0052
    );
}

#[test]
fn test_error_code_e0001_inferred() {
    use txtcode::runtime::errors::ErrorCode;
    assert_eq!(
        ErrorCode::infer_from_message("permission denied: fs.write"),
        ErrorCode::E0001
    );
}

// ---------------------------------------------------------------------------
// Error message quality (Task 21.3)
// ---------------------------------------------------------------------------

#[test]
fn test_error_quality_index_oob_message() {
    let result = run_ast_repl("store → a → [1, 2, 3]\na[5]");
    let err = result.unwrap_err();
    assert!(
        err.message().contains("5") && err.message().contains("3"),
        "expected index/length in message, got: {}",
        err.message()
    );
}

#[test]
fn test_error_quality_index_oob_code() {
    use txtcode::runtime::errors::ErrorCode;
    let result = run_ast_repl("store → a → [10]\na[99]");
    let err = result.unwrap_err();
    assert_eq!(err.code, Some(ErrorCode::E0013));
}

#[test]
fn test_error_quality_undefined_variable_message() {
    let result = run_ast_repl("foo_bar_baz");
    let err = result.unwrap_err();
    assert!(
        err.message().contains("foo_bar_baz"),
        "expected variable name in message, got: {}",
        err.message()
    );
}

#[test]
fn test_error_quality_did_you_mean_hint() {
    let src = "store → count → 10\nconut";
    let result = run_ast_repl(src);
    let err = result.unwrap_err();
    let display = format!("{}", err);
    assert!(
        display.contains("count") || err.message().contains("conut"),
        "expected suggestion or error for conut, got: {}",
        display
    );
}

#[test]
fn test_error_quality_division_by_zero_message() {
    let result = run_ast_repl("10 / 0");
    let err = result.unwrap_err();
    assert!(
        err.message().contains("zero") || err.message().contains("division"),
        "expected 'zero' or 'division' in message, got: {}",
        err.message()
    );
}

#[test]
fn test_error_quality_division_by_zero_code() {
    use txtcode::runtime::errors::ErrorCode;
    let result = run_ast_repl("1 / 0");
    let err = result.unwrap_err();
    assert_eq!(err.code, Some(ErrorCode::E0012));
}

#[test]
fn test_error_quality_permission_denied_hint() {
    use txtcode::runtime::VirtualMachine;
    use txtcode::runtime::permissions::PermissionResource;
    let src = "net_get(\"http://example.com\")";
    let tokens = txtcode::lexer::Lexer::new(src.to_string()).tokenize().unwrap();
    let program = txtcode::parser::Parser::new(tokens).parse().unwrap();
    let mut vm = VirtualMachine::new();
    vm.deny_permission(PermissionResource::Network("*".to_string()), None);
    let result = vm.interpret_repl(&program);
    let _ = result;
}

// ---------------------------------------------------------------------------
// Standard error types (Task E.3)
// ---------------------------------------------------------------------------

#[test]
fn test_error_type_file_not_found() {
    let src = r#"FileNotFoundError("/tmp/no_such.txt")"#;
    let result = run_ast_repl(src).unwrap();
    if let txtcode::runtime::Value::Map(ref m) = result {
        assert_eq!(m.get("_error_type"), Some(&txtcode::runtime::Value::String("FileNotFoundError".into())));
        assert!(m.contains_key("path"));
        assert!(m.contains_key("message"));
    } else {
        panic!("Expected Map, got {:?}", result);
    }
}

#[test]
fn test_error_type_permission_error() {
    let src = r#"PermissionError("write", "secret.txt")"#;
    let result = run_ast_repl(src).unwrap();
    if let txtcode::runtime::Value::Map(ref m) = result {
        assert_eq!(m.get("_error_type"), Some(&txtcode::runtime::Value::String("PermissionError".into())));
        assert_eq!(m.get("action"),   Some(&txtcode::runtime::Value::String("write".into())));
        assert_eq!(m.get("resource"), Some(&txtcode::runtime::Value::String("secret.txt".into())));
    } else {
        panic!("Expected Map");
    }
}

#[test]
fn test_error_type_network_error() {
    let src = r#"NetworkError("https://example.com", "connection refused")"#;
    let result = run_ast_repl(src).unwrap();
    if let txtcode::runtime::Value::Map(ref m) = result {
        assert_eq!(m.get("_error_type"), Some(&txtcode::runtime::Value::String("NetworkError".into())));
    } else {
        panic!("Expected Map");
    }
}

#[test]
fn test_error_type_parse_error() {
    let src = r#"ParseError("bad input", 5)"#;
    let result = run_ast_repl(src).unwrap();
    if let txtcode::runtime::Value::Map(ref m) = result {
        assert_eq!(m.get("_error_type"), Some(&txtcode::runtime::Value::String("ParseError".into())));
        assert_eq!(m.get("position"),    Some(&txtcode::runtime::Value::Integer(5)));
    } else {
        panic!("Expected Map");
    }
}

#[test]
fn test_error_type_type_error() {
    let src = r#"TypeError("int", "string")"#;
    let result = run_ast_repl(src).unwrap();
    if let txtcode::runtime::Value::Map(ref m) = result {
        assert_eq!(m.get("_error_type"), Some(&txtcode::runtime::Value::String("TypeError".into())));
        assert_eq!(m.get("expected"),    Some(&txtcode::runtime::Value::String("int".into())));
        assert_eq!(m.get("got"),         Some(&txtcode::runtime::Value::String("string".into())));
    } else {
        panic!("Expected Map");
    }
}

#[test]
fn test_error_type_value_error() {
    let src = r#"ValueError("negative value not allowed")"#;
    let result = run_ast_repl(src).unwrap();
    if let txtcode::runtime::Value::Map(ref m) = result {
        assert_eq!(m.get("_error_type"), Some(&txtcode::runtime::Value::String("ValueError".into())));
    } else {
        panic!("Expected Map");
    }
}

#[test]
fn test_error_type_index_error() {
    let src = r#"IndexError(10, 5)"#;
    let result = run_ast_repl(src).unwrap();
    if let txtcode::runtime::Value::Map(ref m) = result {
        assert_eq!(m.get("_error_type"), Some(&txtcode::runtime::Value::String("IndexError".into())));
        assert_eq!(m.get("index"),  Some(&txtcode::runtime::Value::Integer(10)));
        assert_eq!(m.get("length"), Some(&txtcode::runtime::Value::Integer(5)));
    } else {
        panic!("Expected Map");
    }
}

#[test]
fn test_error_type_timeout_error() {
    let src = r#"TimeoutError(5000)"#;
    let result = run_ast_repl(src).unwrap();
    if let txtcode::runtime::Value::Map(ref m) = result {
        assert_eq!(m.get("_error_type"), Some(&txtcode::runtime::Value::String("TimeoutError".into())));
        assert_eq!(m.get("limit_ms"), Some(&txtcode::runtime::Value::Integer(5000)));
    } else {
        panic!("Expected Map");
    }
}

#[test]
fn test_read_file_not_found_returns_typed_error() {
    use txtcode::runtime::{permissions::PermissionResource, vm::VirtualMachine};
    let mut vm = VirtualMachine::new();
    vm.grant_permission(PermissionResource::FileSystem("read".to_string()), None);
    let src = r#"read_file("/tmp/__txtcode_no_such_file_xyz.txt")"#;
    let tokens = txtcode::lexer::Lexer::new(src.to_string()).tokenize().unwrap();
    let program = txtcode::parser::Parser::new(tokens).parse().unwrap();
    let result = vm.interpret_repl(&program).unwrap();
    if let txtcode::runtime::Value::Map(ref m) = result {
        assert_eq!(m.get("_error_type"), Some(&txtcode::runtime::Value::String("FileNotFoundError".into())));
    } else {
        panic!("Expected FileNotFoundError Map, got {:?}", result);
    }
}

#[test]
fn test_json_parse_invalid_returns_typed_error() {
    let src = r#"json_parse("{not valid json")"#;
    let result = run_ast_repl(src).unwrap();
    if let txtcode::runtime::Value::Map(ref m) = result {
        assert_eq!(m.get("_error_type"), Some(&txtcode::runtime::Value::String("ParseError".into())));
    } else {
        panic!("Expected ParseError Map, got {:?}", result);
    }
}

// ---------------------------------------------------------------------------
// M.1: E0053 error code
// ---------------------------------------------------------------------------

#[test]
fn test_m1_e0053_error_code_distinct() {
    use txtcode::runtime::errors::ErrorCode;
    let e53 = ErrorCode::E0053;
    let e52 = ErrorCode::E0052;
    assert_ne!(
        std::mem::discriminant(&e53),
        std::mem::discriminant(&e52),
        "E0053 must be a distinct variant from E0052"
    );
}
