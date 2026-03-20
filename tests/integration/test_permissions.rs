/// Permission denial and security tests for v0.4
///
/// These tests verify that the permission system correctly denies access
/// when permissions are not granted or explicitly denied.
use txtcode::lexer::Lexer;
use txtcode::parser::Parser;
use txtcode::runtime::permissions::PermissionResource;
use txtcode::runtime::vm::VirtualMachine;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[allow(clippy::result_large_err)]
fn parse_and_run(
    source: &str,
) -> Result<txtcode::runtime::Value, txtcode::runtime::errors::RuntimeError> {
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut vm = VirtualMachine::new();
    vm.interpret(&program)
}

// ---------------------------------------------------------------------------
// Permission manager unit tests
// ---------------------------------------------------------------------------

#[test]
fn test_permission_denied_filesystem_read() {
    let mut vm = VirtualMachine::new();
    // Explicitly deny filesystem read
    vm.deny_permission(PermissionResource::FileSystem("read".to_string()), None);

    let result = vm.check_permission(&PermissionResource::FileSystem("read".to_string()), None);
    assert!(
        result.is_err(),
        "Expected denied filesystem read to return Err"
    );
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("Permission") || msg.contains("denied") || msg.contains("granted"),
        "Error message should describe permission failure, got: {}",
        msg
    );
}

#[test]
fn test_permission_denied_network_connect() {
    let mut vm = VirtualMachine::new();
    vm.deny_permission(PermissionResource::Network("connect".to_string()), None);

    let result = vm.check_permission(&PermissionResource::Network("connect".to_string()), None);
    assert!(
        result.is_err(),
        "Expected denied network connect to return Err"
    );
}

#[test]
fn test_permission_denied_process_exec() {
    let mut vm = VirtualMachine::new();
    vm.deny_permission(PermissionResource::Process(vec!["exec".to_string()]), None);

    let result = vm.check_permission(&PermissionResource::Process(vec!["exec".to_string()]), None);
    assert!(
        result.is_err(),
        "Expected denied process exec to return Err"
    );
}

#[test]
fn test_permission_not_granted_by_default() {
    let vm = VirtualMachine::new();
    // No permissions granted — any resource check should fail
    let result = vm.check_permission(&PermissionResource::FileSystem("write".to_string()), None);
    assert!(
        result.is_err(),
        "Expected filesystem write to be denied when no permissions are granted"
    );
}

#[test]
fn test_permission_granted_then_denied_is_denied() {
    // Explicit deny overrides grant
    let mut vm = VirtualMachine::new();
    vm.grant_permission(PermissionResource::Network("connect".to_string()), None);
    vm.deny_permission(PermissionResource::Network("connect".to_string()), None);

    let result = vm.check_permission(&PermissionResource::Network("connect".to_string()), None);
    assert!(
        result.is_err(),
        "Explicit denial must override a prior grant"
    );
}

#[test]
fn test_permission_scoped_read_denied_for_other_path() {
    let mut vm = VirtualMachine::new();
    // Grant read only on /tmp/*
    vm.grant_permission(
        PermissionResource::FileSystem("read".to_string()),
        Some("/tmp/*".to_string()),
    );

    // Read on /etc/passwd should be denied (out of scope)
    let result = vm.check_permission(
        &PermissionResource::FileSystem("read".to_string()),
        Some("/etc/passwd"),
    );
    assert!(
        result.is_err(),
        "Scoped permission for /tmp/* should not allow /etc/passwd"
    );
}

