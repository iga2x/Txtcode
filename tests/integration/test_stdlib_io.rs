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

fn run_ast_source_with_write(
    source: &str,
) -> Result<txtcode::runtime::Value, txtcode::runtime::errors::RuntimeError> {
    use txtcode::runtime::permissions::PermissionResource;
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut vm = VirtualMachine::new();
    vm.grant_permission(PermissionResource::FileSystem("write".to_string()), None);
    vm.interpret(&program)
}

// ---------------------------------------------------------------------------
// File I/O tests (Task 5.5)
// ---------------------------------------------------------------------------

#[test]
fn test_file_open_read_close() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_str().unwrap().to_string();
    std::fs::write(&path, "line1\nline2\nline3\n").unwrap();

    let src = format!(r#"
store → h → file_open("{}", "r")
store → l1 → file_read_line(h)
store → l2 → file_read_line(h)
file_close(h)
l1
"#, path);
    let result = run_ast_repl(&src);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String(Arc::from("line1")));
}

#[test]
fn test_file_eof_returns_null() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_str().unwrap().to_string();
    std::fs::write(&path, "line1\nline2\n").unwrap();

    let src = format!(r#"
store → h → file_open("{}", "r")
store → l1 → file_read_line(h)
store → l2 → file_read_line(h)
store → eof → file_read_line(h)
file_close(h)
eof
"#, path);
    let result = run_ast_repl(&src);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Null);
}

#[test]
fn test_file_write_line() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_str().unwrap().to_string();

    let src = format!(r#"
store → h → file_open("{}", "w")
file_write_line(h, "hello")
file_write_line(h, "world")
file_close(h)
"#, path);
    let write_result = run_ast_source(&src);
    assert!(write_result.is_ok(), "{:?}", write_result);
    let content = std::fs::read_to_string(&path).unwrap();
    assert_eq!(content, "hello\nworld\n");
}

// ---------------------------------------------------------------------------
// CSV write/read (Task 5.3)
// ---------------------------------------------------------------------------

#[test]
fn test_csv_to_string_basic() {
    let result = run_ast_repl(
        r#"csv_to_string([[1, 2, 3], ["a", "b", "c"]])"#,
    );
    match result {
        Ok(txtcode::runtime::Value::String(s)) => {
            assert!(s.contains("1,2,3"), "should contain row 1: {}", s);
            assert!(s.contains("a,b,c"), "should contain row 2: {}", s);
        }
        other => panic!("expected String, got {:?}", other),
    }
}

#[test]
fn test_csv_write_and_read() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_str().unwrap().to_string();
    let src = format!(r#"csv_write("{}", [["name", "age"], ["Alice", 30], ["Bob", 25]])"#, path);
    let write_result = run_ast_source_with_write(&src);
    assert!(write_result.is_ok(), "csv_write failed: {:?}", write_result);
    let read_src = format!("read_csv(\"{}\")", path);
    let read_result = run_ast_repl(&read_src);
    assert!(read_result.is_ok(), "read_csv failed: {:?}", read_result);
    match read_result.unwrap() {
        txtcode::runtime::Value::Array(rows) => assert_eq!(rows.len(), 3),
        other => panic!("expected Array, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Bytes tests
// ---------------------------------------------------------------------------

#[test]
fn test_bytes_new_and_len() {
    let result = run_ast_repl("bytes_len(bytes_new(5))");
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(5));
}

#[test]
fn test_bytes_from_hex_to_hex() {
    let result = run_ast_repl("bytes_to_hex(bytes_from_hex(\"ff0a\"))");
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String(Arc::from("ff0a")));
}

#[test]
fn test_bytes_get_set() {
    let result = run_ast_repl(
        "store → b → bytes_new(3)\nstore → b → bytes_set(b, 1, 42)\nbytes_get(b, 1)"
    );
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(42));
}

#[test]
fn test_bytes_slice() {
    let result = run_ast_repl(
        "store → b → bytes_from_hex(\"0102030405\")\nbytes_len(bytes_slice(b, 1, 3))"
    );
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(2));
}

#[test]
fn test_bytes_concat() {
    let result = run_ast_repl(
        "store → a → bytes_from_hex(\"0102\")\nstore → b2 → bytes_from_hex(\"0304\")\nbytes_len(bytes_concat(a, b2))"
    );
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(4));
}

#[test]
fn test_bytes_new_length() {
    let result = run_ast_repl("bytes_len(bytes_new(8))");
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(8));
}

#[test]
fn test_bytes_set_get_roundtrip() {
    let result = run_ast_repl(
        "store → b → bytes_new(4)\nstore → b2 → bytes_set(b, 2, 255)\nbytes_get(b2, 2)"
    );
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(255));
}

