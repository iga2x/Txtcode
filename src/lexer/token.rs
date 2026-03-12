/// Token representation for the lexer
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub value: String,
    pub span: (usize, usize),
}

impl Token {
    pub fn new(kind: TokenKind, value: String, span: (usize, usize)) -> Self {
        Self { kind, value, span }
    }
}

/// Token kinds for Txt-code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    // Literals
    Integer,
    Float,
    String,
    InterpolatedString,
    Char,
    Boolean,
    Null,

    // Identifiers
    Identifier,
    Keyword,

    // Operators
    Arrow,         // -> or → (both supported)
    Plus,          // +
    Minus,         // -
    Star,          // *
    Slash,         // /
    Percent,       // %
    Power,         // **
    Assignment,    // = (single equals, for compound assignment or future use)
    Equal,         // ==
    NotEqual,      // !=
    Less,          // <
    Greater,       // >
    LessEqual,     // <=
    GreaterEqual,  // >=
    And,           // and
    Or,            // or
    Not,           // not
    NullCoalesce,  // ??
    OptionalChain, // ?.
    QuestionMark,  // ? (standalone)
    Pipe,          // |>
    Increment,     // ++
    Decrement,     // --

    // Compound assignment operators
    PlusAssign,    // +=
    MinusAssign,   // -=
    StarAssign,    // *=
    SlashAssign,   // /=
    PercentAssign, // %=
    PowerAssign,   // **=
    BitAndAssign,  // &=
    BitOrAssign,   // |=
    BitXorAssign,  // ^=

    // Bitwise
    BitAnd,     // &
    BitOr,      // |
    BitXor,     // ^
    LeftShift,  // <<
    RightShift, // >>
    BitNot,     // ~

    // Delimiters
    LeftParen,    // (
    RightParen,   // )
    LeftBrace,    // {
    RightBrace,   // }
    LeftBracket,  // [
    RightBracket, // ]
    Comma,        // ,
    Semicolon,    // ;
    Colon,        // :
    Dot,          // .
    Spread,       // ... (spread/rest operator)

    // Special
    Eof,
    Newline,
    Whitespace,
    Comment,
}
