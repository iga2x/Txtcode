/// Permission denial and security tests for v0.2
///
/// These tests verify that the permission system correctly denies access
/// when permissions are not granted or explicitly denied.

use txtcode::lexer::Lexer;
use txtcode::parser::Parser;
use txtcode::runtime::vm::VirtualMachine;
use txtcode::runtime::permissions::PermissionResource;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_and_run(source: &str) -> Result<txtcode::runtime::Value, txtcode::runtime::errors::RuntimeError> {
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

    let result = vm.check_permission(
        &PermissionResource::FileSystem("read".to_string()),
        None,
    );
    assert!(result.is_err(), "Expected denied filesystem read to return Err");
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

    let result = vm.check_permission(
        &PermissionResource::Network("connect".to_string()),
        None,
    );
    assert!(result.is_err(), "Expected denied network connect to return Err");
}

#[test]
fn test_permission_denied_process_exec() {
    let mut vm = VirtualMachine::new();
    vm.deny_permission(PermissionResource::Process(vec!["exec".to_string()]), None);

    let result = vm.check_permission(
        &PermissionResource::Process(vec!["exec".to_string()]),
        None,
    );
    assert!(result.is_err(), "Expected denied process exec to return Err");
}

#[test]
fn test_permission_not_granted_by_default() {
    let vm = VirtualMachine::new();
    // No permissions granted — any resource check should fail
    let result = vm.check_permission(
        &PermissionResource::FileSystem("write".to_string()),
        None,
    );
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

    let result = vm.check_permission(
        &PermissionResource::Network("connect".to_string()),
        None,
    );
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
    use txtcode::stdlib::StdLib;
    use txtcode::stdlib::FunctionExecutor;
    use txtcode::runtime::Value;

    struct NoopExecutor;
    impl FunctionExecutor for NoopExecutor {
        fn call_function_value(&mut self, _func: &Value, _args: &[Value]) -> Result<Value, txtcode::runtime::errors::RuntimeError> {
            Err(txtcode::runtime::errors::RuntimeError::new("Not supported".to_string()))
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

#[test]
fn test_bytecode_vm_null_coalesce_returns_default_for_null() {
    use txtcode::compiler::bytecode::{Instruction, Constant};
    use txtcode::runtime::bytecode_vm::BytecodeVM;
    use txtcode::runtime::Value;
    use txtcode::compiler::bytecode::Bytecode;

    // Manually build bytecode: push null, push 42, NullCoalesce → expect 42
    let bytecode = Bytecode {
        instructions: vec![
            Instruction::PushConstant(0), // null
            Instruction::PushConstant(1), // 42
            Instruction::NullCoalesce,
        ],
        constants: vec![
            Constant::Null,
            Constant::Integer(42),
        ],
    };

    let mut vm = BytecodeVM::new();
    let result = vm.execute(&bytecode);
    assert!(result.is_ok(), "NullCoalesce should not error: {:?}", result);
    assert_eq!(result.unwrap(), Value::Integer(42));
}

#[test]
fn test_bytecode_vm_null_coalesce_returns_value_when_not_null() {
    use txtcode::compiler::bytecode::{Instruction, Constant};
    use txtcode::runtime::bytecode_vm::BytecodeVM;
    use txtcode::runtime::Value;
    use txtcode::compiler::bytecode::Bytecode;

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
    };

    let mut vm = BytecodeVM::new();
    let result = vm.execute(&bytecode);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Value::String("hello".to_string()));
}

#[test]
fn test_bytecode_vm_optional_member_null_safe() {
    // OptionalGetField on Null must return Null (safe navigation, no error)
    use txtcode::compiler::bytecode::{Instruction, Constant};
    use txtcode::runtime::bytecode_vm::BytecodeVM;
    use txtcode::compiler::bytecode::Bytecode;
    use txtcode::runtime::core::Value;

    let bytecode = Bytecode {
        instructions: vec![
            Instruction::PushConstant(0),
            Instruction::OptionalGetField("name".to_string()),
        ],
        constants: vec![Constant::Null],
    };

    let mut vm = BytecodeVM::new();
    let result = vm.execute(&bytecode);
    assert!(result.is_ok(), "OptionalGetField on null must not error");
    assert_eq!(result.unwrap(), Value::Null, "OptionalGetField on null must return Null");
}

// ---------------------------------------------------------------------------
// Migration: dry-run smoke test
// ---------------------------------------------------------------------------

#[test]
fn test_migration_dry_run_does_not_modify_source() {
    use txtcode::runtime::migration::MigrationFramework;
    use txtcode::runtime::compatibility::Version;

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
    assert!(result.is_ok(), "Dry-run migration should not error: {:?}", result);

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
    let result = parse_and_run(
        "store → i → 0\nwhile → i < 3\n  store → i → i + 1\nend",
    );
    assert!(result.is_ok());
}

#[test]
fn test_smoke_run_function_definition_and_call() {
    let source = "define \u{2192} add \u{2192} (a, b)\n  return \u{2192} a + b\nend\nstore \u{2192} r \u{2192} add(3, 4)\n";
    let result = parse_and_run(source);
    assert!(result.is_ok(), "Function definition and call should work: {:?}", result);
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
    let result = parse_and_run(
        "try\n  store → x → 1 / 0\ncatch → e\n  print → \"caught\"\nend",
    );
    assert!(result.is_ok(), "try/catch should handle division by zero: {:?}", result);
}

#[test]
fn test_smoke_formatter_does_not_panic() {
    use txtcode::tools::formatter::Formatter;
    let source = "store → x → 42\nprint → x\n";
    let result = Formatter::format_source(source);
    assert!(result.is_ok(), "Formatter should not panic on simple source: {:?}", result);
}

#[test]
fn test_smoke_linter_does_not_panic() {
    use txtcode::tools::linter::Linter;
    let source = "store → x → 42\nprint → x\n";
    let result = Linter::lint_source(source);
    assert!(result.is_ok(), "Linter should not panic on simple source: {:?}", result);
}

// ---------------------------------------------------------------------------
// Bytecode VM: control flow and iterators (v0.3 additions)
// ---------------------------------------------------------------------------

#[test]
fn test_bytecode_vm_while_loop_executes_correctly() {
    use txtcode::compiler::bytecode::{Instruction, Constant, Bytecode};
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
    let _ = Value::Integer(0);  // prevent unused import warning

    // Instead use the compiler end-to-end
    use txtcode::lexer::Lexer;
    use txtcode::parser::Parser;
    use txtcode::compiler::bytecode::BytecodeCompiler;

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

#[test]
fn test_bytecode_vm_for_loop_iterates_array() {
    use txtcode::lexer::Lexer;
    use txtcode::parser::Parser;
    use txtcode::compiler::bytecode::BytecodeCompiler;
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

#[test]
fn test_bytecode_vm_for_loop_empty_array_skips_body() {
    use txtcode::lexer::Lexer;
    use txtcode::parser::Parser;
    use txtcode::compiler::bytecode::BytecodeCompiler;
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

#[test]
fn test_bytecode_vm_increment_operator() {
    use txtcode::lexer::Lexer;
    use txtcode::parser::Parser;
    use txtcode::compiler::bytecode::BytecodeCompiler;
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
    assert_eq!(vm.get_variable("x"), Some(&Value::Integer(6)), "x should be 6 after ++x");
    assert_eq!(vm.get_variable("y"), Some(&Value::Integer(6)), "y should capture new value 6");
}

// ---------------------------------------------------------------------------
// Overflow guards (v0.3)
// ---------------------------------------------------------------------------

#[test]
fn test_integer_overflow_add_returns_error() {
    let result = parse_and_run(&format!("store \u{2192} x \u{2192} {} + 1", i64::MAX));
    assert!(result.is_err(), "Adding 1 to i64::MAX should produce an overflow error");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("overflow") || msg.contains("Overflow"),
        "Error should mention overflow, got: {}", msg);
}

#[test]
fn test_integer_overflow_multiply_returns_error() {
    let result = parse_and_run(&format!("store \u{2192} x \u{2192} {} * 2", i64::MAX));
    assert!(result.is_err(), "Multiplying i64::MAX by 2 should produce an overflow error");
}

#[test]
fn test_recursion_depth_limit() {
    // Infinite recursion should produce a RuntimeError, not a stack overflow / panic
    let source = "define \u{2192} recurse \u{2192} (n)\n  return \u{2192} recurse(n + 1)\nend\nrecurse(0)\n";
    let result = parse_and_run(source);
    assert!(result.is_err(), "Infinite recursion should return a RuntimeError");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("call stack") || msg.contains("recursion") || msg.contains("depth") || msg.contains("exceeded"),
        "Error should describe call stack exhaustion, got: {}", msg
    );
}

#[test]
fn test_bytecode_vm_match_string_case() {
    use txtcode::lexer::Lexer;
    use txtcode::parser::Parser;
    use txtcode::compiler::bytecode::BytecodeCompiler;
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
// User-defined functions in bytecode VM (v0.3)
// ---------------------------------------------------------------------------

#[test]
fn test_bytecode_vm_user_function_simple() {
    use txtcode::lexer::Lexer;
    use txtcode::parser::Parser;
    use txtcode::compiler::bytecode::BytecodeCompiler;
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
    assert!(exec_result.is_ok(), "Bytecode execution should succeed: {:?}", exec_result);
    assert_eq!(
        vm.get_variable("result"),
        Some(&Value::Integer(10)),
        "double(5) should return 10"
    );
}

#[test]
fn test_bytecode_vm_user_function_two_params() {
    use txtcode::lexer::Lexer;
    use txtcode::parser::Parser;
    use txtcode::compiler::bytecode::BytecodeCompiler;
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
    assert!(exec_result.is_ok(), "Bytecode execution should succeed: {:?}", exec_result);
    assert_eq!(
        vm.get_variable("result"),
        Some(&Value::Integer(7)),
        "add(3, 4) should return 7"
    );
}

#[test]
fn test_bytecode_vm_recursion_depth_limit() {
    use txtcode::lexer::Lexer;
    use txtcode::parser::Parser;
    use txtcode::compiler::bytecode::BytecodeCompiler;
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
    assert!(result.is_err(), "Infinite bytecode recursion should produce a RuntimeError");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("call stack") || msg.contains("recursion") || msg.contains("depth") || msg.contains("exceeded"),
        "Error should describe call stack exhaustion, got: {}", msg
    );
}