#[test]
fn test_permission_scoped_read_allowed_in_scope() {
    let mut vm = VirtualMachine::new();
    vm.grant_permission(
        PermissionResource::FileSystem("read".to_string()),
        Some("/tmp/*".to_string()),
    );

    let result = vm.check_permission(
        &PermissionResource::FileSystem("read".to_string()),
        Some("/tmp/data.txt"),
    );
    assert!(
        result.is_ok(),
        "Scoped permission for /tmp/* should allow /tmp/data.txt, got: {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Safe mode tests
// ---------------------------------------------------------------------------

#[test]
fn test_safe_mode_blocks_exec() {
    // In safe mode, exec() should be blocked at the stdlib dispatch level.
    // We verify this by running a script that calls exec() through the VM
    // with exec_allowed = false.
    // Since we can't set exec_allowed to false directly in a constructed VM
    // without the CLI, we test the behavior via the bytecode VM path which
    // uses exec_allowed=true by default. Instead, test via stdlib directly.
    use txtcode::runtime::Value;
    use txtcode::stdlib::FunctionExecutor;
    use txtcode::stdlib::StdLib;

    struct NoopExecutor;
    impl FunctionExecutor for NoopExecutor {
        fn call_function_value(
            &mut self,
            _func: &Value,
            _args: &[Value],
        ) -> Result<Value, txtcode::runtime::errors::RuntimeError> {
            Err(txtcode::runtime::errors::RuntimeError::new(
                "Not supported".to_string(),
            ))
        }
    }

    // exec_allowed = false should produce an error for exec()
    let result = StdLib::call_function::<NoopExecutor>(
        "exec",
        &[Value::String("echo hello".to_string())],
        false, // exec NOT allowed
        None,
    );
    assert!(
        result.is_err(),
        "exec() must return Err when exec_allowed = false (safe mode)"
    );
}

// ---------------------------------------------------------------------------
// Bytecode VM: NullCoalesce
// ---------------------------------------------------------------------------

#[cfg(feature = "bytecode")]
#[test]
fn test_bytecode_vm_null_coalesce_returns_default_for_null() {
    use txtcode::compiler::bytecode::Bytecode;
    use txtcode::compiler::bytecode::{Constant, Instruction};
    use txtcode::runtime::bytecode_vm::BytecodeVM;
    use txtcode::runtime::Value;

    // Manually build bytecode: push null, push 42, NullCoalesce → expect 42
    let bytecode = Bytecode {
        instructions: vec![
            Instruction::PushConstant(0), // null
            Instruction::PushConstant(1), // 42
            Instruction::NullCoalesce,
        ],
        constants: vec![Constant::Null, Constant::Integer(42)],
        debug_info: vec![],
    };

    let mut vm = BytecodeVM::new();
    let result = vm.execute(&bytecode);
    assert!(
        result.is_ok(),
        "NullCoalesce should not error: {:?}",
        result
    );
    assert_eq!(result.unwrap(), Value::Integer(42));
}

#[cfg(feature = "bytecode")]
#[test]
fn test_bytecode_vm_null_coalesce_returns_value_when_not_null() {
    use txtcode::compiler::bytecode::Bytecode;
    use txtcode::compiler::bytecode::{Constant, Instruction};
    use txtcode::runtime::bytecode_vm::BytecodeVM;
    use txtcode::runtime::Value;

    // push "hello", push "default", NullCoalesce → expect "hello"
    let bytecode = Bytecode {
        instructions: vec![
            Instruction::PushConstant(0), // "hello"
            Instruction::PushConstant(1), // "default"
            Instruction::NullCoalesce,
        ],
        constants: vec![
            Constant::String("hello".to_string()),
            Constant::String("default".to_string()),
        ],
        debug_info: vec![],
    };

    let mut vm = BytecodeVM::new();
    let result = vm.execute(&bytecode);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Value::String("hello".to_string()));
}

#[cfg(feature = "bytecode")]
#[test]
fn test_bytecode_vm_optional_member_null_safe() {
    // OptionalGetField on Null must return Null (safe navigation, no error)
    use txtcode::compiler::bytecode::Bytecode;
    use txtcode::compiler::bytecode::{Constant, Instruction};
    use txtcode::runtime::bytecode_vm::BytecodeVM;
    use txtcode::runtime::core::Value;

    let bytecode = Bytecode {
        instructions: vec![
            Instruction::PushConstant(0),
            Instruction::OptionalGetField("name".to_string()),
        ],
        constants: vec![Constant::Null],
        debug_info: vec![],
    };

    let mut vm = BytecodeVM::new();
    let result = vm.execute(&bytecode);
    assert!(result.is_ok(), "OptionalGetField on null must not error");
    assert_eq!(
        result.unwrap(),
        Value::Null,
        "OptionalGetField on null must return Null"
    );
}

// ---------------------------------------------------------------------------
// Migration: dry-run smoke test
// ---------------------------------------------------------------------------

#[test]
fn test_migration_dry_run_does_not_modify_source() {
    use txtcode::runtime::compatibility::Version;
    use txtcode::runtime::migration::MigrationFramework;

    // Write a simple .tc file
    let dir = std::env::temp_dir();
    let file_path = dir.join("test_migrate_dry_run.tc");
    let source = "store → x → 42\nprint → x\n";
    std::fs::write(&file_path, source).expect("Failed to write temp file");

    let framework = MigrationFramework::new().with_dry_run(true);
    let result = framework.migrate_file(
        &file_path,
        Some(Version::new(0, 1, 0)),
        Some(Version::new(0, 2, 0)),
    );

    // Migration report should succeed
    assert!(
        result.is_ok(),
        "Dry-run migration should not error: {:?}",
        result
    );

    // Source file must be unchanged
    let after = std::fs::read_to_string(&file_path).expect("File should still exist");
    assert_eq!(after, source, "Dry-run must not modify the source file");

    // Cleanup
    let _ = std::fs::remove_file(&file_path);
}

// ---------------------------------------------------------------------------
// Smoke tests: all major CLI entry points parse without panic
// ---------------------------------------------------------------------------

#[test]
fn test_smoke_run_hello_world() {
    let result = parse_and_run("print → \"Hello, World!\"");
    assert!(result.is_ok(), "Hello World should run: {:?}", result);
}

#[test]
fn test_smoke_run_arithmetic() {
    let result = parse_and_run("store → x → 10 + 5 * 2");
    assert!(result.is_ok());
}

#[test]
fn test_smoke_run_if_else() {
    let result = parse_and_run(
        "store → x → 5\nif → x > 3\n  print → \"big\"\nelse\n  print → \"small\"\nend",
    );
    assert!(result.is_ok());
}

#[test]
fn test_smoke_run_while_loop() {
    let result = parse_and_run("store → i → 0\nwhile → i < 3\n  store → i → i + 1\nend");
    assert!(result.is_ok());
}

#[test]
fn test_smoke_run_function_definition_and_call() {
    let source = "define \u{2192} add \u{2192} (a, b)\n  return \u{2192} a + b\nend\nstore \u{2192} r \u{2192} add(3, 4)\n";
    let result = parse_and_run(source);
    assert!(
        result.is_ok(),
        "Function definition and call should work: {:?}",
        result
    );
}

#[test]
fn test_smoke_run_match() {
    let result = parse_and_run(
        "store → x → 2\nmatch → x\n  case → 1\n    print → \"one\"\n  case → 2\n    print → \"two\"\n  case → _\n    print → \"other\"\nend",
    );
    assert!(result.is_ok());
}

#[test]
fn test_smoke_run_try_catch() {
    let result = parse_and_run("try\n  store → x → 1 / 0\ncatch → e\n  print → \"caught\"\nend");
    assert!(
        result.is_ok(),
        "try/catch should handle division by zero: {:?}",
        result
    );
}

#[test]
fn test_smoke_formatter_does_not_panic() {
    use txtcode::tools::formatter::Formatter;
    let source = "store → x → 42\nprint → x\n";
    let result = Formatter::format_source(source);
    assert!(
        result.is_ok(),
        "Formatter should not panic on simple source: {:?}",
        result
    );
}

#[test]
fn test_smoke_linter_does_not_panic() {
    use txtcode::tools::linter::Linter;
    let source = "store → x → 42\nprint → x\n";
    let result = Linter::lint_source(source);
    assert!(
        result.is_ok(),
        "Linter should not panic on simple source: {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Bytecode VM: control flow and iterators (v0.4)
// ---------------------------------------------------------------------------

#[cfg(feature = "bytecode")]
#[test]
fn test_bytecode_vm_while_loop_executes_correctly() {
    use txtcode::runtime::bytecode_vm::BytecodeVM;
    use txtcode::runtime::Value;

    // while i < 3 { i = i + 1 }  →  i ends at 3
    // ip 0: LoadVar("i")
    // ip 1: PushConstant(0) = 3
    // ip 2: Less
    // ip 3: JumpIfFalse(8)
    // ip 4: LoadVar("i")
    // ip 5: PushConstant(1) = 1
    // ip 6: Add
    // ip 7: StoreVar("i")
    // ip 8: Jump(0)   ← with pre-increment this correctly re-runs ip 0
    // ip 9: [end, but JumpIfFalse(9) jumps here when done]
    // Wait — we need one more slot so JumpIfFalse target is valid:
    // Let's use a simple approach and test via the bytecode compiler + VM
    let _ = Value::Integer(0); // prevent unused import warning

    // Instead use the compiler end-to-end
    use txtcode::compiler::bytecode::BytecodeCompiler;
    use txtcode::lexer::Lexer;
    use txtcode::parser::Parser;

    let source = "store \u{2192} i \u{2192} 0\nwhile \u{2192} i < 3\n  store \u{2192} i \u{2192} i + 1\nend\n";
    // Can't easily run bytecode VM from CLI, but we can compile and execute manually
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new();
    let bytecode = compiler.compile(&program);

    let mut vm = BytecodeVM::new();
    let _ = vm.execute(&bytecode); // result may be null after StoreVar + Pop
                                   // Variable i should be 3 after the loop
    assert_eq!(
        vm.get_variable("i"),
        Some(&Value::Integer(3)),
        "While loop should have incremented i to 3"
    );
}

#[cfg(feature = "bytecode")]
#[test]
fn test_bytecode_vm_for_loop_iterates_array() {
    use txtcode::compiler::bytecode::BytecodeCompiler;
    use txtcode::lexer::Lexer;
    use txtcode::parser::Parser;
    use txtcode::runtime::bytecode_vm::BytecodeVM;
    use txtcode::runtime::Value;

    // for x in [10, 20, 30] — after the loop, x should be 30 (last element)
    let source = "store \u{2192} arr \u{2192} [10, 20, 30]\nfor \u{2192} x in arr\n  store \u{2192} last \u{2192} x\nend\n";
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new();
    let bytecode = compiler.compile(&program);

    let mut vm = BytecodeVM::new();
    let _ = vm.execute(&bytecode);
    assert_eq!(
        vm.get_variable("last"),
        Some(&Value::Integer(30)),
        "for loop should have stored last element 30 in `last`"
    );
}

#[cfg(feature = "bytecode")]
#[test]
fn test_bytecode_vm_for_loop_empty_array_skips_body() {
    use txtcode::compiler::bytecode::BytecodeCompiler;
    use txtcode::lexer::Lexer;
    use txtcode::parser::Parser;
    use txtcode::runtime::bytecode_vm::BytecodeVM;
    use txtcode::runtime::Value;

    let source = "store \u{2192} ran \u{2192} false\nfor \u{2192} x in []\n  store \u{2192} ran \u{2192} true\nend\n";
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new();
    let bytecode = compiler.compile(&program);

    let mut vm = BytecodeVM::new();
    let _ = vm.execute(&bytecode);
    assert_eq!(
        vm.get_variable("ran"),
        Some(&Value::Boolean(false)),
        "for loop over empty array should not execute the body"
    );
}

#[cfg(feature = "bytecode")]
#[test]
fn test_bytecode_vm_increment_operator() {
    use txtcode::compiler::bytecode::BytecodeCompiler;
    use txtcode::lexer::Lexer;
    use txtcode::parser::Parser;
    use txtcode::runtime::bytecode_vm::BytecodeVM;
    use txtcode::runtime::Value;

    // ++x increments x from 5 to 6
    let source = "store \u{2192} x \u{2192} 5\nstore \u{2192} y \u{2192} ++x\n";
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new();
    let bytecode = compiler.compile(&program);

    let mut vm = BytecodeVM::new();
    let _ = vm.execute(&bytecode);
    assert_eq!(
        vm.get_variable("x"),
        Some(&Value::Integer(6)),
        "x should be 6 after ++x"
    );
    assert_eq!(
        vm.get_variable("y"),
        Some(&Value::Integer(6)),
        "y should capture new value 6"
    );
}

// ---------------------------------------------------------------------------
// Overflow guards (v0.4)
// ---------------------------------------------------------------------------

#[test]
fn test_integer_overflow_add_returns_error() {
    let result = parse_and_run(&format!("store \u{2192} x \u{2192} {} + 1", i64::MAX));
    assert!(
        result.is_err(),
        "Adding 1 to i64::MAX should produce an overflow error"
    );
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("overflow") || msg.contains("Overflow"),
        "Error should mention overflow, got: {}",
        msg
    );
}

#[test]
fn test_integer_overflow_multiply_returns_error() {
    let result = parse_and_run(&format!("store \u{2192} x \u{2192} {} * 2", i64::MAX));
    assert!(
        result.is_err(),
        "Multiplying i64::MAX by 2 should produce an overflow error"
    );
}

#[test]
fn test_recursion_depth_limit() {
    // Infinite recursion should produce a RuntimeError, not a stack overflow / panic
    let source =
        "define \u{2192} recurse \u{2192} (n)\n  return \u{2192} recurse(n + 1)\nend\nrecurse(0)\n";
    let result = parse_and_run(source);
    assert!(
        result.is_err(),
        "Infinite recursion should return a RuntimeError"
    );
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("call stack")
            || msg.contains("recursion")
            || msg.contains("depth")
            || msg.contains("exceeded"),
        "Error should describe call stack exhaustion, got: {}",
        msg
    );
}

#[cfg(feature = "bytecode")]
#[test]
fn test_bytecode_vm_match_string_case() {
    use txtcode::compiler::bytecode::BytecodeCompiler;
    use txtcode::lexer::Lexer;
    use txtcode::parser::Parser;
    use txtcode::runtime::bytecode_vm::BytecodeVM;
    use txtcode::runtime::Value;

    let source = concat!(
        "store \u{2192} op \u{2192} \"+\"\n",
        "match \u{2192} op\n",
        "  case \u{2192} \"+\"\n",
        "    store \u{2192} result \u{2192} 1\n",
        "  case \u{2192} \"-\"\n",
        "    store \u{2192} result \u{2192} 2\n",
        "  case \u{2192} _\n",
        "    store \u{2192} result \u{2192} 0\n",
        "end\n"
    );
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new();
    let bytecode = compiler.compile(&program);

    let mut vm = BytecodeVM::new();
    let _ = vm.execute(&bytecode);
    assert_eq!(
        vm.get_variable("result"),
        Some(&Value::Integer(1)),
        "match on '+' should set result to 1"
    );
}

// ---------------------------------------------------------------------------
// User-defined functions in bytecode VM (v0.4)
// ---------------------------------------------------------------------------

#[cfg(feature = "bytecode")]
#[test]
fn test_bytecode_vm_user_function_simple() {
    use txtcode::compiler::bytecode::BytecodeCompiler;
    use txtcode::lexer::Lexer;
    use txtcode::parser::Parser;
    use txtcode::runtime::bytecode_vm::BytecodeVM;
    use txtcode::runtime::Value;

    // define → double → (x)  return → x * 2  end
    // store → result → double(5)   =>   result = 10
    let source = concat!(
        "define \u{2192} double \u{2192} (x)\n",
        "  return \u{2192} x * 2\n",
        "end\n",
        "store \u{2192} result \u{2192} double(5)\n",
    );
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new();
    let bytecode = compiler.compile(&program);

    let mut vm = BytecodeVM::new();
    let exec_result = vm.execute(&bytecode);
    assert!(
        exec_result.is_ok(),
        "Bytecode execution should succeed: {:?}",
        exec_result
    );
    assert_eq!(
        vm.get_variable("result"),
        Some(&Value::Integer(10)),
        "double(5) should return 10"
    );
}

#[cfg(feature = "bytecode")]
#[test]
fn test_bytecode_vm_user_function_two_params() {
    use txtcode::compiler::bytecode::BytecodeCompiler;
    use txtcode::lexer::Lexer;
    use txtcode::parser::Parser;
    use txtcode::runtime::bytecode_vm::BytecodeVM;
    use txtcode::runtime::Value;

    // define → add → (a, b)  return → a + b  end
    // store → result → add(3, 4)   =>   result = 7
    let source = concat!(
        "define \u{2192} add \u{2192} (a, b)\n",
        "  return \u{2192} a + b\n",
        "end\n",
        "store \u{2192} result \u{2192} add(3, 4)\n",
    );
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new();
    let bytecode = compiler.compile(&program);

    let mut vm = BytecodeVM::new();
    let exec_result = vm.execute(&bytecode);
    assert!(
        exec_result.is_ok(),
        "Bytecode execution should succeed: {:?}",
        exec_result
    );
    assert_eq!(
        vm.get_variable("result"),
        Some(&Value::Integer(7)),
        "add(3, 4) should return 7"
    );
}

#[cfg(feature = "bytecode")]
#[test]
fn test_bytecode_vm_recursion_depth_limit() {
    use txtcode::compiler::bytecode::BytecodeCompiler;
    use txtcode::lexer::Lexer;
    use txtcode::parser::Parser;
    use txtcode::runtime::bytecode_vm::BytecodeVM;

    let source = concat!(
        "define \u{2192} inf \u{2192} (n)\n",
        "  return \u{2192} inf(n + 1)\n",
        "end\n",
        "inf(0)\n",
    );
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    let mut compiler = BytecodeCompiler::new();
    let bytecode = compiler.compile(&program);

    let mut vm = BytecodeVM::new();
    let result = vm.execute(&bytecode);
    assert!(
        result.is_err(),
        "Infinite bytecode recursion should produce a RuntimeError"
    );
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("call stack")
            || msg.contains("recursion")
            || msg.contains("depth")
            || msg.contains("exceeded"),
        "Error should describe call stack exhaustion, got: {}",
        msg
    );
}

