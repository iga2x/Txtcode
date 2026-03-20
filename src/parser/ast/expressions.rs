// Expression AST nodes

use super::common::{BinaryOperator, InterpolatedSegment, Literal, Parameter, Span, UnaryOperator};

/// Expression nodes
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Literal(Literal),
    Identifier(String),
    BinaryOp {
        left: Box<Expression>,
        op: BinaryOperator,
        right: Box<Expression>,
        span: Span,
    },
    UnaryOp {
        op: UnaryOperator,
        operand: Box<Expression>,
        span: Span,
    },
    FunctionCall {
        name: String,
        type_arguments: Option<Vec<crate::typecheck::types::Type>>, // Generic type arguments (e.g., <int, string>)
        arguments: Vec<Expression>,
        span: Span,
    },
    Array {
        elements: Vec<Expression>,
        span: Span,
    },
    Map {
        entries: Vec<(Expression, Expression)>,
        span: Span,
    },
    Set {
        elements: Vec<Expression>,
        span: Span,
    },
    Index {
        target: Box<Expression>,
        index: Box<Expression>,
        span: Span,
    },
    Member {
        target: Box<Expression>,
        name: String,
        span: Span,
    },
    Lambda {
        params: Vec<Parameter>,
        body: Box<Expression>,
        span: Span,
    },
    Ternary {
        condition: Box<Expression>,
        true_expr: Box<Expression>,
        false_expr: Box<Expression>,
        span: Span,
    },
    Slice {
        target: Box<Expression>,
        start: Option<Box<Expression>>,
        end: Option<Box<Expression>>,
        step: Option<Box<Expression>>,
        span: Span,
    },
    InterpolatedString {
        segments: Vec<InterpolatedSegment>,
        span: Span,
    },
    Await {
        expression: Box<Expression>,
        span: Span,
    },
    OptionalMember {
        target: Box<Expression>,
        name: String,
        span: Span,
    },
    OptionalCall {
        target: Box<Expression>,
        arguments: Vec<Expression>,
        span: Span,
    },
    OptionalIndex {
        target: Box<Expression>,
        index: Box<Expression>,
        span: Span,
    },
    /// Method call on a complex expression (e.g., arr[0].trim(), map["k"].split(","))
    MethodCall {
        object: Box<Expression>,
        method: String,
        type_arguments: Option<Vec<crate::typecheck::types::Type>>,
        arguments: Vec<Expression>,
        span: Span,
    },
    /// Struct literal: Point { x: 1, y: 2 }
    StructLiteral {
        name: String,
        fields: Vec<(String, Expression)>,
        span: Span,
    },
    /// Spread: ...expr — expands array into surrounding array/call
    Spread {
        value: Box<Expression>,
        span: Span,
    },
    /// Error propagation: expr? — if Err, early-return the error; if Ok, unwrap
    Propagate {
        value: Box<Expression>,
        span: Span,
    },
}
