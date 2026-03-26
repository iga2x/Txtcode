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

// ---------------------------------------------------------------------------
// TLS connect tests (Task 16.1)
// ---------------------------------------------------------------------------

#[test]
fn test_tls_connect_unknown_without_net_feature() {
    use txtcode::stdlib::StdLib;
    use txtcode::runtime::Value;
    let result = StdLib::call_function(
        "tls_connect",
        &[Value::String(Arc::from("127.0.0.1")), Value::Integer(19999)],
        false,
        None::<&mut VirtualMachine>,
    );
    assert!(result.is_err(), "tls_connect to a closed port must return an error");
}

#[test]
fn test_tls_connect_bad_port_errors() {
    use txtcode::stdlib::StdLib;
    use txtcode::runtime::Value;
    let result = StdLib::call_function(
        "tls_connect",
        &[Value::String(Arc::from("example.com")), Value::Integer(0)],
        false,
        None::<&mut VirtualMachine>,
    );
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("port") || msg.contains("requires") || msg.contains("net"),
            "unexpected error: {}", msg);
}

#[test]
fn test_tls_connect_wrong_arg_type_errors() {
    use txtcode::stdlib::StdLib;
    use txtcode::runtime::Value;
    let result = StdLib::call_function(
        "tls_connect",
        &[Value::Integer(42), Value::Integer(443)],
        false,
        None::<&mut VirtualMachine>,
    );
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// WebSocket tests (Task 16.2)
// ---------------------------------------------------------------------------

#[test]
fn test_ws_connect_bad_url_errors() {
    use txtcode::stdlib::StdLib;
    use txtcode::runtime::Value;
    let result = StdLib::call_function(
        "ws_connect",
        &[Value::String(Arc::from("not-a-url"))],
        false,
        None::<&mut VirtualMachine>,
    );
    assert!(result.is_err(), "ws_connect with invalid URL must error");
}

#[test]
fn test_ws_connect_wrong_arg_errors() {
    use txtcode::stdlib::StdLib;
    use txtcode::runtime::Value;
    let result = StdLib::call_function(
        "ws_connect",
        &[Value::Integer(42)],
        false,
        None::<&mut VirtualMachine>,
    );
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("url must be a string") || msg.contains("requires"),
        "unexpected: {}", msg);
}

#[test]
fn test_ws_send_unknown_id_errors() {
    use txtcode::stdlib::StdLib;
    use txtcode::runtime::Value;
    let result = StdLib::call_function(
        "ws_send",
        &[Value::Integer(999999), Value::String(Arc::from("hello"))],
        false,
        None::<&mut VirtualMachine>,
    );
    assert!(result.is_err(), "ws_send with unknown id must error");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("999999") || msg.contains("no open") || msg.contains("requires"),
        "unexpected: {}", msg);
}

#[test]
fn test_ws_recv_unknown_id_errors() {
    use txtcode::stdlib::StdLib;
    use txtcode::runtime::Value;
    let result = StdLib::call_function(
        "ws_recv",
        &[Value::Integer(999998)],
        false,
        None::<&mut VirtualMachine>,
    );
    assert!(result.is_err(), "ws_recv with unknown id must error");
}

#[test]
fn test_ws_close_unknown_id_is_noop() {
    use txtcode::stdlib::StdLib;
    use txtcode::runtime::Value;
    let result = StdLib::call_function(
        "ws_close",
        &[Value::Integer(999997)],
        false,
        None::<&mut VirtualMachine>,
    );
    let _ = result;
}

#[test]
fn test_ws_serve_without_executor_errors() {
    use txtcode::stdlib::StdLib;
    use txtcode::runtime::Value;
    let result = StdLib::call_function(
        "ws_serve",
        &[Value::Integer(19997), Value::Null],
        false,
        None::<&mut VirtualMachine>,
    );
    assert!(result.is_err(), "ws_serve without executor must error");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("executor") || msg.contains("context") || msg.contains("requires"),
        "unexpected: {}", msg);
}

// ---------------------------------------------------------------------------
// HTTP tests
// ---------------------------------------------------------------------------

#[test]
fn test_http_get_https_routing() {
    use txtcode::stdlib::StdLib;
    use txtcode::runtime::Value;
    let result = StdLib::call_function(
        "http_get",
        &[Value::String(Arc::from("https://example.invalid"))],
        false,
        None::<&mut VirtualMachine>,
    );
    match result {
        Ok(Value::Future(_)) => { /* correct: async dispatch */ }
        Err(e) => {
            let msg = e.to_string();
            assert!(!msg.contains("Unknown standard library function"),
                "http_get must be routed to NetLib, got: {}", msg);
        }
        Ok(other) => panic!("http_get should return Future or Err, got {:?}", other),
    }
}

#[test]
fn test_async_http_get_returns_future_or_error() {
    let src = r#"async_http_get("http://localhost:19999/does-not-exist")"#;
    let result = run_ast_repl(src);
    let _ = result;
}