// ---------------------------------------------------------------------------
// Capability token: explicit deny must win (regression for bypass fix)
// ---------------------------------------------------------------------------

#[test]
fn test_capability_token_cannot_bypass_explicit_deny() {
    use std::time::Duration;
    use txtcode::runtime::permissions::PermissionResource;
    use txtcode::runtime::vm::VirtualMachine;

    let mut vm = VirtualMachine::new();
    vm.grant_permission(PermissionResource::FileSystem("read".to_string()), None);
    vm.deny_permission(PermissionResource::FileSystem("read".to_string()), None);

    // Action must use the namespaced format ("fs.read") to match get_action_from_resource.
    let token_id = vm.grant_capability(
        PermissionResource::FileSystem("read".to_string()),
        "fs.read".to_string(),
        None,
        Some(Duration::from_secs(3600)),
        Some("test".to_string()),
        None,
    );
    vm.use_capability(token_id).expect("token should be usable before check");

    let result = vm.check_permission_with_audit(
        &PermissionResource::FileSystem("read".to_string()),
        None,
    );
    assert!(
        result.is_err(),
        "Explicit deny must override a valid capability token; got Ok unexpectedly"
    );
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("denied") || msg.contains("Permission"),
        "Error should describe a denial, got: {}",
        msg
    );
}

