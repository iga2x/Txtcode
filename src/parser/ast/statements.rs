// Statement AST nodes

use super::capabilities::CapabilityExpr;
use super::common::{BinaryOperator, Pattern, Span};
use super::Expression;

/// Statement nodes
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Assignment {
        pattern: Pattern, // Changed from name to pattern for destructuring
        type_annotation: Option<crate::typecheck::types::Type>,
        value: Expression,
        span: Span,
    },
    IndexAssignment {
        target: Expression, // The object being indexed (e.g., results)
        index: Expression,  // The index/key (e.g., "device")
        value: Expression,  // The value to assign
        span: Span,
    },
    CompoundAssignment {
        name: String,
        op: BinaryOperator,
        value: Expression,
        span: Span,
    },
    FunctionDef {
        name: String,
        type_params: Vec<super::common::TypeParam>, // Generic type parameters (e.g., <T>, <T: Comparable>)
        params: Vec<super::common::Parameter>,
        return_type: Option<crate::typecheck::types::Type>,
        body: Vec<Statement>,
        is_async: bool,                         // Whether this is an async function
        intent: Option<String>, // Intent declaration (what this function is meant to do)
        ai_hint: Option<String>, // AI guidance hint
        allowed_actions: Vec<CapabilityExpr>, // Explicitly allowed capabilities (e.g., [CapabilityExpr::Simple { resource: "fs", action: "read" }])
        forbidden_actions: Vec<CapabilityExpr>, // Explicitly forbidden capabilities (e.g., [CapabilityExpr::Simple { resource: "fs", action: "write" }])
        span: Span,
    },
    Return {
        value: Option<Expression>,
        span: Span,
    },
    /// `yield → value` — produce a value from a generator function
    Yield {
        value: Expression,
        span: Span,
    },
    Break {
        span: Span,
    },
    Continue {
        span: Span,
    },
    If {
        condition: Expression,
        then_branch: Vec<Statement>,
        else_if_branches: Vec<(Expression, Vec<Statement>)>,
        else_branch: Option<Vec<Statement>>,
        span: Span,
    },
    While {
        condition: Expression,
        body: Vec<Statement>,
        span: Span,
    },
    DoWhile {
        body: Vec<Statement>,
        condition: Expression,
        span: Span,
    },
    For {
        variable: String,
        iterable: Expression,
        body: Vec<Statement>,
        span: Span,
    },
    Repeat {
        count: Expression,
        body: Vec<Statement>,
        span: Span,
    },
    Expression(Expression),
    Assert {
        condition: Expression,
        message: Option<Expression>,
        span: Span,
    },
    Enum {
        name: String,
        variants: Vec<(String, Option<Expression>)>,
        span: Span,
    },
    Struct {
        name: String,
        /// Task E.2: Generic type parameters, e.g. `<T, U>`.
        type_params: Vec<String>,
        fields: Vec<(String, crate::typecheck::types::Type)>,
        /// Protocols this struct declares itself to implement (Task E.1).
        implements: Vec<String>,
        span: Span,
    },
    /// Task E.4 — Placeholder node inserted during error recovery so parsing can continue.
    Error {
        message: String,
        span: Span,
    },
    /// Task E.1 — Protocol declaration: a named set of required method signatures.
    Protocol {
        name: String,
        /// (method_name, param_types, return_type)
        methods: Vec<(String, Vec<String>, Option<String>)>,
        span: Span,
    },
    Match {
        value: Expression,
        cases: Vec<(Pattern, Option<Expression>, Vec<Statement>)>, // pattern, guard, body
        default: Option<Vec<Statement>>,
        span: Span,
    },
    Try {
        body: Vec<Statement>,
        catch: Option<(String, Vec<Statement>)>, // error_var, catch_body
        finally: Option<Vec<Statement>>,
        span: Span,
    },
    Import {
        modules: Vec<String>,
        from: Option<String>,
        alias: Option<String>,
        span: Span,
    },
    Export {
        names: Vec<String>,
        span: Span,
    },
    Const {
        name: String,
        value: Expression,
        span: Span,
    },
    Permission {
        resource: String,      // "fs", "net", "sys"
        action: String,        // "read", "write", "connect"
        scope: Option<String>, // Optional scope like "/tmp/*"
        span: Span,
    },
    /// Type alias: type → UserId → int
    TypeAlias {
        name: String,
        target: String,
        span: Span,
    },
    /// Named error: error → NotFound → "Resource not found"
    NamedError {
        name: String,
        message: Expression,
        span: Span,
    },
    /// impl → StructName ... end  — attach methods to a struct type
    Impl {
        struct_name: String,
        methods: Vec<Statement>, // Each element is Statement::FunctionDef
        span: Span,
    },
    /// `async → nursery\n  body\nend` — structured concurrency block.
    /// All tasks spawned with `nursery_spawn(fn)` inside this block are
    /// awaited when the block exits. Any error from a child task propagates
    /// to the nursery scope.
    Nursery {
        body: Vec<Statement>,
        span: Span,
    },
}

impl Statement {
    /// Return the (line, column) of this statement's first token, when available.
    /// Used by the VM to attach source locations to runtime errors.
    pub fn source_location(&self) -> Option<(usize, usize)> {
        let span = match self {
            Statement::Assignment { span, .. }
            | Statement::IndexAssignment { span, .. }
            | Statement::CompoundAssignment { span, .. }
            | Statement::FunctionDef { span, .. }
            | Statement::Return { span, .. }
            | Statement::Break { span, .. }
            | Statement::Continue { span, .. }
            | Statement::If { span, .. }
            | Statement::While { span, .. }
            | Statement::DoWhile { span, .. }
            | Statement::For { span, .. }
            | Statement::Repeat { span, .. }
            | Statement::Assert { span, .. }
            | Statement::Enum { span, .. }
            | Statement::Struct { span, .. }
            | Statement::Match { span, .. }
            | Statement::Try { span, .. }
            | Statement::Import { span, .. }
            | Statement::Export { span, .. }
            | Statement::Const { span, .. }
            | Statement::Permission { span, .. }
            | Statement::TypeAlias { span, .. }
            | Statement::NamedError { span, .. }
            | Statement::Impl { span, .. }
            | Statement::Yield { span, .. }
            | Statement::Nursery { span, .. }
            | Statement::Protocol { span, .. }
            | Statement::Error { span, .. } => Some(span),
            Statement::Expression(_) => None,
        };
        span.map(|s| (s.line, s.column))
    }
}

/// Program root node
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Statement>,
}
