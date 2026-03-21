// Common AST types shared across expressions and statements

use crate::typecheck::types::Type;

/// Source code location information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

impl Default for Span {
    fn default() -> Self {
        Self {
            start: 0,
            end: 0,
            line: 1,
            column: 1,
        }
    }
}

/// Literal values
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    String(String),
    Char(char),
    Boolean(bool),
    Null,
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Power,
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    And,
    Or,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    LeftShift,
    RightShift,
    NullCoalesce, // ??
    Pipe,         // |> (function pipe)
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Not,
    Minus,
    BitNot,
    Increment, // ++ (prefix)
    Decrement, // -- (prefix)
}

/// Function parameter
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub type_annotation: Option<Type>,
    pub is_variadic: bool,
    pub default_value: Option<crate::parser::ast::Expression>, // Forward reference
}

/// Pattern for destructuring
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Identifier(String),
    Array(Vec<Pattern>),
    Struct {
        fields: Vec<(String, Pattern)>, // field_name, pattern
        rest: Option<String>,           // rest pattern (e.g., ...rest)
    },
    Constructor {
        type_name: String,  // e.g., "Point"
        args: Vec<Pattern>, // e.g., [10, 20] or [x, y]
    },
    Ignore, // _ pattern
    /// Or-pattern: `1 | 2 | 3` — matches any of the sub-patterns
    Or(Vec<Pattern>),
    /// Inclusive range pattern: `1..=5` — matches integers/floats in [start, end]
    Range(
        Box<crate::parser::ast::Expression>,
        Box<crate::parser::ast::Expression>,
    ),
    /// Rest/spread pattern: `...name` — captures remaining array elements
    /// Only valid as the final element in an array pattern: `[a, b, ...rest]`
    Rest(String),
}

/// A generic type parameter with an optional constraint bound.
/// Examples: `T` (unconstrained), `T: Comparable`, `T: Numeric`
#[derive(Debug, Clone, PartialEq)]
pub struct TypeParam {
    pub name: String,
    /// Optional bound name, e.g. "Comparable" or "Numeric"
    pub constraint: Option<String>,
}

/// Segment of an interpolated string
#[derive(Debug, Clone, PartialEq)]
pub enum InterpolatedSegment {
    Text(String),
    Expression(crate::parser::ast::Expression), // Forward reference
}