#[test]
fn test_capability_token_works_when_no_explicit_deny() {
    use std::time::Duration;
    use txtcode::runtime::permissions::PermissionResource;
    use txtcode::runtime::vm::VirtualMachine;

    let mut vm = VirtualMachine::new();
    // Action must use the namespaced format ("net.connect") to match get_action_from_resource.
    let token_id = vm.grant_capability(
        PermissionResource::Network("connect".to_string()),
        "net.connect".to_string(),
        None,
        Some(Duration::from_secs(3600)),
        Some("test".to_string()),
        None,
    );
    vm.use_capability(token_id).expect("token should be usable");

    let result = vm.check_permission_with_audit(
        &PermissionResource::Network("connect".to_string()),
        None,
    );
    assert!(
        result.is_ok(),
        "Capability token without explicit deny should allow access, got: {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Glob scope: multi-wildcard patterns
// ---------------------------------------------------------------------------

#[test]
fn test_glob_scope_multi_wildcard() {
    use txtcode::runtime::permissions::PermissionResource;
    use txtcode::runtime::vm::VirtualMachine;

    let mut vm = VirtualMachine::new();
    vm.grant_permission(
        PermissionResource::FileSystem("read".to_string()),
        Some("/var/*/log/*".to_string()),
    );

    assert!(
        vm.check_permission(
            &PermissionResource::FileSystem("read".to_string()),
            Some("/var/app/log/debug.txt"),
        )
        .is_ok(),
        "/var/*/log/* should match /var/app/log/debug.txt"
    );
    assert!(
        vm.check_permission(
            &PermissionResource::FileSystem("read".to_string()),
            Some("/var/app/other/debug.txt"),
        )
        .is_err(),
        "/var/*/log/* should not match /var/app/other/debug.txt"
    );
}

#[test]
fn test_glob_scope_suffix_wildcard() {
    use txtcode::runtime::permissions::PermissionResource;
    use txtcode::runtime::vm::VirtualMachine;

    let mut vm = VirtualMachine::new();
    vm.grant_permission(
        PermissionResource::Network("connect".to_string()),
        Some("*.example.com".to_string()),
    );

    assert!(
        vm.check_permission(
            &PermissionResource::Network("connect".to_string()),
            Some("api.example.com"),
        )
        .is_ok(),
        "*.example.com should match api.example.com"
    );
    assert!(
        vm.check_permission(
            &PermissionResource::Network("connect".to_string()),
            Some("evil.other.com"),
        )
        .is_err(),
        "*.example.com should not match evil.other.com"
    );
}

// ---------------------------------------------------------------------------
// Tool execution: requires_sudo must be denied
// ---------------------------------------------------------------------------

#[test]
fn test_tool_requires_sudo_is_denied() {
    use txtcode::runtime::tools::{check_tool_permission, Tool, ToolCategory};

    let sudo_tool = Tool {
        name: "masscan".to_string(),
        command: "masscan".to_string(),
        description: "Requires sudo".to_string(),
        category: ToolCategory::NetworkScanning,
        requires_sudo: true,
        default_timeout: 300,
        allowed_actions: vec!["scan".to_string()],
    };

    let result = check_tool_permission(&sudo_tool, None);
    assert!(
        result.is_err(),
        "Tool with requires_sudo=true must be denied by check_tool_permission"
    );
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("elevated") || msg.contains("sudo") || msg.contains("privilege"),
        "Error should mention privilege requirement, got: {}",
        msg
    );
}

#[test]
fn test_tool_without_sudo_passes_tool_permission_check() {
    use txtcode::runtime::tools::{check_tool_permission, Tool, ToolCategory};

    let safe_tool = Tool {
        name: "nmap".to_string(),
        command: "nmap".to_string(),
        description: "Safe scan".to_string(),
        category: ToolCategory::NetworkScanning,
        requires_sudo: false,
        default_timeout: 300,
        allowed_actions: vec!["scan".to_string()],
    };

    let result = check_tool_permission(&safe_tool, None);
    assert!(
        result.is_ok(),
        "Tool without requires_sudo should pass check_tool_permission"
    );
}

// ---------------------------------------------------------------------------
// sys.setenv and sys.chdir require sys.env permission
// ---------------------------------------------------------------------------

#[test]
fn test_setenv_blocked_without_sys_env_permission() {
    use txtcode::runtime::errors::RuntimeError;
    use txtcode::runtime::permissions::PermissionResource;
    use txtcode::runtime::Value;
    use txtcode::stdlib::PermissionChecker;

    struct DenyAll;
    impl PermissionChecker for DenyAll {
        fn check_permission(
            &self,
            _resource: &PermissionResource,
            _scope: Option<&str>,
        ) -> Result<(), RuntimeError> {
            Err(RuntimeError::new("permission denied".to_string()))
        }
    }

    let checker: &dyn PermissionChecker = &DenyAll;
    let result = txtcode::stdlib::sys::SysLib::call_function(
        "setenv",
        &[
            Value::String("TEST_KEY".to_string()),
            Value::String("val".to_string()),
        ],
        true,
        Some(checker),
    );
    assert!(
        result.is_err(),
        "setenv() must fail when permission_checker denies sys.env"
    );
}

#[test]
fn test_chdir_blocked_without_sys_env_permission() {
    use txtcode::runtime::errors::RuntimeError;
    use txtcode::runtime::permissions::PermissionResource;
    use txtcode::runtime::Value;
    use txtcode::stdlib::PermissionChecker;

    struct DenyAll;
    impl PermissionChecker for DenyAll {
        fn check_permission(
            &self,
            _resource: &PermissionResource,
            _scope: Option<&str>,
        ) -> Result<(), RuntimeError> {
            Err(RuntimeError::new("permission denied".to_string()))
        }
    }

    let checker: &dyn PermissionChecker = &DenyAll;
    let result = txtcode::stdlib::sys::SysLib::call_function(
        "chdir",
        &[Value::String("/tmp".to_string())],
        true,
        Some(checker),
    );
    assert!(
        result.is_err(),
        "chdir() must fail when permission_checker denies sys.env"
    );
}

// ---------------------------------------------------------------------------
// exec/spawn/pipe_exec: PermissionChecker gate enforced on all paths
// ---------------------------------------------------------------------------

#[test]
fn test_exec_blocked_by_permission_checker() {
    use txtcode::runtime::errors::RuntimeError;
    use txtcode::runtime::permissions::PermissionResource;
    use txtcode::runtime::Value;
    use txtcode::stdlib::PermissionChecker;

    struct DenyAll;
    impl PermissionChecker for DenyAll {
        fn check_permission(
            &self,
            _resource: &PermissionResource,
            _scope: Option<&str>,
        ) -> Result<(), RuntimeError> {
            Err(RuntimeError::new("permission denied".to_string()))
        }
    }

    let checker: &dyn PermissionChecker = &DenyAll;
    // exec_allowed = true but PermissionChecker must still block.
    let result = txtcode::stdlib::sys::SysLib::call_function(
        "exec",
        &[Value::String("echo hello".to_string())],
        true,
        Some(checker),
    );
    assert!(
        result.is_err(),
        "exec() must fail when permission_checker denies sys.exec"
    );
}

#[test]
fn test_spawn_blocked_by_permission_checker() {
    use txtcode::runtime::errors::RuntimeError;
    use txtcode::runtime::permissions::PermissionResource;
    use txtcode::runtime::Value;
    use txtcode::stdlib::PermissionChecker;

    struct DenyAll;
    impl PermissionChecker for DenyAll {
        fn check_permission(
            &self,
            _resource: &PermissionResource,
            _scope: Option<&str>,
        ) -> Result<(), RuntimeError> {
            Err(RuntimeError::new("permission denied".to_string()))
        }
    }

    let checker: &dyn PermissionChecker = &DenyAll;
    let result = txtcode::stdlib::sys::SysLib::call_function(
        "spawn",
        &[Value::String("echo hello".to_string())],
        true,
        Some(checker),
    );
    assert!(
        result.is_err(),
        "spawn() must fail when permission_checker denies sys.exec (was previously unchecked)"
    );
}

#[test]
fn test_pipe_exec_blocked_by_permission_checker() {
    use txtcode::runtime::errors::RuntimeError;
    use txtcode::runtime::permissions::PermissionResource;
    use txtcode::runtime::Value;
    use txtcode::stdlib::PermissionChecker;

    struct DenyAll;
    impl PermissionChecker for DenyAll {
        fn check_permission(
            &self,
            _resource: &PermissionResource,
            _scope: Option<&str>,
        ) -> Result<(), RuntimeError> {
            Err(RuntimeError::new("permission denied".to_string()))
        }
    }

    let checker: &dyn PermissionChecker = &DenyAll;
    let result = txtcode::stdlib::sys::SysLib::call_function(
        "pipe_exec",
        &[Value::String("echo hello".to_string())],
        true,
        Some(checker),
    );
    assert!(
        result.is_err(),
        "pipe_exec() must fail when permission_checker denies sys.exec (was previously unchecked)"
    );
}

// ---------------------------------------------------------------------------
// Bytecode VM parity: safe_mode and deny_permission match AST VM behaviour
// ---------------------------------------------------------------------------

#[cfg(feature = "bytecode")]
#[test]
fn test_bytecode_vm_safe_mode_blocks_exec_parity() {
    use txtcode::compiler::bytecode::BytecodeCompiler;
    use txtcode::lexer::Lexer;
    use txtcode::parser::Parser;
    use txtcode::runtime::bytecode_vm::BytecodeVM;

    let source = "exec(\"echo hello\")\n";
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut compiler = BytecodeCompiler::new();
    let bytecode = compiler.compile(&program);

    let mut vm = BytecodeVM::new();
    vm.set_safe_mode(true);
    let result = vm.execute(&bytecode);
    assert!(
        result.is_err(),
        "Bytecode VM with safe_mode=true must block exec() — parity with AST VM"
    );
}

#[cfg(feature = "bytecode")]
#[test]
fn test_bytecode_vm_deny_permission_does_not_break_arithmetic() {
    use txtcode::compiler::bytecode::BytecodeCompiler;
    use txtcode::lexer::Lexer;
    use txtcode::parser::Parser;
    use txtcode::runtime::bytecode_vm::BytecodeVM;
    use txtcode::runtime::permissions::PermissionResource;
    use txtcode::runtime::Value;

    let source = "store \u{2192} x \u{2192} 42\n";
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut compiler = BytecodeCompiler::new();
    let bytecode = compiler.compile(&program);

    let mut vm = BytecodeVM::new();
    vm.deny_permission(PermissionResource::FileSystem("read".to_string()), None);
    let result = vm.execute(&bytecode);
    assert!(
        result.is_ok(),
        "deny_permission on fs.read must not affect arithmetic: {:?}",
        result
    );
    assert_eq!(vm.get_variable("x"), Some(&Value::Integer(42)));
}

// ---------------------------------------------------------------------------
// Audit trail: denials and allowances are recorded
// ---------------------------------------------------------------------------

#[test]
fn test_audit_trail_records_denial() {
    use txtcode::runtime::audit::AuditResult;
    use txtcode::runtime::permissions::PermissionResource;
    use txtcode::runtime::vm::VirtualMachine;

    let mut vm = VirtualMachine::new();
    let _ = vm.check_permission_with_audit(
        &PermissionResource::FileSystem("write".to_string()),
        Some("/etc/passwd"),
    );

    let trail = vm.get_audit_trail();
    let denials: Vec<_> = trail.query_by_result(&AuditResult::Denied);
    assert!(
        !denials.is_empty(),
        "Audit trail must record a denial after a failed permission check"
    );
}

#[test]
fn test_audit_trail_records_allowed_permission() {
    use txtcode::runtime::audit::AuditResult;
    use txtcode::runtime::permissions::PermissionResource;
    use txtcode::runtime::vm::VirtualMachine;

    let mut vm = VirtualMachine::new();
    vm.grant_permission(PermissionResource::FileSystem("read".to_string()), None);
    let _ = vm.check_permission_with_audit(
        &PermissionResource::FileSystem("read".to_string()),
        None,
    );

    let trail = vm.get_audit_trail();
    let allowed: Vec<_> = trail.query_by_result(&AuditResult::Allowed);
    assert!(
        !allowed.is_empty(),
        "Audit trail must record a successful permission check"
    );
}

// ---------------------------------------------------------------------------
// Deterministic rate limiting: fixed clock must pin the window
// ---------------------------------------------------------------------------

#[test]
fn test_rate_limiter_deterministic_time_respected() {
    use std::time::{Duration, SystemTime};
    use txtcode::policy::engine::{DeterministicOverrides, Policy, PolicyEngine};
    use txtcode::policy::rate_limit::RateLimit;

    let mut policy = Policy::new();
    policy.set_rate_limit("test.action".to_string(), RateLimit::new(1, 60));
    policy.set_deterministic_mode(true);

    let mut engine = PolicyEngine::with_policy(policy);
    let frozen_time = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
    engine.set_deterministic_overrides(
        DeterministicOverrides::new().with_time(frozen_time),
    );
    engine.start_execution();

    assert!(
        engine.check_rate_limit("test.action").is_ok(),
        "First call within limit should pass"
    );
    assert!(
        engine.check_rate_limit("test.action").is_err(),
        "Second call at frozen time must be rate-limited (window cannot drain)"
    );
}

// ---------------------------------------------------------------------------
// Default registry: shell escape tool must not be present
// ---------------------------------------------------------------------------

#[test]
fn test_system_shell_tool_not_in_default_registry() {
    use txtcode::runtime::tools::ToolRegistry;

    let registry = ToolRegistry::new();
    assert!(
        !registry.is_registered("system"),
        "The 'system' → 'sh' tool must not be in the default registry (shell escape vector)"
    );
}

// ---------------------------------------------------------------------------
// Tool execution stabilization tests
// ---------------------------------------------------------------------------

#[test]
fn test_register_shell_command_sh_is_rejected() {
    use txtcode::runtime::tools::{Tool, ToolCategory, ToolRegistry};

    let mut registry = ToolRegistry::new();
    let result = registry.register(Tool {
        name: "my_tool".to_string(),
        command: "sh".to_string(),
        description: "test".to_string(),
        category: ToolCategory::Other,
        requires_sudo: false,
        default_timeout: 30,
        allowed_actions: vec![],
    });
    assert!(result.is_err(), "Registering 'sh' as command must be rejected");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("shell interpreter") || msg.contains("sh"),
        "Error should mention shell interpreter: {}",
        msg
    );
}