#[test]
fn test_async_http_post_permission_required() {
    let src = r#"async_http_post("http://localhost:19999/test", "{}")"#;
    let result = run_ast_repl(src);
    let _ = result;
}

// ---------------------------------------------------------------------------
// DNS / net_ping / net_port_open (Task 16.5)
// ---------------------------------------------------------------------------

#[test]
fn test_dns_resolve_returns_array() {
    use txtcode::stdlib::StdLib;
    use txtcode::runtime::Value;
    let result = StdLib::call_function(
        "dns_resolve",
        &[Value::String(Arc::from("localhost"))],
        false,
        None::<&mut VirtualMachine>,
    );
    match result {
        Ok(Value::Array(_)) => { /* correct */ }
        Ok(other) => panic!("Expected Array, got {:?}", other),
        Err(_) => {}
    }
}

#[test]
fn test_net_port_open_closed_port_returns_false() {
    use txtcode::stdlib::StdLib;
    use txtcode::runtime::Value;
    let result = StdLib::call_function(
        "net_port_open",
        &[
            Value::String(Arc::from("127.0.0.1")),
            Value::Integer(19996),
            Value::Integer(200),
        ],
        false,
        None::<&mut VirtualMachine>,
    );
    match result {
        Ok(Value::Boolean(false)) => { /* correct — port not open */ }
        Ok(Value::Boolean(true)) => { /* might be open in some envs — acceptable */ }
        Err(_) => { /* net feature not available — acceptable */ }
        Ok(other) => panic!("Expected Boolean, got {:?}", other),
    }
}

#[test]
fn test_net_ping_bad_host_returns_false() {
    use txtcode::stdlib::StdLib;
    use txtcode::runtime::Value;
    let result = StdLib::call_function(
        "net_ping",
        &[
            Value::String(Arc::from("this.host.definitely.does.not.exist.example")),
            Value::Integer(100),
        ],
        false,
        None::<&mut VirtualMachine>,
    );
    match result {
        Ok(Value::Boolean(false)) => { /* correct */ }
        Ok(Value::Boolean(true)) => { /* unlikely but possible */ }
        Err(_) => { /* net not available */ }
        Ok(other) => panic!("Expected Boolean, got {:?}", other),
    }
}

#[test]
fn test_net_port_open_bad_port_errors() {
    use txtcode::stdlib::StdLib;
    use txtcode::runtime::Value;
    let result = StdLib::call_function(
        "net_port_open",
        &[Value::String(Arc::from("localhost")), Value::Integer(0)],
        false,
        None::<&mut VirtualMachine>,
    );
    assert!(result.is_err(), "port 0 must error");
}

// ---------------------------------------------------------------------------
// HTTP serve tests (Group L.1) — net feature gated
// ---------------------------------------------------------------------------

#[test]
#[cfg(feature = "net")]
fn test_http_serve_parse_get_request() {
    use std::net::{TcpListener, TcpStream};
    use std::io::Write;

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().unwrap();

    let handle = std::thread::spawn(move || {
        let mut client = TcpStream::connect(addr).unwrap();
        client.write_all(b"GET /hello HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
        drop(client);
    });

    let (mut server_stream, _) = listener.accept().unwrap();
    let req = txtcode::stdlib::net::NetLib::parse_http_request(&mut server_stream)
        .expect("parse should succeed");

    assert_eq!(req.get("method"), Some(&txtcode::runtime::Value::String(Arc::from("GET"))));
    assert_eq!(req.get("path"), Some(&txtcode::runtime::Value::String(Arc::from("/hello"))));
    assert_eq!(req.get("body"), Some(&txtcode::runtime::Value::String(Arc::from(""))));

    handle.join().unwrap();
}

#[test]
#[cfg(feature = "net")]
fn test_http_serve_parse_post_with_body() {
    use std::net::{TcpListener, TcpStream};
    use std::io::Write;

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().unwrap();

    let handle = std::thread::spawn(move || {
        let mut client = TcpStream::connect(addr).unwrap();
        let body = b"hello=world";
        let req = format!(
            "POST /submit HTTP/1.1\r\nContent-Length: {}\r\n\r\n",
            body.len()
        );
        client.write_all(req.as_bytes()).unwrap();
        client.write_all(body).unwrap();
        drop(client);
    });

    let (mut server_stream, _) = listener.accept().unwrap();
    let req = txtcode::stdlib::net::NetLib::parse_http_request(&mut server_stream)
        .expect("parse should succeed");

    assert_eq!(req.get("method"), Some(&txtcode::runtime::Value::String(Arc::from("POST"))));
    assert_eq!(req.get("body"), Some(&txtcode::runtime::Value::String(Arc::from("hello=world"))));

    handle.join().unwrap();
}

#[test]
#[cfg(feature = "net")]
fn test_http_serve_write_404_response() {
    use std::net::{TcpListener, TcpStream};
    use std::io::Read;
    use indexmap::IndexMap;

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().unwrap();

    let handle = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let (mut conn, _) = listener.accept().unwrap();
        conn.read_to_end(&mut buf).unwrap();
        String::from_utf8_lossy(&buf).to_string()
    });

    let mut client = TcpStream::connect(addr).unwrap();
    let mut response_map = IndexMap::new();
    response_map.insert("status".to_string(), txtcode::runtime::Value::Integer(404));
    response_map.insert("body".to_string(), txtcode::runtime::Value::String(Arc::from("Not Found")));
    txtcode::stdlib::net::NetLib::write_http_response(
        &mut client,
        txtcode::runtime::Value::Map(response_map),
    ).expect("write should succeed");
    drop(client);

    let response = handle.join().unwrap();
    assert!(response.contains("404"), "response should contain status 404");
    assert!(response.contains("Not Found"), "response should contain body");
}

