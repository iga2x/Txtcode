/// V.3: Grammar.ebnf verification — Rust integration tests.
/// Each test calls the parser directly (no subprocess, no Python).
/// One test per grammar divergence corrected in docs/grammar.ebnf.

use txtcode::lexer::Lexer;
use txtcode::parser::Parser;

fn parses_ok(src: &str) {
    let mut lexer = Lexer::new(src.to_string());
    let tokens = lexer.tokenize().expect("lex error");
    let mut parser = Parser::new(tokens);
    let result = parser.parse();
    assert!(
        result.is_ok(),
        "expected parse OK:\nsource: {}\nerror: {:?}",
        src,
        result.err()
    );
}

fn parses_err(src: &str) {
    let mut lexer = Lexer::new(src.to_string());
    let tokens = lexer.tokenize().expect("lex error");
    let mut parser = Parser::new(tokens);
    let result = parser.parse();
    assert!(
        result.is_err(),
        "expected parse error but succeeded:\nsource: {}",
        src
    );
}

// ── G1: struct implements — arrow is optional, colon is NOT required ──────────

#[test]
fn test_grammar_struct_implements_with_arrow() {
    parses_ok("struct Foo(x: int) implements → Serializable");
}

#[test]
fn test_grammar_struct_implements_no_separator() {
    parses_ok("struct Foo(x: int) implements Serializable");
}

// ── G2: function_def — dotted method name Type.method ─────────────────────────

#[test]
fn test_grammar_dotted_method_def() {
    parses_ok("define → Foo.greet → (self)\n  return → \"hi\"\nend");
}

// ── G3: compound_assign is a valid statement ──────────────────────────────────

#[test]
fn test_grammar_compound_assign_all_ops() {
    parses_ok(
        "store → x → 10\n\
         x += 1\n\
         x -= 1\n\
         x *= 2\n\
         x /= 2\n\
         x %= 3\n\
         x **= 2",
    );
}

// ── G4: optional index ?[ — two separate tokens, not one ─────────────────────

#[test]
fn test_grammar_optional_index_separate_tokens() {
    parses_ok("store → m → {\"a\": 1}\nstore → v → m?[\"a\"]");
}

// ── G5: import — "from" clause is optional ────────────────────────────────────

#[test]
fn test_grammar_import_without_from() {
    parses_ok("import math");
}

#[test]
fn test_grammar_import_with_arrow_and_from() {
    parses_ok("import → json from \"json\"");
}

// ── G6: pattern_list — correct comma-separated notation ──────────────────────

#[test]
fn test_grammar_pattern_list_with_rest() {
    parses_ok(
        "store → arr → [1, 2, 3]\n\
         store → [a, b, ...rest] → arr",
    );
}

#[test]
fn test_grammar_pattern_list_no_rest() {
    parses_ok(
        "store → arr → [1, 2, 3]\n\
         store → [a, b, c] → arr",
    );
}

// ── G7: set_literal — bare braces, no "set" keyword prefix ───────────────────

#[test]
fn test_grammar_set_literal_bare_braces() {
    parses_ok("store → s → {1, 2, 3}");
}

#[test]
fn test_grammar_map_vs_set_disambiguation() {
    parses_ok("store → m → {\"a\": 1, \"b\": 2}");
    parses_ok("store → s → {1, 2, 3}");
}

// ── G8: slice — single colon, not double colon ────────────────────────────────

#[test]
fn test_grammar_slice_start_end() {
    parses_ok("store → arr → [1,2,3,4,5]\nstore → s → arr[1:3]");
}

#[test]
fn test_grammar_slice_step_only() {
    parses_ok("store → arr → [1,2,3,4,5]\nstore → s → arr[::2]");
}

// ── G9: optional call — obj?.(args) with dot before paren ────────────────────

#[test]
fn test_grammar_optional_call_with_dot() {
    parses_ok("store → f → null\nstore → r → f?.(1, 2)");
}