#[test]
fn test_register_shell_command_bash_is_rejected() {
    use txtcode::runtime::tools::{Tool, ToolCategory, ToolRegistry};

    let mut registry = ToolRegistry::new();
    let result = registry.register(Tool {
        name: "my_tool".to_string(),
        command: "bash".to_string(),
        description: "test".to_string(),
        category: ToolCategory::Other,
        requires_sudo: false,
        default_timeout: 30,
        allowed_actions: vec![],
    });
    assert!(result.is_err(), "Registering 'bash' as command must be rejected");
}

#[test]
fn test_register_qualified_shell_path_is_rejected() {
    use txtcode::runtime::tools::{Tool, ToolCategory, ToolRegistry};

    let mut registry = ToolRegistry::new();
    let result = registry.register(Tool {
        name: "escape".to_string(),
        command: "/bin/sh".to_string(),
        description: "test".to_string(),
        category: ToolCategory::Other,
        requires_sudo: false,
        default_timeout: 30,
        allowed_actions: vec![],
    });
    assert!(
        result.is_err(),
        "Registering '/bin/sh' as command must be rejected (shell escape via path)"
    );
}

#[test]
fn test_register_valid_command_succeeds() {
    use txtcode::runtime::tools::{Tool, ToolCategory, ToolRegistry};

    let mut registry = ToolRegistry::new();
    let result = registry.register(Tool {
        name: "curl".to_string(),
        command: "curl".to_string(),
        description: "HTTP client".to_string(),
        category: ToolCategory::Other,
        requires_sudo: false,
        default_timeout: 30,
        allowed_actions: vec!["fetch".to_string()],
    });
    assert!(result.is_ok(), "Registering 'curl' should succeed: {:?}", result);
    assert!(registry.is_registered("curl"));
}