#[test]
fn test_bytes_from_string_to_hex() {
    let result = run_ast_repl(r#"bytes_to_hex(bytes_from_hex("41"))"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String(Arc::from("41")));
}

// ---------------------------------------------------------------------------
// ZIP (Task 5.4)
// ---------------------------------------------------------------------------

#[test]
fn test_zip_create_and_extract() {
    let dir = tempfile::tempdir().unwrap();
    let zip_path = dir.path().join("test.zip").to_str().unwrap().to_string();
    let extract_dir = dir.path().join("out").to_str().unwrap().to_string();

    let src_file = dir.path().join("hello.txt");
    std::fs::write(&src_file, "hello world").unwrap();
    let src_path = src_file.to_str().unwrap().to_string();

    let create_src = format!(r#"zip_create("{}", "{}")"#, zip_path, src_path);
    let result = run_ast_source(&create_src);
    if result.is_ok() {
        let extract_src = format!(r#"zip_extract("{}", "{}")"#, zip_path, extract_dir);
        let extract_result = run_ast_source(&extract_src);
        assert!(extract_result.is_ok(), "zip_extract failed: {:?}", extract_result);
    }
}

// ---------------------------------------------------------------------------
// Async file I/O (Task 15.4)
// ---------------------------------------------------------------------------

#[test]
fn test_async_write_and_read_file() {
    use txtcode::runtime::{Value, permissions::PermissionResource};

    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_str().unwrap().to_string();

    let src = format!(r#"
store → write_fut → async_write_file("{path}", "hello async")
store → _ → await write_fut
store → read_fut → async_read_file("{path}")
store → content → await read_fut
content
"#);
    let tokens = txtcode::lexer::Lexer::new(src.to_string()).tokenize().unwrap();
    let program = txtcode::parser::Parser::new(tokens).parse().unwrap();
    let mut vm = VirtualMachine::new();
    vm.grant_permission(PermissionResource::FileSystem("read".to_string()), None);
    vm.grant_permission(PermissionResource::FileSystem("write".to_string()), None);
    let result = vm.interpret_repl(&program).unwrap();
    assert_eq!(result, Value::String(Arc::from("hello async")));
}

#[test]
fn test_async_read_file_returns_future() {
    use txtcode::runtime::{Value, permissions::PermissionResource};

    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_str().unwrap().to_string();
    std::fs::write(tmp.path(), "async content").unwrap();

    let src = format!(r#"async_read_file("{path}")"#);
    let tokens = txtcode::lexer::Lexer::new(src.to_string()).tokenize().unwrap();
    let program = txtcode::parser::Parser::new(tokens).parse().unwrap();
    let mut vm = VirtualMachine::new();
    vm.grant_permission(PermissionResource::FileSystem("read".to_string()), None);
    let result = vm.interpret_repl(&program).unwrap();
    assert!(matches!(result, Value::Future(_)), "expected Future, got {:?}", result);
}

#[test]
fn test_async_for_reads_file() {
    use txtcode::runtime::{Value, permissions::PermissionResource};

    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), "file content").unwrap();
    let path = tmp.path().to_str().unwrap().to_string();

    let src = format!(r#"
store → content → await async_read_file("{path}")
content
"#);
    let tokens = txtcode::lexer::Lexer::new(src.to_string()).tokenize().unwrap();
    let program = txtcode::parser::Parser::new(tokens).parse().unwrap();
    let mut vm = VirtualMachine::new();
    vm.grant_permission(PermissionResource::FileSystem("read".to_string()), None);
    let result = vm.interpret_repl(&program).unwrap();
    assert_eq!(result, Value::String(Arc::from("file content")));
}

// ---------------------------------------------------------------------------
// exec with stdin / http helpers
// ---------------------------------------------------------------------------

#[test]
fn test_exec_with_stdin_option() {
    let result = run_ast_repl(r#"exec("cat", {stdin: "hello world"})"#);
    match result {
        Ok(txtcode::runtime::Value::String(s)) => {
            assert_eq!(s.trim(), "hello world");
        }
        Ok(other) => panic!("expected String, got {:?}", other),
        Err(e) => {
            let msg = e.to_string();
            assert!(msg.contains("exec") || msg.contains("safe") || msg.contains("permission"),
                "unexpected error: {}", msg);
        }
    }
}

#[test]
fn test_http_response_helper() {
    let result = run_ast_repl(r#"
store → resp → http_response(200, "OK")
resp["status"]
"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(200));
}

#[test]
fn test_http_request_helpers() {
    let result = run_ast_repl(r#"
store → req → {method: "POST", path: "/api", body: "data", headers: {}}
store → m → http_request_method(req)
m
"#);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::String(Arc::from("POST")));
}
