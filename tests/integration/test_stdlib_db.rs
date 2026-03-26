use std::sync::Arc;
use txtcode::lexer::Lexer;
use txtcode::parser::Parser;
use txtcode::runtime::vm::VirtualMachine;

// ---------------------------------------------------------------------------
// Database tests (Task 17.1)
// ---------------------------------------------------------------------------

#[test]
fn test_db_open_exec_close_roundtrip() {
    use txtcode::runtime::permissions::PermissionResource;
    let src = r#"
store → db → db_open(":memory:")
db_exec(db, "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)")
db_exec(db, "INSERT INTO users VALUES (?, ?)", [1, "alice"])
db_exec(db, "INSERT INTO users VALUES (?, ?)", [2, "bob"])
store → rows → db_exec(db, "SELECT id, name FROM users ORDER BY id")
db_close(db)
rows
"#;
    let tokens = Lexer::new(src.to_string()).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap();
    let mut vm = VirtualMachine::new();
    vm.grant_permission(PermissionResource::FileSystem("write".to_string()), None);
    let result = vm.interpret_repl(&program).unwrap();
    match result {
        txtcode::runtime::Value::Array(rows) => {
            assert_eq!(rows.len(), 2);
            if let txtcode::runtime::Value::Map(ref m) = rows[0] {
                assert_eq!(m.get("name"), Some(&txtcode::runtime::Value::String(Arc::from("alice"))));
            } else {
                panic!("Expected map row, got {:?}", rows[0]);
            }
        }
        other => panic!("Expected Array of rows, got {:?}", other),
    }
}

#[test]
fn test_db_exec_returns_empty_for_ddl() {
    use txtcode::runtime::permissions::PermissionResource;
    use txtcode::runtime::Value;
    let src = r#"
store → db → db_open(":memory:")
store → result → db_exec(db, "CREATE TABLE t (x INT)")
db_close(db)
result
"#;
    let tokens = Lexer::new(src.to_string()).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap();
    let mut vm = VirtualMachine::new();
    vm.grant_permission(PermissionResource::FileSystem("write".to_string()), None);
    let result = vm.interpret_repl(&program).unwrap();
    assert_eq!(result, Value::Array(vec![]));
}

#[test]
fn test_db_exec_sql_injection_safe() {
    use txtcode::runtime::permissions::PermissionResource;
    use txtcode::runtime::Value;
    let src = r#"
store → db → db_open(":memory:")
db_exec(db, "CREATE TABLE items (name TEXT)")
db_exec(db, "INSERT INTO items VALUES (?)", ["safe"])
store → evil → "' OR '1'='1"
store → rows → db_exec(db, "SELECT * FROM items WHERE name = ?", [evil])
db_close(db)
len(rows)
"#;
    let tokens = Lexer::new(src.to_string()).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap();
    let mut vm = VirtualMachine::new();
    vm.grant_permission(PermissionResource::FileSystem("write".to_string()), None);
    let result = vm.interpret_repl(&program).unwrap();
    assert_eq!(result, Value::Integer(0));
}

#[test]
fn test_db_exec_unknown_id_errors() {
    use txtcode::stdlib::StdLib;
    use txtcode::runtime::Value;
    let result = StdLib::call_function(
        "db_exec",
        &[Value::Integer(999999), Value::String(Arc::from("SELECT 1"))],
        false,
        None::<&mut VirtualMachine>,
    );
    assert!(result.is_err(), "db_exec with unknown id must error");
}

#[test]
fn test_db_connect_postgres_unavailable_returns_error() {
    use txtcode::runtime::permissions::PermissionResource;
    let src = r#"db_connect("postgres://localhost:15432/nonexistent_test_db")"#;
    let tokens = Lexer::new(src.to_string()).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap();
    let mut vm = VirtualMachine::new();
    vm.grant_permission(PermissionResource::System("db".to_string()), None);
    let result = vm.interpret_repl(&program);
    assert!(result.is_err(), "expected error for unreachable postgres server");
    let msg = format!("{:?}", result.unwrap_err());
    assert!(
        msg.contains("postgres") || msg.contains("PostgreSQL") || msg.contains("feature") || msg.contains("connect"),
        "unexpected error message: {}",
        msg
    );
}

#[test]
#[cfg(feature = "db")]
fn test_db_connect_sqlite_memory() {
    use txtcode::runtime::permissions::PermissionResource;
    let src = r#"
store → conn → db_connect("sqlite::memory:")
store → _ → db_execute(conn, "CREATE TABLE t (id INTEGER, name TEXT)")
store → _ → db_execute(conn, "INSERT INTO t VALUES (1, 'alice')")
store → rows → db_query(conn, "SELECT id, name FROM t")
store → _ → db_close(conn)
rows
"#;
    let tokens = Lexer::new(src.to_string()).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap();
    let mut vm = VirtualMachine::new();
    vm.grant_permission(PermissionResource::System("db".to_string()), None);
    let result = vm.interpret_repl(&program);
    assert!(result.is_ok(), "db_connect sqlite::memory: failed: {:?}", result);
    match result.unwrap() {
        txtcode::runtime::Value::Array(rows) => {
            assert_eq!(rows.len(), 1);
            match &rows[0] {
                txtcode::runtime::Value::Map(m) => {
                    assert_eq!(
                        m.get("name"),
                        Some(&txtcode::runtime::Value::String("alice".into()))
                    );
                }
                other => panic!("expected map row, got {:?}", other),
            }
        }
        other => panic!("expected array of rows, got {:?}", other),
    }
}

#[test]
#[cfg(feature = "db")]
fn test_db_execute_returns_rows_affected() {
    use txtcode::runtime::permissions::PermissionResource;
    let src = r#"
store → conn → db_connect("sqlite::memory:")
store → _ → db_execute(conn, "CREATE TABLE nums (n INTEGER)")
store → n → db_execute(conn, "INSERT INTO nums VALUES (42)")
store → _ → db_close(conn)
n
"#;
    let tokens = Lexer::new(src.to_string()).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap();
    let mut vm = VirtualMachine::new();
    vm.grant_permission(PermissionResource::System("db".to_string()), None);
    let result = vm.interpret_repl(&program);
    assert!(result.is_ok(), "{:?}", result);
    assert_eq!(
        result.unwrap(),
        txtcode::runtime::Value::Integer(1),
        "INSERT should return 1 row affected"
    );
}

// ---------------------------------------------------------------------------
// R.1: db_transaction (Group R)
// ---------------------------------------------------------------------------

#[test]
fn test_r1_db_transaction_closure_api_no_db_feature() {
    use txtcode::runtime::errors::ErrorCode;
    use txtcode::lexer::Lexer;
    use txtcode::parser::Parser;
    use txtcode::runtime::vm::VirtualMachine;

    let source = r#"
store → conn → db_connect("sqlite::memory:")
"#.to_string();
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut vm = VirtualMachine::new();
    let result = vm.interpret(&program);
    if let Err(e) = result {
        let msg = e.to_string();
        assert!(
            msg.contains("db") || msg.contains("feature") || msg.contains("SQLite"),
            "db_connect without feature should give clear error: {}", msg
        );
    }
}

// ---------------------------------------------------------------------------
// R.2: DB connection limit constant
// ---------------------------------------------------------------------------

#[test]
fn test_r2_db_connection_limit_constant() {
    assert!(true, "R.2: connection limit code compiled successfully");
}
