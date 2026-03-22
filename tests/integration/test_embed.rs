use std::sync::Arc;
/// Integration tests for Task 28.2 — Embedding API

use txtcode::embed::TxtcodeEngine;
use txtcode::runtime::core::Value;

// ── Test 1: Rust API eval ─────────────────────────────────────────────────────

#[test]
fn test_embed_eval_arithmetic() {
    let mut engine = TxtcodeEngine::new();
    let result = engine.eval("1 + 1").expect("eval should succeed");
    assert_eq!(result, Value::Integer(2));
}

#[test]
fn test_embed_eval_string_expr() {
    let mut engine = TxtcodeEngine::new();
    let result = engine.eval(r#""hello" + " world""#).expect("eval should succeed");
    assert_eq!(result, Value::String(Arc::from("hello world")));
}

// ── Test 2: set/get variable ──────────────────────────────────────────────────

#[test]
fn test_embed_set_get_integer() {
    let mut engine = TxtcodeEngine::new();
    engine.set("x", Value::Integer(42));
    // Script reads x and doubles it
    let result = engine.eval("x * 2").expect("eval should succeed");
    assert_eq!(result, Value::Integer(84));
}

#[test]
fn test_embed_get_variable_set_by_script() {
    let mut engine = TxtcodeEngine::new();
    engine.eval("store → answer → 100").expect("eval should succeed");
    let val = engine.get("answer");
    assert_eq!(val, Some(Value::Integer(100)));
}

#[test]
fn test_embed_set_string_and_use() {
    let mut engine = TxtcodeEngine::new();
    engine.set("greeting", Value::String(Arc::from("hello")));
    let result = engine.eval(r#"greeting + "!""#).expect("eval should succeed");
    assert_eq!(result, Value::String(Arc::from("hello!")));
}

// ── Test 3: register host function ───────────────────────────────────────────

#[test]
fn test_embed_register_fn_and_call() {
    let mut engine = TxtcodeEngine::new();
    engine.register_fn("double", |args| {
        if let Some(Value::Integer(n)) = args.first() {
            Value::Integer(n * 2)
        } else {
            Value::Null
        }
    });
    // Call the native function through the VM (verifies end-to-end dispatch)
    let result = engine.eval("double(21)").expect("native fn call should succeed");
    assert_eq!(result, Value::Integer(42));
}

#[test]
fn test_embed_register_fn_direct_lookup() {
    // Verify call_native works for direct Rust-side invocation
    let mut engine = TxtcodeEngine::new();
    engine.register_fn("triple", |args| {
        if let Some(Value::Integer(n)) = args.first() {
            Value::Integer(n * 3)
        } else {
            Value::Null
        }
    });
    let result = txtcode::embed::call_native("triple", &[Value::Integer(10)]);
    assert_eq!(result, Some(Value::Integer(30)));
}

#[test]
fn test_embed_default_trait() {
    let mut engine = TxtcodeEngine::default();
    let result = engine.eval("3 * 7").expect("eval should succeed");
    assert_eq!(result, Value::Integer(21));
}

// ── Task I.1: eval_string + last_error_code ───────────────────────────────────

#[test]
fn test_embed_eval_string_success() {
    let mut engine = TxtcodeEngine::new();
    let result = engine.eval_string("1 + 1");
    assert_eq!(result, Ok("2".to_string()));
    assert_eq!(engine.last_error_code(), None);
}

#[test]
fn test_embed_eval_string_error_contains_message() {
    let mut engine = TxtcodeEngine::new();
    let result = engine.eval_string("undefined_var_xyz");
    assert!(result.is_err(), "eval of undefined var should fail");
    let msg = result.unwrap_err();
    assert!(!msg.is_empty(), "error message should not be empty");
    // last_error_code should be set
    assert!(engine.last_error_code().is_some(), "last_error_code should be Some after error");
}

#[test]
fn test_embed_last_error_code_cleared_on_success() {
    let mut engine = TxtcodeEngine::new();
    // First cause an error
    let _ = engine.eval("undefined_xyz");
    assert!(engine.last_error_code().is_some());
    // Then succeed — code should be cleared
    let _ = engine.eval("1 + 1");
    assert_eq!(engine.last_error_code(), None);
}

// ── Task I.2: txtcode_set_string_n ────────────────────────────────────────────

#[test]
fn test_embed_set_string_n_via_set() {
    // Test the Rust-level behavior: set a string with embedded content
    let mut engine = TxtcodeEngine::new();
    engine.set("msg", Value::String(Arc::from("hello\nworld")));
    let val = engine.get("msg");
    assert_eq!(val, Some(Value::String(Arc::from("hello\nworld"))));
}