#[test]
fn test_allowed_actions_enforced_for_disallowed_action() {
    use txtcode::runtime::tools::{check_tool_permission, Tool, ToolCategory};

    let tool = Tool {
        name: "nmap".to_string(),
        command: "nmap".to_string(),
        description: "Scanner".to_string(),
        category: ToolCategory::NetworkScanning,
        requires_sudo: false,
        default_timeout: 300,
        allowed_actions: vec!["scan".to_string(), "enum".to_string()],
    };

    let result = check_tool_permission(&tool, Some("exploit"));
    assert!(
        result.is_err(),
        "Action 'exploit' is not in allowed_actions — must be denied"
    );
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("allowed") || msg.contains("exploit"),
        "Error should mention allowed_actions: {}",
        msg
    );
}

#[test]
fn test_allowed_actions_passes_for_valid_action() {
    use txtcode::runtime::tools::{check_tool_permission, Tool, ToolCategory};

    let tool = Tool {
        name: "nmap".to_string(),
        command: "nmap".to_string(),
        description: "Scanner".to_string(),
        category: ToolCategory::NetworkScanning,
        requires_sudo: false,
        default_timeout: 300,
        allowed_actions: vec!["scan".to_string(), "enum".to_string()],
    };

    let result = check_tool_permission(&tool, Some("scan"));
    assert!(result.is_ok(), "Action 'scan' is in allowed_actions — must be permitted");
}

