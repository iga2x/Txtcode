use crate::lexer::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Expression(Expression),
    Assignment {
        name: String,
        type_annotation: Option<Type>,
        value: Expression,
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
        params: Vec<Parameter>,
        return_type: Option<Type>,
        body: Vec<Statement>,
        span: Span,
    },
    Return {
        value: Option<Expression>,
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
    Match {
        value: Expression,
        cases: Vec<MatchCase>,
        default: Option<Vec<Statement>>,
        span: Span,
    },
    Break {
        span: Span,
    },
    Continue {
        span: Span,
    },
    Try {
        body: Vec<Statement>,
        catch: Option<(String, Vec<Statement>)>,
        span: Span,
    },
    Import {
        items: Vec<String>,
        from: Option<String>,
        alias: Option<String>,
        span: Span,
    },
    Assert {
        condition: Expression,
        message: Option<Expression>,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchCase {
    pub pattern: Pattern,
    pub guard: Option<Expression>,
    pub body: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Literal(Expression),
    Identifier(String),
    Wildcard,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub type_annotation: Option<Type>,
    pub span: Span,
}

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
        arguments: Vec<Expression>,
        span: Span,
    },
    Array {
        elements: Vec<Expression>,
        span: Span,
    },
    Map {
        entries: Vec<(String, Expression)>,
        span: Span,
    },
    Index {
        target: Box<Expression>,
        index: Box<Expression>,
        span: Span,
    },
    Slice {
        target: Box<Expression>,
        start: Option<Box<Expression>>,
        end: Option<Box<Expression>>,
        span: Span,
    },
    Member {
        target: Box<Expression>,
        member: String,
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
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Null,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
    Add,          // +
    Subtract,     // -
    Multiply,     // *
    Divide,       // /
    Modulo,       // %
    Power,        // **
    Equal,        // ==
    NotEqual,     // !=
    Less,         // <
    Greater,      // >
    LessEqual,    // <=
    GreaterEqual, // >=
    And,          // and
    Or,           // or
    BitAnd,       // &
    BitOr,        // |
    BitXor,       // ^
    LeftShift,    // <<
    RightShift,   // >>
    Arrow,        // -> (for function calls)
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOperator {
    Not,    // not
    Minus,  // -
    BitNot, // ~
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    String,
    Bool,
    Array(Box<Type>),
    Map(Box<Type>), // Map value type
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
    },
    Identifier(String), // User-defined types
    Generic(String),    // Generic type parameter
}

impl Expression {
    pub fn span(&self) -> Span {
        match self {
            Expression::Literal(_) => Span::new(0, 0, 1, 1), // Default span for literals
            Expression::Identifier(_) => Span::new(0, 0, 1, 1),
            Expression::BinaryOp { span, .. } => span.clone(),
            Expression::UnaryOp { span, .. } => span.clone(),
            Expression::FunctionCall { span, .. } => span.clone(),
            Expression::Array { span, .. } => span.clone(),
            Expression::Map { span, .. } => span.clone(),
            Expression::Index { span, .. } => span.clone(),
            Expression::Slice { span, .. } => span.clone(),
            Expression::Member { span, .. } => span.clone(),
            Expression::Lambda { span, .. } => span.clone(),
            Expression::Ternary { span, .. } => span.clone(),
        }
    }
}

