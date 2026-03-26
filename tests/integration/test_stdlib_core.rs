use std::sync::Arc;
use txtcode::lexer::Lexer;
use txtcode::parser::Parser;
use txtcode::runtime::vm::VirtualMachine;

fn run_ast_source(
    source: &str,
) -> Result<txtcode::runtime::Value, txtcode::runtime::errors::RuntimeError> {
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut vm = VirtualMachine::new();
    vm.interpret(&program)
}

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

fn run_with_sys_info(src: &str) -> Result<txtcode::runtime::Value, txtcode::runtime::errors::RuntimeError> {
    use txtcode::runtime::permissions::PermissionResource;
    let mut lexer = Lexer::new(src.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut vm = VirtualMachine::new();
    vm.grant_permission(PermissionResource::System("info".to_string()), None);
    vm.interpret_repl(&program)
}

// ---------------------------------------------------------------------------
// Substring tests
// ---------------------------------------------------------------------------

#[test]
fn test_substring_ascii_valid() {
    let result = run_ast_repl(r#"substring("hello", 1, 3)"#);
    assert!(result.is_ok(), "{:?}", result);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String(Arc::from("el")));
}

#[test]
fn test_substring_unicode_valid() {
    let result = run_ast_repl(r#"substring("héllo", 0, 2)"#);
    assert!(result.is_ok(), "{:?}", result);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String(Arc::from("hé")));
}

#[test]
fn test_substring_unicode_mid_char() {
    let result = run_ast_repl(r#"substring("héllo", 1, 2)"#);
    assert!(result.is_ok(), "{:?}", result);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String(Arc::from("é")));
}

#[test]
fn test_substring_negative_start_errors() {
    let result = run_ast_source(r#"store → r → substring("hello", -1, 3)"#);
    assert!(result.is_err(), "expected error for negative start index");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("non-negative"), "error message: {}", msg);
}

#[test]
fn test_substring_oob_errors() {
    let result = run_ast_source(r#"store → r → substring("hello", 2, 10)"#);
    assert!(result.is_err(), "expected error for out-of-bounds end index");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("out of bounds"), "error message: {}", msg);
}

// ---------------------------------------------------------------------------
// str_pad tests
// ---------------------------------------------------------------------------

#[test]
fn test_str_pad_left_valid() {
    let result = run_ast_repl(r#"str_pad_left("hi", 5)"#);
    assert!(result.is_ok(), "{:?}", result);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String(Arc::from("   hi")));
}

#[test]
fn test_str_pad_left_negative_width_errors() {
    let result = run_ast_source(r#"store → r → str_pad_left("x", -1)"#);
    assert!(result.is_err(), "expected error for negative width");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("non-negative"), "error message: {}", msg);
}

#[test]
fn test_str_pad_right_valid() {
    let result = run_ast_repl(r#"str_pad_right("hi", 5)"#);
    assert!(result.is_ok(), "{:?}", result);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String(Arc::from("hi   ")));
}

#[test]
fn test_str_pad_right_negative_width_errors() {
    let result = run_ast_source(r#"store → r → str_pad_right("x", -5)"#);
    assert!(result.is_err(), "expected error for negative width");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("non-negative"), "error message: {}", msg);
}

// ---------------------------------------------------------------------------
// math_random_int tests
// ---------------------------------------------------------------------------

#[test]
fn test_math_random_int_valid() {
    let result = run_ast_repl(r#"math_random_int(1, 100)"#);
    assert!(result.is_ok(), "{:?}", result);
    if let Ok(txtcode::runtime::Value::Integer(n)) = result {
        assert!((1..=100).contains(&n), "got out-of-range value: {}", n);
    } else {
        panic!("expected integer");
    }
}

#[test]
fn test_math_random_int_equal_bounds() {
    let result = run_ast_repl(r#"math_random_int(5, 5)"#);
    assert!(result.is_ok(), "{:?}", result);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(5));
}

#[test]
fn test_math_random_int_inverted_range_errors() {
    let result = run_ast_source(r#"store → r → math_random_int(10, 1)"#);
    assert!(result.is_err(), "inverted range should error");
}

// ---------------------------------------------------------------------------
// str_format, str_repeat, str_contains, str_chars, str_reverse, str_center
// ---------------------------------------------------------------------------

#[test]
fn test_str_format_sequential() {
    let result = run_ast_repl(r#"str_format("{} + {} = {}", 1, 2, 3)"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String(Arc::from("1 + 2 = 3")));
}

#[test]
fn test_str_format_positional() {
    let result = run_ast_repl(r#"str_format("{1} before {0}", "world", "hello")"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String(Arc::from("hello before world")));
}

#[test]
fn test_format_alias() {
    let result = run_ast_repl(r#"format("x={}", 42)"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String(Arc::from("x=42")));
}

#[test]
fn test_str_repeat() {
    let result = run_ast_repl(r#"str_repeat("ab", 3)"#);
    assert!(result.is_ok(), "{:?}", result);
}

#[test]
fn test_str_repeat_zero() {
    let result = run_ast_repl(r#"str_repeat("ab", 0)"#);
    assert!(result.is_ok(), "{:?}", result);
}

#[test]
fn test_str_contains_true() {
    let result = run_ast_repl(r#"str_contains("hello world", "world")"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Boolean(true));
}

#[test]
fn test_str_contains_false() {
    let result = run_ast_repl(r#"str_contains("hello", "xyz")"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Boolean(false));
}

#[test]
fn test_str_chars() {
    let result = run_ast_repl(r#"str_chars("abc")"#);
    assert!(result.is_ok(), "{:?}", result);
}

#[test]
fn test_str_reverse() {
    let result = run_ast_repl(r#"str_reverse("hello")"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String(Arc::from("olleh")));
}

#[test]
fn test_str_center() {
    let result = run_ast_repl(r#"str_center("hi", 6)"#);
    assert!(result.is_ok(), "{:?}", result);
}

#[test]
fn test_str_center_custom_pad() {
    let result = run_ast_repl(r#"str_center("hi", 6, "-")"#);
    assert!(result.is_ok(), "{:?}", result);
}

// ---------------------------------------------------------------------------
// str_build (R.4)
// ---------------------------------------------------------------------------

#[test]
fn test_r4_str_build_empty_array() {
    let result = run_ast_repl("str_build([])");
    assert!(result.is_ok(), "str_build([]) should return empty string: {:?}", result);
    match result.unwrap() {
        txtcode::runtime::Value::String(s) => assert_eq!(s.as_ref(), "", "empty array → empty string"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_r4_str_build_concatenates_parts() {
    let result = run_ast_repl(r#"str_build(["hello", " ", "world"])"#);
    assert!(result.is_ok(), "str_build should concatenate: {:?}", result);
    match result.unwrap() {
        txtcode::runtime::Value::String(s) => assert_eq!(s.as_ref(), "hello world"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_r4_str_build_with_numbers() {
    let result = run_ast_repl(r#"str_build(["item_", 42])"#);
    assert!(result.is_ok(), "str_build with numbers: {:?}", result);
    match result.unwrap() {
        txtcode::runtime::Value::String(s) => assert_eq!(s.as_ref(), "item_42"),
        other => panic!("expected string, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// array_* functions
// ---------------------------------------------------------------------------

#[test]
fn test_array_sum_int() {
    let result = run_ast_repl("array_sum([1, 2, 3, 4])");
    assert!(result.is_ok(), "{:?}", result);
}

#[test]
fn test_array_sum_float() {
    let result = run_ast_repl("array_sum([1.5, 2.5])");
    assert!(result.is_ok(), "{:?}", result);
}

#[test]
fn test_array_flatten() {
    let result = run_ast_repl("array_flatten([[1, 2], [3, 4]])");
    assert!(result.is_ok(), "{:?}", result);
}

#[test]
fn test_array_enumerate() {
    let result = run_ast_repl("array_enumerate([\"a\", \"b\"])");
    assert!(result.is_ok(), "{:?}", result);
}

#[test]
fn test_array_zip() {
    let result = run_ast_repl("array_zip([1, 2], [3, 4])");
    assert!(result.is_ok(), "{:?}", result);
}

#[test]
fn test_array_contains_true() {
    let result = run_ast_repl("array_contains([1, 2, 3], 2)");
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Boolean(true));
}

#[test]
fn test_array_contains_false() {
    let result = run_ast_repl("array_contains([1, 2, 3], 99)");
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Boolean(false));
}

#[test]
fn test_array_push() {
    let result = run_ast_repl("array_push([1, 2], 3)");
    assert!(result.is_ok(), "{:?}", result);
}

#[test]
fn test_array_head() {
    let result = run_ast_repl("array_head([10, 20, 30])");
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(10));
}

#[test]
fn test_array_head_empty() {
    let result = run_ast_repl("array_head([])");
    assert!(result.is_ok(), "{:?}", result);
}

#[test]
fn test_array_tail() {
    let result = run_ast_repl("array_tail([10, 20, 30])");
    assert!(result.is_ok(), "{:?}", result);
}

#[test]
fn test_array_tail_empty() {
    let result = run_ast_repl("array_tail([])");
    assert!(result.is_ok(), "{:?}", result);
}

// zip / chain (iterator protocol, Group 14.3)
#[test]
fn test_zip_basic() {
    use txtcode::runtime::Value;
    let src = r#"
store → result → 0
for → pair in zip([1, 2, 3], [10, 20, 30])
  result += pair[0] * pair[1]
end
result
"#;
    assert_eq!(run(src), Value::Integer(140));
}

#[test]
fn test_chain_basic() {
    use txtcode::runtime::Value;
    let src = "store → total → 0\nfor → x in chain([1, 2], [3, 4])\n  total += x\nend\ntotal";
    assert_eq!(run(src), Value::Integer(10));
}

// ---------------------------------------------------------------------------
// Regex tests (Group 20.1)
// ---------------------------------------------------------------------------

#[test]
fn test_regex_match_basic() {
    let result = run_ast_repl(r#"regex_match("[0-9]+", "abc123")"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Boolean(true));
}

#[test]
fn test_regex_match_no_match() {
    let result = run_ast_repl(r#"regex_match("[0-9]+", "abc")"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Boolean(false));
}

#[test]
fn test_regex_find_returns_match_map() {
    let result = run_ast_repl(r#"regex_find("[0-9]+", "abc123def")["match"]"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String(Arc::from("123")));
}

#[test]
fn test_regex_find_no_match_returns_null() {
    let result = run_ast_repl(r#"regex_find("[0-9]+", "abc")"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Null);
}

#[test]
fn test_regex_find_all_returns_array() {
    let result = run_ast_repl(r#"len(regex_find_all("[0-9]+", "a1b22c333"))"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(3));
}

#[test]
fn test_regex_replace_first() {
    let result = run_ast_repl(r#"regex_replace("[0-9]+", "abc123def456", "NUM")"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String(Arc::from("abcNUMdef456")));
}

#[test]
fn test_regex_replace_all() {
    let result = run_ast_repl(r#"regex_replace_all("[0-9]+", "abc123def456", "NUM")"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String(Arc::from("abcNUMdefNUM")));
}

#[test]
fn test_regex_split_basic() {
    let result = run_ast_repl(r#"len(regex_split(",", "a,b,c"))"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(3));
}

#[test]
fn test_regex_invalid_pattern_errors() {
    let result = run_ast_repl(r#"regex_match("[invalid", "text")"#);
    assert!(result.is_err(), "invalid regex pattern should return error");
}

#[test]
fn test_regex_cache_correctness_match() {
    for text in &["hello world", "abc 123", "no digits here"] {
        let src = format!(r#"regex_match("[0-9]+", "{}")"#, text);
        let result = run_ast_repl(&src);
        let expected = text.chars().any(|c| c.is_ascii_digit());
        assert_eq!(
            result.unwrap(),
            txtcode::runtime::Value::Boolean(expected),
            "cached regex_match gave wrong result for '{}'",
            text
        );
    }
}

#[test]
fn test_regex_cache_correctness_split() {
    let result1 = run_ast_repl(r#"regex_split("\\s+", "hello   world")"#).unwrap();
    let result2 = run_ast_repl(r#"regex_split("\\s+", "hello   world")"#).unwrap();
    assert_eq!(result1, result2, "regex_split must give identical results on cache hit");
    match result1 {
        txtcode::runtime::Value::Array(parts) => {
            assert_eq!(parts.len(), 2);
            assert_eq!(parts[0], txtcode::runtime::Value::String(Arc::from("hello")));
            assert_eq!(parts[1], txtcode::runtime::Value::String(Arc::from("world")));
        }
        other => panic!("expected array, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// assert_* functions (Group F.4)
// ---------------------------------------------------------------------------

#[test]
fn test_assert_eq_passes_when_equal() {
    let result = run_ast_repl("assert_eq(42, 42)");
    assert!(result.is_ok(), "assert_eq should pass: {:?}", result);
}

#[test]
fn test_assert_eq_fails_when_not_equal() {
    let result = run_ast_repl("assert_eq(1, 2)");
    assert!(result.is_err(), "assert_eq should fail when values differ");
    assert!(result.unwrap_err().message().contains("Assertion failed"));
}

#[test]
fn test_assert_ne_passes_when_different() {
    let result = run_ast_repl("assert_ne(1, 2)");
    assert!(result.is_ok(), "assert_ne should pass: {:?}", result);
}

#[test]
fn test_assert_ne_fails_when_equal() {
    let result = run_ast_repl("assert_ne(5, 5)");
    assert!(result.is_err(), "assert_ne should fail when values are equal");
}

#[test]
fn test_assert_true_passes() {
    let result = run_ast_repl("assert_true(1 == 1)");
    assert!(result.is_ok(), "assert_true should pass: {:?}", result);
}

#[test]
fn test_assert_false_passes() {
    let result = run_ast_repl("assert_false(1 == 2)");
    assert!(result.is_ok(), "assert_false should pass: {:?}", result);
}

#[test]
fn test_assert_error_passes_on_error_value() {
    let result = run_ast_repl(r#"assert_error(err("boom"))"#);
    assert!(result.is_ok(), "assert_error should pass on error value: {:?}", result);
}

#[test]
fn test_assert_error_fails_on_ok_value() {
    let result = run_ast_repl(r#"assert_error(ok(42))"#);
    assert!(result.is_err(), "assert_error should fail on ok value");
}

#[test]
fn test_assert_type_int() {
    let result = run_ast_repl(r#"assert_type(42, "int")"#);
    assert!(result.is_ok(), "assert_type int: {:?}", result);
}

#[test]
fn test_assert_type_string() {
    let result = run_ast_repl(r#"assert_type("hello", "string")"#);
    assert!(result.is_ok(), "assert_type string: {:?}", result);
}

#[test]
fn test_assert_type_fails_wrong_type() {
    let result = run_ast_repl(r#"assert_type(42, "string")"#);
    assert!(result.is_err(), "assert_type should fail with wrong type");
}

#[test]
fn test_assert_contains_string() {
    let result = run_ast_repl(r#"assert_contains("hello world", "world")"#);
    assert!(result.is_ok(), "assert_contains string: {:?}", result);
}

#[test]
fn test_assert_contains_array() {
    let result = run_ast_repl("assert_contains([1, 2, 3], 2)");
    assert!(result.is_ok(), "assert_contains array: {:?}", result);
}

#[test]
fn test_assert_approx_passes_within_epsilon() {
    let result = run_ast_repl("assert_approx(3.14, 3.141, 0.01)");
    assert!(result.is_ok(), "assert_approx within epsilon: {:?}", result);
}

#[test]
fn test_assert_approx_fails_outside_epsilon() {
    let result = run_ast_repl("assert_approx(1.0, 2.0, 0.5)");
    assert!(result.is_err(), "assert_approx should fail outside epsilon");
}

// ---------------------------------------------------------------------------
// JSON / XML / YAML / TOML parse/stringify
// ---------------------------------------------------------------------------

#[test]
fn test_yaml_parse_stringify_roundtrip() {
    use txtcode::runtime::Value;
    let src = r#"
store → obj → {"name": "alice", "age": 30}
store → yaml_str → yaml_stringify(obj)
store → back → yaml_parse(yaml_str)
back["name"]
"#;
    let tokens = Lexer::new(src.to_string()).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap();
    let result = VirtualMachine::new().interpret_repl(&program).unwrap();
    assert_eq!(result, Value::String(Arc::from("alice")));
}

#[test]
fn test_toml_parse_stringify_roundtrip() {
    use txtcode::runtime::Value;
    let src = r#"
store → obj → {"key": "value"}
store → toml_str → toml_stringify(obj)
store → back → toml_parse(toml_str)
back["key"]
"#;
    let tokens = Lexer::new(src.to_string()).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap();
    let result = VirtualMachine::new().interpret_repl(&program).unwrap();
    assert_eq!(result, Value::String(Arc::from("value")));
}

#[test]
fn test_xml_stringify_simple_element() {
    use txtcode::runtime::Value;
    let source = r#"xml_stringify({"_tag": "item", "id": "1", "_text": "hello"})"#.to_string();
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut vm = VirtualMachine::new();
    let val = vm.interpret_repl(&program).unwrap();
    let Value::String(xml) = val else { panic!("expected String, got {:?}", val); };
    assert!(xml.contains("<item"), "missing element: {}", xml);
    assert!(xml.contains("id=\"1\""), "missing attribute: {}", xml);
    assert!(xml.contains("hello"), "missing text: {}", xml);
}

#[test]
fn test_xml_stringify_self_closing_element() {
    use txtcode::runtime::Value;
    let source = r#"xml_stringify({"_tag": "br"})"#.to_string();
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut vm = VirtualMachine::new();
    let val = vm.interpret_repl(&program).unwrap();
    let Value::String(xml) = val else { panic!("expected String, got {:?}", val); };
    assert!(xml.contains("<br/>") || xml.contains("<br />"), "expected self-closing: {}", xml);
}

// ---------------------------------------------------------------------------
// Template engine (Task 17.3)
// ---------------------------------------------------------------------------

#[test]
fn test_template_variable_substitution() {
    use txtcode::runtime::Value;
    let src = r#"
store → ctx → {"name": "world", "lang": "NPL"}
template_render("Hello, {{name}}! Welcome to {{lang}}.", ctx)
"#;
    let tokens = Lexer::new(src.to_string()).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap();
    let result = VirtualMachine::new().interpret_repl(&program).unwrap();
    assert_eq!(result, Value::String(Arc::from("Hello, world! Welcome to NPL.")));
}

#[test]
fn test_template_if_else() {
    use txtcode::runtime::Value;
    let src = r#"
store → ctx → {"admin": true}
template_render("{{#if admin}}admin{{else}}user{{/if}}", ctx)
"#;
    let tokens = Lexer::new(src.to_string()).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap();
    let result = VirtualMachine::new().interpret_repl(&program).unwrap();
    assert_eq!(result, Value::String(Arc::from("admin")));
}

#[test]
fn test_template_each_loop() {
    use txtcode::runtime::Value;
    let src = r#"
store → ctx → {"items": ["a", "b", "c"]}
template_render("{{#each items as item}}{{item}},{{/each}}", ctx)
"#;
    let tokens = Lexer::new(src.to_string()).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap();
    let result = VirtualMachine::new().interpret_repl(&program).unwrap();
    assert_eq!(result, Value::String(Arc::from("a,b,c,")));
}

// ---------------------------------------------------------------------------
// System info (R.3)
// ---------------------------------------------------------------------------

#[test]
fn test_r3_cpu_count_returns_positive_integer() {
    let result = run_with_sys_info("cpu_count()");
    assert!(result.is_ok(), "cpu_count() should not error: {:?}", result.err());
    match result.unwrap() {
        txtcode::runtime::Value::Integer(n) => assert!(n >= 1, "cpu_count() must be >= 1, got {}", n),
        other => panic!("cpu_count() should return Integer, got {:?}", other),
    }
}

#[test]
fn test_r3_platform_returns_nonempty_string() {
    let result = run_with_sys_info("platform()");
    assert!(result.is_ok(), "platform() should not error: {:?}", result.err());
    match result.unwrap() {
        txtcode::runtime::Value::String(s) => assert!(!s.is_empty(), "platform() returned empty string"),
        other => panic!("platform() should return String, got {:?}", other),
    }
}

#[test]
fn test_r3_memory_available_returns_integer() {
    let result = run_with_sys_info("memory_available()");
    assert!(result.is_ok(), "memory_available() should not error: {:?}", result.err());
    match result.unwrap() {
        txtcode::runtime::Value::Integer(n) => assert!(n >= 0, "memory_available() must be >= 0, got {}", n),
        other => panic!("memory_available() should return Integer, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// sleep_basic
// ---------------------------------------------------------------------------

#[test]
fn test_sleep_basic() {
    use txtcode::runtime::Value;
    let src = r#"
sleep(1)
"done"
"#;
    let result = run(src);
    assert_eq!(result, Value::String(Arc::from("done")));
}

// ---------------------------------------------------------------------------
// CSV decode/roundtrip (stdlib, not file-based)
// ---------------------------------------------------------------------------

#[test]
fn test_csv_decode_basic() {
    let result = run_ast_repl(r#"len(csv_decode("a,b,c\n1,2,3"))"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(2));
}

#[test]
fn test_csv_roundtrip() {
    let result = run_ast_repl(r#"csv_decode(csv_to_string([["x", "y"], [1, 2]]))"#);
    match result.unwrap() {
        txtcode::runtime::Value::Array(rows) => assert_eq!(rows.len(), 2),
        other => panic!("expected Array, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// datetime / time functions (used from older tests that belong here)
// ---------------------------------------------------------------------------

#[test]
fn test_now_utc_returns_iso8601() {
    let result = run_ast_repl("now_utc()");
    match result {
        Ok(txtcode::runtime::Value::String(s)) => {
            assert!(s.len() >= 10, "now_utc() should return a date string");
            assert!(s.contains('T') || s.len() == 10, "should be ISO 8601: {}", s);
        }
        other => panic!("expected String, got {:?}", other),
    }
}

#[test]
fn test_now_local_returns_string() {
    let result = run_ast_repl("now_local()");
    assert!(result.is_ok(), "{:?}", result);
    assert!(matches!(result.unwrap(), txtcode::runtime::Value::String(_)));
}

#[test]
fn test_datetime_add_days() {
    let result = run_ast_repl("datetime_add(0, 1, \"days\")");
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(86400));
}

#[test]
fn test_datetime_add_hours() {
    let result = run_ast_repl("datetime_add(0, 2, \"hours\")");
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(7200));
}

#[test]
fn test_datetime_diff_days() {
    let result = run_ast_repl("datetime_diff(86400, 0, \"days\")");
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(1));
}

#[test]
fn test_format_datetime_utc() {
    let result = run_ast_repl("format_datetime(0, \"%Y-%m-%d\", \"UTC\")");
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String(Arc::from("1970-01-01")));
}

#[test]
fn test_time_format_epoch_utc() {
    let result = run_ast_repl(r#"format_datetime(0, "%Y-%m-%d", "UTC")"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String(Arc::from("1970-01-01")));
}

#[test]
fn test_time_format_local_no_crash() {
    let result = run_ast_repl("format_time(0, \"%Y\")");
    assert!(matches!(result.unwrap(), txtcode::runtime::Value::String(_)));
}

#[test]
fn test_datetime_add_days_v2() {
    let result = run_ast_repl(r#"datetime_add(0, 1, "days")"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(86400));
}

#[test]
fn test_datetime_add_hours_v2() {
    let result = run_ast_repl(r#"datetime_add(0, 2, "hours")"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(7200));
}

#[test]
fn test_datetime_diff_seconds_v2() {
    let result = run_ast_repl(r#"datetime_diff(100, 40, "seconds")"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(60));
}

#[test]
fn test_datetime_diff_days_v2() {
    let result = run_ast_repl(r#"datetime_diff(86400, 0, "days")"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(1));
}

#[test]
fn test_now_utc_returns_string() {
    let result = run_ast_repl("now_utc()");
    match result.unwrap() {
        txtcode::runtime::Value::String(s) => assert!(s.contains("T"), "now_utc should return ISO8601: {}", s),
        other => panic!("expected String, got {:?}", other),
    }
}

// logging
#[test]
fn test_log_info_returns_null() {
    let result = run_ast_repl(r#"log_info("hello world")"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Null);
}

#[test]
fn test_log_warn_returns_null() {
    let result = run_ast_repl(r#"log_warn("something fishy")"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Null);
}

#[test]
fn test_log_error_returns_null() {
    let result = run_ast_repl(r#"log_error("something broke")"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Null);
}

#[test]
fn test_log_debug_returns_null() {
    let result = run_ast_repl(r#"log_debug("verbose info")"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Null);
}

#[test]
fn test_log_multi_args() {
    let result = run_ast_repl(r#"log_info("hello", "world", 42)"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Null);
}

// ---------------------------------------------------------------------------
// f-string tests (13.3)
// ---------------------------------------------------------------------------

#[test]
fn test_fstring_basic() {
    use txtcode::runtime::Value;
    assert_eq!(
        run("store → x → 42\nf\"{x}\""),
        Value::String(Arc::from("42"))
    );
}

#[test]
fn test_fstring_adjacent_interpolations() {
    use txtcode::runtime::Value;
    assert_eq!(
        run("store → a → \"hello\"\nstore → b → \"world\"\nf\"{a}{b}\""),
        Value::String(Arc::from("helloworld"))
    );
}

#[test]
fn test_fstring_escaped_brace_not_interpolated() {
    use txtcode::runtime::Value;
    assert_eq!(
        run("f\"literal \\{not interpolated\\}\""),
        Value::String(Arc::from("literal {not interpolated}"))
    );
}

#[test]
fn test_fstring_nested_braces_in_expr() {
    use txtcode::runtime::Value;
    let src = "store → m → {x: 42}\nf\"{m[\"x\"]}\"";
    let result = run(src);
    // Just verify it doesn't panic
    let _ = result;
}

#[test]
fn test_fstring_expr_with_arithmetic() {
    use txtcode::runtime::Value;
    assert_eq!(
        run("store → a → 3\nstore → b → 4\nf\"{a + b}\""),
        Value::String(Arc::from("7"))
    );
}

// ---------------------------------------------------------------------------
// Gzip (Group 27.3)
// ---------------------------------------------------------------------------

#[test]
fn test_gzip_compress_decompress_roundtrip() {
    use txtcode::stdlib::BytesLib;
    use txtcode::runtime::Value;
    let data = Value::String(Arc::from("Hello, Txtcode gzip compression!"));
    let compressed = BytesLib::call_function("gzip_compress", &[data]).unwrap();
    let Value::Bytes(compressed_bytes) = &compressed else { panic!("expected Bytes"); };
    assert!(!compressed_bytes.is_empty());
    let decompressed = BytesLib::call_function("gzip_decompress", &[compressed]).unwrap();
    let Value::Bytes(result) = decompressed else { panic!("expected Bytes"); };
    let s = String::from_utf8(result).unwrap();
    assert_eq!(s.as_str(), "Hello, Txtcode gzip compression!");
}

#[test]
fn test_gzip_compress_reduces_size_on_repetitive_data() {
    use txtcode::stdlib::BytesLib;
    use txtcode::runtime::Value;
    let input = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".repeat(20);
    let data = Value::String(Arc::from(input.as_str()));
    let compressed = BytesLib::call_function("gzip_compress", &[data]).unwrap();
    let Value::Bytes(compressed_bytes) = compressed else { panic!("expected Bytes"); };
    assert!(compressed_bytes.len() < input.len());
}

#[test]
fn test_gzip_decompress_string_roundtrip() {
    use txtcode::stdlib::BytesLib;
    use txtcode::runtime::Value;
    let original = "Txtcode compression test string.".to_string();
    let compressed = BytesLib::call_function(
        "gzip_compress",
        &[Value::String(Arc::from(original.as_str()))],
    ).unwrap();
    let decompressed_str = BytesLib::call_function(
        "gzip_decompress_string",
        &[compressed],
    ).unwrap();
    let Value::String(result) = decompressed_str else { panic!("expected String"); };
    assert_eq!(result.as_ref(), original.as_str());
}