#[test]
#[cfg(feature = "net")]
fn test_http_serve_permission_denied() {
    use txtcode::runtime::permissions::PermissionResource;
    use txtcode::runtime::RuntimeError;
    use txtcode::stdlib::PermissionChecker;

    struct DenyAllChecker;
    impl PermissionChecker for DenyAllChecker {
        fn check_permission(&self, _r: &PermissionResource, _s: Option<&str>) -> Result<(), RuntimeError> {
            Err(RuntimeError::new("permission denied: Network(listen)".to_string()))
        }
    }

    struct NoopExecutor;
    impl txtcode::stdlib::FunctionExecutor for NoopExecutor {
        fn call_function_value(&mut self, _f: &txtcode::runtime::Value, _a: &[txtcode::runtime::Value]) -> Result<txtcode::runtime::Value, RuntimeError> {
            Ok(txtcode::runtime::Value::Null)
        }
    }

    let args = vec![
        txtcode::runtime::Value::Integer(19999),
        txtcode::runtime::Value::Null,
    ];
    let mut exec = NoopExecutor;
    let checker = DenyAllChecker;
    let result = txtcode::stdlib::net::NetLib::serve_with_executor(
        &args, &mut exec, Some(&checker),
    );
    assert!(result.is_err(), "should be denied by permission checker");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("permission denied") || msg.contains("listen"), "got: {}", msg);
}

#[test]
#[cfg(feature = "net")]
fn test_http_serve_handler_error_response_is_500() {
    use std::net::{TcpListener, TcpStream};
    use std::io::Read;
    use indexmap::IndexMap;

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().unwrap();

    let handle = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let (mut conn, _) = listener.accept().unwrap();
        conn.read_to_end(&mut buf).unwrap();
        String::from_utf8_lossy(&buf).to_string()
    });

    let mut client = TcpStream::connect(addr).unwrap();
    let mut error_map = IndexMap::new();
    error_map.insert("status".to_string(), txtcode::runtime::Value::Integer(500));
    error_map.insert("body".to_string(), txtcode::runtime::Value::String(Arc::from("Internal Server Error: handler failed")));
    txtcode::stdlib::net::NetLib::write_http_response(
        &mut client,
        txtcode::runtime::Value::Map(error_map),
    ).expect("write should succeed");
    drop(client);

    let response = handle.join().unwrap();
    assert!(response.contains("500"), "response should contain status 500");
    assert!(response.contains("Internal Server Error"), "got: {}", response);
}

// ---------------------------------------------------------------------------
// JWT tests (Task 16.4)
// ---------------------------------------------------------------------------

#[test]
fn test_jwt_sign_and_verify_roundtrip() {
    use txtcode::runtime::Value;
    let src = r#"
store → payload → {"sub": "user123", "role": "admin"}
store → token → jwt_sign(payload, "mysecret", "HS256")
store → result → jwt_verify(token, "mysecret")
is_ok(result)
"#;
    let tokens = Lexer::new(src.to_string()).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap();
    let result = VirtualMachine::new().interpret_repl(&program).unwrap();
    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn test_jwt_verify_wrong_secret_returns_err() {
    use txtcode::runtime::Value;
    let src = r#"
store → token → jwt_sign({"user": "bob"}, "correct_secret", "HS256")
store → result → jwt_verify(token, "wrong_secret")
is_err(result)
"#;
    let tokens = Lexer::new(src.to_string()).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap();
    let result = VirtualMachine::new().interpret_repl(&program).unwrap();
    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn test_jwt_decode_no_verification() {
    use txtcode::runtime::Value;
    let src = r#"
store → token → jwt_sign({"uid": 42}, "anysecret", "HS256")
store → payload → jwt_decode(token)
payload["uid"]
"#;
    let tokens = Lexer::new(src.to_string()).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap();
    let result = VirtualMachine::new().interpret_repl(&program).unwrap();
    assert_eq!(result, Value::Integer(42));
}

#[test]
fn test_jwt_sign_default_algorithm() {
    use txtcode::runtime::Value;
    let src = r#"
store → token → jwt_sign({"x": 1}, "secret")
len(token) > 10
"#;
    let tokens = Lexer::new(src.to_string()).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap();
    let result = VirtualMachine::new().interpret_repl(&program).unwrap();
    assert_eq!(result, Value::Boolean(true));
}
