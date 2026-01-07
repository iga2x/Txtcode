use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

impl Span {
    pub fn new(start: usize, end: usize, line: usize, column: usize) -> Self {
        Self {
            start,
            end,
            line,
            column,
        }
    }

    pub fn merge(&self, other: &Span) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            line: self.line,
            column: self.column,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Null,

    // Identifiers
    Identifier(String),

    // Keywords
    Store,
    Define,
    Return,
    If,
    Else,
    Elseif,
    End,
    While,
    For,
    In,
    Repeat,
    Times,
    Match,
    Case,
    Break,
    Continue,
    Try,
    Catch,
    Import,
    From,
    As,
    And,
    Or,
    Not,
    Assert,

    // Operators
    Arrow,        // →
    Plus,         // +
    Minus,        // -
    Star,         // *
    Slash,        // /
    Percent,      // %
    Power,        // **
    Equal,        // ==
    NotEqual,     // !=
    Less,         // <
    Greater,      // >
    LessEqual,    // <=
    GreaterEqual, // >=
    Assign,       // =
    PlusEqual,    // +=
    MinusEqual,   // -=
    StarEqual,    // *=
    SlashEqual,   // /=
    PercentEqual, // %=
    PowerEqual,   // **=
    BitAnd,       // &
    BitOr,        // |
    BitXor,       // ^
    LeftShift,    // <<
    RightShift,   // >>
    BitNot,       // ~

    // Punctuation
    LeftParen,    // (
    RightParen,   // )
    LeftBracket, // [
    RightBracket, // ]
    LeftBrace,    // {
    RightBrace,   // }
    Comma,        // ,
    Colon,        // :
    Semicolon,    // ;
    Dot,          // .
    Question,     // ?
    Exclamation,  // !

    // Comments
    Comment(String),
    MultiLineComment(String),

    // Special
    Newline,
    Whitespace,
    Eof,
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Integer(n) => write!(f, "{}", n),
            TokenKind::Float(n) => write!(f, "{}", n),
            TokenKind::String(s) => write!(f, "\"{}\"", s),
            TokenKind::Boolean(b) => write!(f, "{}", b),
            TokenKind::Null => write!(f, "null"),
            TokenKind::Identifier(s) => write!(f, "{}", s),
            TokenKind::Store => write!(f, "store"),
            TokenKind::Define => write!(f, "define"),
            TokenKind::Return => write!(f, "return"),
            TokenKind::If => write!(f, "if"),
            TokenKind::Else => write!(f, "else"),
            TokenKind::Elseif => write!(f, "elseif"),
            TokenKind::End => write!(f, "end"),
            TokenKind::While => write!(f, "while"),
            TokenKind::For => write!(f, "for"),
            TokenKind::In => write!(f, "in"),
            TokenKind::Repeat => write!(f, "repeat"),
            TokenKind::Times => write!(f, "times"),
            TokenKind::Match => write!(f, "match"),
            TokenKind::Case => write!(f, "case"),
            TokenKind::Break => write!(f, "break"),
            TokenKind::Continue => write!(f, "continue"),
            TokenKind::Try => write!(f, "try"),
            TokenKind::Catch => write!(f, "catch"),
            TokenKind::Import => write!(f, "import"),
            TokenKind::From => write!(f, "from"),
            TokenKind::As => write!(f, "as"),
            TokenKind::And => write!(f, "and"),
            TokenKind::Or => write!(f, "or"),
            TokenKind::Not => write!(f, "not"),
            TokenKind::Arrow => write!(f, "->"),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Star => write!(f, "*"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::Percent => write!(f, "%"),
            TokenKind::Power => write!(f, "**"),
            TokenKind::Equal => write!(f, "=="),
            TokenKind::NotEqual => write!(f, "!="),
            TokenKind::Less => write!(f, "<"),
            TokenKind::Greater => write!(f, ">"),
            TokenKind::LessEqual => write!(f, "<="),
            TokenKind::GreaterEqual => write!(f, ">="),
            TokenKind::Assign => write!(f, "="),
            TokenKind::PlusEqual => write!(f, "+="),
            TokenKind::MinusEqual => write!(f, "-="),
            TokenKind::StarEqual => write!(f, "*="),
            TokenKind::SlashEqual => write!(f, "/="),
            TokenKind::PercentEqual => write!(f, "%="),
            TokenKind::PowerEqual => write!(f, "**="),
            TokenKind::BitAnd => write!(f, "&"),
            TokenKind::BitOr => write!(f, "|"),
            TokenKind::BitXor => write!(f, "^"),
            TokenKind::LeftShift => write!(f, "<<"),
            TokenKind::RightShift => write!(f, ">>"),
            TokenKind::BitNot => write!(f, "~"),
            TokenKind::LeftParen => write!(f, "("),
            TokenKind::RightParen => write!(f, ")"),
            TokenKind::LeftBracket => write!(f, "["),
            TokenKind::RightBracket => write!(f, "]"),
            TokenKind::LeftBrace => write!(f, "{{"),
            TokenKind::RightBrace => write!(f, "}}"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::Semicolon => write!(f, ";"),
            TokenKind::Dot => write!(f, "."),
            TokenKind::Question => write!(f, "?"),
            TokenKind::Exclamation => write!(f, "!"),
            TokenKind::Comment(s) => write!(f, "# {}", s),
            TokenKind::MultiLineComment(s) => write!(f, "## {} ##", s),
            TokenKind::Newline => write!(f, "\\n"),
            TokenKind::Whitespace => write!(f, " "),
            TokenKind::Eof => write!(f, "EOF"),
        }
    }
}

