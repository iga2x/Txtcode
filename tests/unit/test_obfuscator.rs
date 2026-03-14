/// Obfuscator unit tests (Issue #4)
///
/// Verify identifier mangling: user-defined names get renamed, stdlib calls
/// are preserved, and the obfuscated program still executes correctly.
use txtcode::lexer::Lexer;
use txtcode::parser::ast::{Expression, Statement};
use txtcode::parser::Parser;
use txtcode::runtime::vm::VirtualMachine;
use txtcode::security::obfuscator::Obfuscator;

fn parse(source: &str) -> txtcode::parser::ast::Program {
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    parser.parse().unwrap()
}

fn run_repl(source: &str) -> txtcode::runtime::Value {
    let program = parse(source);
    let mut vm = VirtualMachine::new();
    vm.interpret_repl(&program).unwrap()
}

fn run_obfuscated_repl(source: &str) -> txtcode::runtime::Value {
    let program = parse(source);
    let obfuscated = Obfuscator::new().obfuscate(&program);
    let mut vm = VirtualMachine::new();
    vm.interpret_repl(&obfuscated).unwrap()
}

/// A variable name is renamed to `_o0`.
#[test]
fn test_obfuscator_renames_variable() {
    let source = "store → x → 5";
    let program = parse(source);
    let obfuscated = Obfuscator::new().obfuscate(&program);

    // The Assignment pattern should now hold `_o0` instead of `x`
    if let Statement::Assignment { pattern, .. } = &obfuscated.statements[0] {
        if let txtcode::parser::ast::Pattern::Identifier(name) = pattern {
            assert_eq!(name, "_o0", "variable 'x' must be renamed to '_o0'");
        } else {
            panic!("expected Identifier pattern");
        }
    } else {
        panic!("expected Assignment statement");
    }
}

/// A `print()` call must not be mangled (stdlib name).
#[test]
fn test_obfuscator_preserves_stdlib_calls() {
    let source = "print(\"hello\")";
    let program = parse(source);
    let obfuscated = Obfuscator::new().obfuscate(&program);

    if let Statement::Expression(Expression::FunctionCall { name, .. }) = &obfuscated.statements[0] {
        assert_eq!(name, "print", "stdlib function 'print' must not be mangled");
    } else {
        panic!("expected Expression(FunctionCall)");
    }
}

/// Function parameters are renamed consistently within the body.
#[test]
fn test_obfuscator_function_params_renamed() {
    // NPL function definition syntax: define → name → (params) \n body \n end
    let source = "define → double → (n)\n  return → n * 2\nend";
    let program = parse(source);
    let obfuscated = Obfuscator::new().obfuscate(&program);

    if let Statement::FunctionDef { name: fn_name, params, body, .. } = &obfuscated.statements[0] {
        // Function name mangled
        assert!(fn_name.starts_with("_o"), "function name must be mangled, got: {}", fn_name);
        // Parameter renamed
        assert!(params[0].name.starts_with("_o"), "param name must be mangled, got: {}", params[0].name);
        let param_mangled = &params[0].name;
        // Body uses the same mangled name
        if let Statement::Return { value: Some(expr), .. } = &body[0] {
            if let Expression::BinaryOp { left, .. } = expr {
                if let Expression::Identifier(id) = left.as_ref() {
                    assert_eq!(id, param_mangled, "param usage in body must match mangled param name");
                } else {
                    panic!("expected Identifier in BinaryOp left");
                }
            } else {
                panic!("expected BinaryOp in return");
            }
        } else {
            panic!("expected Return statement in body");
        }
    } else {
        panic!("expected FunctionDef");
    }
}

/// Obfuscated program produces the same output as the original.
#[test]
fn test_obfuscator_program_still_runs() {
    // Use interpret_repl to get the last expression value (avoids top-level return issue)
    let source = "store → x → 10\nstore → y → x * 2\ny";
    let original_result = run_repl(source);
    let obfuscated_result = run_obfuscated_repl(source);
    assert_eq!(
        original_result, obfuscated_result,
        "obfuscated program must produce identical output"
    );
}