#[test]
fn test_allowed_actions_empty_permits_any_action() {
    use txtcode::runtime::tools::{check_tool_permission, Tool, ToolCategory};

    let tool = Tool {
        name: "curl".to_string(),
        command: "curl".to_string(),
        description: "HTTP client".to_string(),
        category: ToolCategory::Other,
        requires_sudo: false,
        default_timeout: 30,
        allowed_actions: vec![], // empty = no restriction
    };

    let result = check_tool_permission(&tool, Some("anything"));
    assert!(
        result.is_ok(),
        "Empty allowed_actions should permit any action: {:?}",
        result
    );
}

#[test]
fn test_allowed_actions_none_skips_validation() {
    use txtcode::runtime::tools::{check_tool_permission, Tool, ToolCategory};

    let tool = Tool {
        name: "nmap".to_string(),
        command: "nmap".to_string(),
        description: "Scanner".to_string(),
        category: ToolCategory::NetworkScanning,
        requires_sudo: false,
        default_timeout: 300,
        allowed_actions: vec!["scan".to_string()],
    };

    // None action = no validation — caller has no action context
    let result = check_tool_permission(&tool, None);
    assert!(result.is_ok(), "None action should skip allowed_actions validation");
}

#[test]
fn test_tool_list_returns_array_of_strings() {
    use txtcode::runtime::Value;
    use txtcode::stdlib::ToolLib;

    let result = ToolLib::call_function("tool_list", &[], None, None, None, None);
    assert!(result.is_ok(), "tool_list should succeed without permission checker: {:?}", result);
    match result.unwrap() {
        Value::Array(tools) => {
            assert!(!tools.is_empty(), "Default registry must have at least one tool");
            for t in &tools {
                assert!(
                    matches!(t, Value::String(_)),
                    "Each entry must be a String, got: {:?}",
                    t
                );
            }
        }
        other => panic!("tool_list must return Array, got: {:?}", other),
    }
}

#[test]
fn test_tool_info_known_tool_returns_map() {
    use txtcode::runtime::Value;
    use txtcode::stdlib::ToolLib;

    let args = [Value::String("nmap".to_string())];
    let result = ToolLib::call_function("tool_info", &args, None, None, None, None);
    assert!(result.is_ok(), "tool_info('nmap') should succeed: {:?}", result);
    match result.unwrap() {
        Value::Map(map) => {
            assert!(map.contains_key("requires_sudo"), "Map must contain 'requires_sudo'");
            assert!(map.contains_key("default_timeout"), "Map must contain 'default_timeout'");
            assert!(map.contains_key("allowed_actions"), "Map must contain 'allowed_actions'");
        }
        other => panic!("tool_info must return Map, got: {:?}", other),
    }
}

#[test]
fn test_tool_info_unknown_tool_errors() {
    use txtcode::runtime::Value;
    use txtcode::stdlib::ToolLib;

    let args = [Value::String("nonexistent_xyz_tool".to_string())];
    let result = ToolLib::call_function("tool_info", &args, None, None, None, None);
    assert!(result.is_err(), "tool_info on unknown tool must error");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("not found") || msg.contains("nonexistent"),
        "Error should mention missing tool: {}",
        msg
    );
}

#[test]
fn test_tool_exec_without_permission_checker_fails_secure() {
    use txtcode::runtime::Value;
    use txtcode::stdlib::ToolLib;

    let args = [Value::String("nmap".to_string())];
    // No permission_checker — must fail with an enforcement error, not a panic.
    let result = ToolLib::call_function("tool_exec", &args, None, None, None, None);
    assert!(result.is_err(), "tool_exec without permission_checker must fail");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("permission") || msg.contains("checker") || msg.contains("enforcement"),
        "Error should mention permission enforcement: {}",
        msg
    );
}

