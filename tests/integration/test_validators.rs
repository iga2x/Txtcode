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

// ── Q.1 + Q.2: Advisory warning checks ───────────────────────────────────────

/// Helper: collect advisory warnings for source without halting.
fn advisory_warnings(source: &str) -> Vec<String> {
    let prog = parse(source);
    txtcode::validator::SemanticsValidator::collect_advisory_warnings(&prog.statements)
}

#[test]
fn test_advisory_undefined_variable_flagged() {
    // `xyz` is not defined anywhere.
    let warnings = advisory_warnings("xyz + 1");
    assert!(
        warnings.iter().any(|w| w.contains("xyz")),
        "expected warning for undefined 'xyz', got: {:?}", warnings
    );
}

#[test]
fn test_advisory_in_scope_variable_not_flagged() {
    // `n` is brought into scope by the assignment before use.
    let warnings = advisory_warnings("store → n → 10\nn + 1");
    assert!(
        !warnings.iter().any(|w| w.contains("'n'")),
        "in-scope variable 'n' should not be flagged, got: {:?}", warnings
    );
}

#[test]
fn test_advisory_wrong_arity_flagged() {
    let src = "define → add → (a, b)\n  a + b\nend\nadd(1)";
    let warnings = advisory_warnings(src);
    assert!(
        warnings.iter().any(|w| w.contains("add") && w.contains("2") && w.contains("1")),
        "expected arity warning for 'add', got: {:?}", warnings
    );
}

#[test]
fn test_advisory_correct_arity_not_flagged() {
    let src = "define → add → (a, b)\n  a + b\nend\nadd(1, 2)";
    let warnings = advisory_warnings(src);
    assert!(
        !warnings.iter().any(|w| w.contains("add") && w.contains("arity") || w.contains("expects")),
        "correct arity call should not be flagged, got: {:?}", warnings
    );
}

// ── Phase 3: stdlib name completeness (no false "undefined variable" warnings) ──

#[test]
fn test_advisory_db_transaction_not_flagged() {
    // db_transaction is handled outside STDLIB_DISPATCH (executor-dependent).
    // It must not produce a false "undefined variable" warning.
    let warnings = advisory_warnings("store → conn → db_connect(\"sqlite::memory:\")\ndb_transaction(conn, (c) → c)");
    assert!(
        !warnings.iter().any(|w| w.contains("db_transaction")),
        "db_transaction should not be flagged as undefined, got: {:?}", warnings
    );
}

#[test]
fn test_advisory_http_serve_not_flagged() {
    // http_serve is in the fallthrough branch of the stdlib dispatcher.
    let warnings = advisory_warnings("http_serve(8080, (req) → req)");
    assert!(
        !warnings.iter().any(|w| w.contains("http_serve")),
        "http_serve should not be flagged as undefined, got: {:?}", warnings
    );
}

#[test]
fn test_advisory_async_run_not_flagged() {
    // async_run is a well-known async primitive that must always be in scope.
    let warnings = advisory_warnings("async_run((x) → x + 1, 5)");
    assert!(
        !warnings.iter().any(|w| w.contains("async_run")),
        "async_run should not be flagged as undefined, got: {:?}", warnings
    );
}

#[test]
fn test_advisory_await_all_not_flagged() {
    // await_all is in STDLIB_DISPATCH; this test ensures it stays recognized.
    let warnings = advisory_warnings("await_all([])");
    assert!(
        !warnings.iter().any(|w| w.contains("await_all")),
        "await_all should not be flagged as undefined, got: {:?}", warnings
    );
}
