//! IR instruction nodes — backend-agnostic intermediate representation.
//!
//! `IrNode` uses structured control flow (no flat jump targets) so that
//! backends (bytecode, WASM) can lower it without recomputing dominator trees.

use crate::parser::ast::common::{BinaryOperator, Literal, Span, UnaryOperator};

/// A capability call site, explicitly tracked for policy enforcement.
/// Backends must emit a permission check before executing the wrapped call.
#[derive(Debug, Clone, PartialEq)]
pub struct CapabilityCall {
    /// e.g. "fs", "net", "process", "db", "sys"
    pub resource: String,
    /// e.g. "read", "write", "connect", "exec"
    pub action: String,
    pub span: Span,
}

/// A function parameter: name + optional type hint string.
#[derive(Debug, Clone, PartialEq)]
pub struct IrParam {
    pub name: String,
    pub type_hint: Option<String>,
}

/// Primary IR node — structured, backend-agnostic.
///
/// All constant folding and dead-branch elimination have already been applied
/// by the time a backend receives an `IrNode`.
#[derive(Debug, Clone, PartialEq)]
pub enum IrNode {
    // ── Values ─────────────────────────────────────────────────────────────────
    /// Compile-time constant (already folded).
    Const(Literal),
    /// Runtime variable reference.
    Var(String),

    // ── Operations ─────────────────────────────────────────────────────────────
    BinOp {
        left: Box<IrNode>,
        op: BinaryOperator,
        right: Box<IrNode>,
        span: Span,
    },
    UnaryOp {
        op: UnaryOperator,
        operand: Box<IrNode>,
        span: Span,
    },

    // ── Calls ──────────────────────────────────────────────────────────────────
    Call {
        name: String,
        args: Vec<IrNode>,
        span: Span,
    },
    /// A call that touches a guarded resource.
    /// Backends must emit a capability permission check before this call.
    CapabilityCall {
        call: Box<IrNode>,
        capability: CapabilityCall,
    },

    // ── Collections ────────────────────────────────────────────────────────────
    Array(Vec<IrNode>),
    Map(Vec<(IrNode, IrNode)>),

    // ── Structured control flow ────────────────────────────────────────────────
    /// Sequential block of statements.
    Block(Vec<IrNode>),
    If {
        condition: Box<IrNode>,
        then_block: Box<IrNode>,
        else_ifs: Vec<(IrNode, IrNode)>,
        else_block: Option<Box<IrNode>>,
        span: Span,
    },
    /// `condition = None` means loop-forever (from `repeat` or `do-while`-like);
    /// `condition = Some(cond)` means while-cond.
    Loop {
        condition: Option<Box<IrNode>>,
        body: Box<IrNode>,
        span: Span,
    },
    ForEach {
        variable: String,
        iterable: Box<IrNode>,
        body: Box<IrNode>,
        span: Span,
    },

    // ── Assignments ─────────────────────────────────────────────────────────────
    Assign {
        name: String,
        value: Box<IrNode>,
        span: Span,
    },
    IndexAssign {
        target: Box<IrNode>,
        index: Box<IrNode>,
        value: Box<IrNode>,
        span: Span,
    },

    // ── Functions ───────────────────────────────────────────────────────────────
    FunctionDef {
        name: String,
        params: Vec<IrParam>,
        body: Box<IrNode>,
        is_async: bool,
        span: Span,
    },
    Return(Option<Box<IrNode>>),
    Break,
    Continue,

    // ── Misc ────────────────────────────────────────────────────────────────────
    /// No-op placeholder for AST nodes that have no IR representation
    /// (e.g. struct defs, type aliases, import statements).
    Nop,
}