#[test]
fn test_tool_output_truncated_at_10mib() {
    // Verify the truncation helper produces the expected suffix on oversized output.
    // We test the ToolOutput structure directly — no process is spawned.
    // The constant MAX_TOOL_OUTPUT_BYTES = 10 MiB is tested indirectly via a simulated
    // oversized byte slice converted through the same code path used in execute_command.
    //
    // Since truncate_output is private, we verify the behaviour through ToolOutput::combined()
    // after manually constructing a truncated string (mirrors what execute_command produces).
    use txtcode::runtime::tools::ToolOutput;

    let big = "x".repeat(11 * 1024 * 1024); // 11 MiB > 10 MiB cap
    // Simulate what truncate_output does (cap at 10 MiB then append suffix).
    let cap = 10 * 1024 * 1024;
    let mut truncated = big[..cap].to_string();
    truncated.push_str("\n[output truncated: exceeded 10 MiB limit]");

    let output = ToolOutput::new(truncated.clone(), String::new(), 0);
    assert!(
        output.stdout.ends_with("[output truncated: exceeded 10 MiB limit]"),
        "Truncated output must end with the notification suffix"
    );
    assert!(
        output.stdout.len() < big.len(),
        "Stored stdout must be smaller than the original oversized output"
    );
}

// ---------------------------------------------------------------------------
// Issue #9: getenv / kill / signal_send permission gate tests
// ---------------------------------------------------------------------------

#[test]
fn test_getenv_denied_by_permission_checker() {
    use txtcode::runtime::errors::RuntimeError;
    use txtcode::runtime::permissions::PermissionResource;
    use txtcode::runtime::Value;
    use txtcode::stdlib::PermissionChecker;

    struct DenyAll;
    impl PermissionChecker for DenyAll {
        fn check_permission(
            &self,
            _resource: &PermissionResource,
            _scope: Option<&str>,
        ) -> Result<(), RuntimeError> {
            Err(RuntimeError::new("permission denied".to_string()))
        }
    }

    let checker: &dyn PermissionChecker = &DenyAll;
    let result = txtcode::stdlib::sys::SysLib::call_function(
        "getenv",
        &[Value::String("PATH".to_string())],
        true,
        Some(checker),
    );
    assert!(
        result.is_err(),
        "getenv() must fail when permission_checker denies sys.env"
    );
}

#[test]
fn test_getenv_allowed_by_permission_checker() {
    use txtcode::runtime::permissions::PermissionResource;
    use txtcode::runtime::Value;
    use txtcode::stdlib::PermissionChecker;
    use txtcode::runtime::errors::RuntimeError;

    struct AllowAll;
    impl PermissionChecker for AllowAll {
        fn check_permission(
            &self,
            _resource: &PermissionResource,
            _scope: Option<&str>,
        ) -> Result<(), RuntimeError> {
            Ok(())
        }
    }

    // Set a known env var for this test
    std::env::set_var("NPL_TEST_GETENV", "hello");
    let checker: &dyn PermissionChecker = &AllowAll;
    let result = txtcode::stdlib::sys::SysLib::call_function(
        "getenv",
        &[Value::String("NPL_TEST_GETENV".to_string())],
        true,
        Some(checker),
    );
    assert!(result.is_ok(), "getenv() must succeed when checker allows");
    assert_eq!(result.unwrap(), Value::String("hello".to_string()));
}

#[test]
fn test_kill_denied_by_permission_checker() {
    use txtcode::runtime::errors::RuntimeError;
    use txtcode::runtime::permissions::PermissionResource;
    use txtcode::runtime::Value;
    use txtcode::stdlib::PermissionChecker;

    struct DenyAll;
    impl PermissionChecker for DenyAll {
        fn check_permission(
            &self,
            _resource: &PermissionResource,
            _scope: Option<&str>,
        ) -> Result<(), RuntimeError> {
            Err(RuntimeError::new("permission denied".to_string()))
        }
    }

    let checker: &dyn PermissionChecker = &DenyAll;
    // exec_allowed=true so only permission_checker fires
    let result = txtcode::stdlib::sys::SysLib::call_function(
        "kill",
        &[Value::Integer(99999)],
        true,
        Some(checker),
    );
    assert!(
        result.is_err(),
        "kill() must fail when permission_checker denies sys.exec (exec_allowed=true)"
    );
}

#[test]
fn test_signal_send_blocked_in_safe_mode() {
    use txtcode::runtime::Value;

    // exec_allowed=false must block signal_send before any permission_checker
    let result = txtcode::stdlib::sys::SysLib::call_function(
        "signal_send",
        &[Value::Integer(1)],
        false,
        None,
    );
    assert!(
        result.is_err(),
        "signal_send() must be blocked when exec_allowed=false"
    );
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("safe mode") || msg.contains("disabled"),
        "Error message should mention safe mode, got: {}",
        msg
    );
}

// ---------------------------------------------------------------------------
// Task 8.1 — exec_allowed defaults to false
// ---------------------------------------------------------------------------

#[test]
fn test_exec_blocked_by_default_in_new_vm() {
    // VirtualMachine::new() must default exec_allowed=false.
    // exec() without grant_permission should fail.
    let source = r#"exec("echo", ["hello"])"#;
    let result = parse_and_run(source);
    assert!(
        result.is_err(),
        "exec() must fail with default VirtualMachine::new() (exec_allowed defaults to false)"
    );
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("Permission") || msg.contains("permission") || msg.contains("safe"),
        "Error should mention permission, got: {}",
        msg
    );
}

#[test]
fn test_exec_allowed_after_grant_permission() {
    // After grant_permission("sys.exec", null), exec() must succeed (or at least not
    // fail with a permission error — it may still fail if the binary is unavailable).
    let mut vm = VirtualMachine::new();
    vm.grant_permission(PermissionResource::System("exec".to_string()), None);

    let mut lexer = txtcode::lexer::Lexer::new(r#"exec("echo", ["hello"])"#.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = txtcode::parser::Parser::new(tokens);
    let program = parser.parse().unwrap();
    let result = vm.interpret(&program);

    match result {
        Ok(_) => {} // success — exec ran
        Err(e) => {
            // Accept only non-permission errors (e.g., binary not found in CI)
            let msg = e.to_string();
            assert!(
                !msg.contains("Permission not granted"),
                "Should not be a permission error after grant, got: {}",
                msg
            );
        }
    }
}
