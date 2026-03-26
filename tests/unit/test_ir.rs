//! Unit tests for the IR layer (Group R).
//!
//! R.3 completion gate: 3 tests —
//!   1. constant fold (integer arithmetic folded at lower time)
//!   2. dead branch elimination (if true / if false)
//!   3. capability call node shape (read_file wrapped correctly)

use txtcode::ir::{IrBuilder, IrNode};
use txtcode::lexer::Lexer;
use txtcode::parser::Parser;

fn parse(src: &str) -> txtcode::parser::ast::statements::Program {
    let mut lexer = Lexer::new(src.to_string());
    let tokens = lexer.tokenize().expect("lex");
    let mut parser = Parser::new(tokens);
    parser.parse().expect("parse")
}

fn lower(src: &str) -> txtcode::ir::ProgramIr {
    let prog = parse(src);
    IrBuilder::new().lower(&prog)
}

// ── R.3 test 1: constant fold ─────────────────────────────────────────────────

/// `store → x → 3 + 4` should produce a single `Const(7)` assignment value,
/// not a BinOp node.
#[test]
fn test_ir_constant_fold_integer_arithmetic() {
    use txtcode::parser::ast::common::Literal;

    let ir = lower("store → x → 3 + 4");
    assert_eq!(ir.fold_count, 1, "expected exactly 1 fold");

    // The top-level node must be an Assign whose value is Const(7).
    match &ir.nodes[0] {
        IrNode::Assign { value, .. } => {
            assert_eq!(
                **value,
                IrNode::Const(Literal::Integer(7)),
                "3 + 4 should fold to 7"
            );
        }
        other => panic!("expected Assign, got {:?}", other),
    }
}

// ── R.3 test 2: dead branch elimination ──────────────────────────────────────

/// `if true` branch should be collapsed to its body; `if false` should become Nop.
#[test]
fn test_ir_dead_branch_elimination() {
    // `if true` → body only (no If node)
    let ir_true = lower(
        "if → true\n  store → x → 1\nend",
    );
    assert!(
        ir_true.dead_branch_count >= 1,
        "expected at least 1 dead branch eliminated"
    );
    // The node must NOT be an IrNode::If — it should be a Block.
    match &ir_true.nodes[0] {
        IrNode::Block(_) => {} // expected
        IrNode::Nop => {}      // also acceptable (empty else)
        IrNode::If { .. } => panic!("if true should not produce an If IR node"),
        other => panic!("unexpected node for if-true: {:?}", other),
    }

    // `if false` with no else → Nop
    let ir_false = lower(
        "if → false\n  store → x → 99\nend",
    );
    assert!(
        ir_false.dead_branch_count >= 1,
        "expected at least 1 dead branch eliminated for if-false"
    );
    assert_eq!(
        ir_false.nodes[0],
        IrNode::Nop,
        "if false with no else should produce Nop"
    );
}

// ── R.3 test 3: capability call node shape ────────────────────────────────────

/// A call to `read_file("foo.txt")` must be wrapped in `IrNode::CapabilityCall`
/// with resource="fs" and action="read".
#[test]
fn test_ir_capability_call_node_shape() {
    let ir = lower(r#"read_file("foo.txt")"#);

    match &ir.nodes[0] {
        IrNode::CapabilityCall { capability, call } => {
            assert_eq!(capability.resource, "fs");
            assert_eq!(capability.action, "read");
            match call.as_ref() {
                IrNode::Call { name, .. } => assert_eq!(name, "read_file"),
                other => panic!("inner call should be Call, got {:?}", other),
            }
        }
        other => panic!("expected CapabilityCall, got {:?}", other),
    }
}
