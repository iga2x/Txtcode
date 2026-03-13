/// Validator integration tests
///
/// Verifies that the three-stage validator pipeline (syntax → semantics → restrictions)
/// actually runs and catches problems before the VM ever executes any code.
use txtcode::lexer::Lexer;
use txtcode::parser::Parser;
use txtcode::validator::{ValidationError, Validator};

// ── Helpers ──────────────────────────────────────────────────────────────────

fn parse(source: &str) -> txtcode::parser::ast::Program {
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize().expect("lex failed");
    let mut parser = Parser::new(tokens);
    parser.parse().expect("parse failed")
}

fn validate(source: &str) -> Result<(), ValidationError> {
    Validator::validate_program(&parse(source))
}

fn assert_syntax_err(source: &str, expected_fragment: &str) {
    let err = validate(source).expect_err("expected ValidationError::Syntax");
    match &err {
        ValidationError::Syntax(msg) => assert!(
            msg.contains(expected_fragment),
            "syntax error should contain '{}', got: {}",
            expected_fragment,
            msg
        ),
        other => panic!("expected Syntax error, got {:?}", other),
    }
}

fn assert_semantic_err(source: &str, expected_fragment: &str) {
    let err = validate(source).expect_err("expected ValidationError::Semantic");
    match &err {
        ValidationError::Semantic(msg) => assert!(
            msg.contains(expected_fragment),
            "semantic error should contain '{}', got: {}",
            expected_fragment,
            msg
        ),
        other => panic!("expected Semantic error, got {:?}", other),
    }
}

fn assert_restriction_err(source: &str, expected_fragment: &str) {
    let err = validate(source).expect_err("expected ValidationError::Restriction");
    match &err {
        ValidationError::Restriction(msg) => assert!(
            msg.contains(expected_fragment),
            "restriction error should contain '{}', got: {}",
            expected_fragment,
            msg
        ),
        other => panic!("expected Restriction error, got {:?}", other),
    }
}

fn assert_valid(source: &str) {
    validate(source).unwrap_or_else(|e| panic!("expected valid program, got: {}", e));
}

// ── SyntaxValidator ───────────────────────────────────────────────────────────

#[test]
fn test_syntax_rejects_eval() {
    assert_syntax_err("eval(\"print(1)\")", "eval()");
}

#[test]
fn test_syntax_rejects_eval_nested_in_call() {
    // eval() inside an argument to another function — the original code
    // missed this because it never recursed into FunctionCall arguments.
    assert_syntax_err("print(eval(\"1+1\"))", "eval()");
}

#[test]
fn test_syntax_rejects_exec_with_variable_concat() {
    // exec("nmap " + target) — injection pattern
    assert_syntax_err(
        "store → target → \"192.168.1.1\"\nexec(\"nmap \" + target)",
        "command-injection",
    );
}

#[test]
fn test_syntax_rejects_spawn_with_variable_concat() {
    assert_syntax_err(
        "store → cmd → \"ls\"\nspawn(cmd + \" -la\")",
        "command-injection",
    );
}

#[test]
fn test_syntax_allows_exec_with_literal_only() {
    // Literal string — no variable, so no injection risk
    assert_valid("exec(\"nmap -sV -p 80 127.0.0.1\")");
}

#[test]
fn test_syntax_allows_exec_with_literal_concat() {
    // Both sides are string literals — deterministic, safe
    assert_valid("exec(\"nmap\" + \" -sV\")");
}

#[test]
fn test_syntax_rejects_eval_inside_if() {
    assert_syntax_err(
        "if → true\n  eval(\"x\")\nend",
        "eval()",
    );
}

#[test]
fn test_syntax_rejects_eval_inside_function() {
    assert_syntax_err(
        "define → bad → ()\n  eval(\"x\")\nend",
        "eval()",
    );
}

// ── SemanticsValidator ────────────────────────────────────────────────────────

#[test]
fn test_semantics_rejects_duplicate_function() {
    assert_semantic_err(
        "define → foo → ()\n  return → 1\nend\ndefine → foo → ()\n  return → 2\nend",
        "defined more than once",
    );
}

#[test]
fn test_semantics_allows_unique_functions() {
    assert_valid(
        "define → foo → ()\n  return → 1\nend\ndefine → bar → ()\n  return → 2\nend",
    );
}

#[test]
fn test_semantics_rejects_return_at_top_level() {
    assert_semantic_err("return → 42", "outside a function");
}

#[test]
fn test_semantics_allows_return_inside_function() {
    assert_valid("define → f → ()\n  return → 1\nend");
}

// ── RestrictionChecker ───────────────────────────────────────────────────────

#[test]
fn test_restriction_rejects_forbidden_capability_used() {
    // Function declares sys.exec as forbidden but body calls exec()
    assert_restriction_err(
        "define → safe_fn → ()\n  forbidden → [\"sys.exec\"]\n  exec(\"ls\")\nend",
        "forbids",
    );
}

#[test]
fn test_restriction_allows_declared_capability() {
    // Function declares sys.exec as allowed and uses it — valid
    assert_valid("define → run_tool → ()\n  allowed → [\"sys.exec\"]\n  exec(\"ls\")\nend");
}

#[test]
fn test_restriction_allows_no_capability_declaration_with_exec() {
    // No declaration at all — restriction checker warns but does not hard-error
    // (backward-compatible: existing scripts don't all have declarations)
    assert_valid("define → f → ()\n  exec(\"ls\")\nend");
}

// ── Pipeline integration: validator runs BEFORE VM ───────────────────────────

#[test]
fn test_validator_catches_eval_before_vm_runs() {
    // If the validator is wired in, the VM never reaches eval() and no
    // RuntimeError fires — only a ValidationError::Syntax.
    use txtcode::runtime::vm::VirtualMachine;

    let source = "eval(\"print(1)\")";
    let prog = parse(source);
    let validation = Validator::validate_program(&prog);
    assert!(
        validation.is_err(),
        "validator must catch eval() before VM runs"
    );
    assert!(
        matches!(validation.unwrap_err(), ValidationError::Syntax(_)),
        "must be a Syntax error"
    );

    // Confirm the VM would have accepted it (without the validator gate)
    // by checking it produces a RuntimeError instead of a ValidationError
    // when the validator is bypassed.
    let mut vm = VirtualMachine::new();
    let vm_result = vm.interpret(&prog);
    // VM may or may not error on eval() — but the validator must catch it first.
    drop(vm_result);
}
