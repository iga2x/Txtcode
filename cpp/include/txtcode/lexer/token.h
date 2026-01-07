#pragma once

#include <string>
#include <variant>
#include <cstdint>
#include <cstddef>

namespace txtcode {

struct Span {
    std::size_t start;
    std::size_t end;
    std::size_t line;
    std::size_t column;

    Span(std::size_t start = 0, std::size_t end = 0, 
         std::size_t line = 0, std::size_t column = 0)
        : start(start), end(end), line(line), column(column) {}

    Span merge(const Span& other) const {
        return Span(
            std::min(start, other.start),
            std::max(end, other.end),
            line,
            column
        );
    }
};

enum class TokenKind {
    // Literals
    Integer,
    Float,
    String,
    Boolean,
    Null,

    // Identifiers
    Identifier,

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
    BitAnd,       // &
    BitOr,        // |
    BitXor,       // ^
    LeftShift,    // <<
    RightShift,   // >>
    BitNot,       // ~

    // Punctuation
    LeftParen,    // (
    RightParen,   // )
    LeftBracket,  // [
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
    Comment,
    MultiLineComment,

    // Special
    Newline,
    Whitespace,
    Eof,
};

struct TokenValue {
    std::variant<
        std::int64_t,      // Integer
        double,            // Float
        std::string,       // String, Identifier, Comment
        bool               // Boolean
    > value;

    TokenValue() = default;
    TokenValue(std::int64_t i) : value(i) {}
    TokenValue(double f) : value(f) {}
    TokenValue(const std::string& s) : value(s) {}
    TokenValue(bool b) : value(b) {}
};

struct Token {
    TokenKind kind;
    TokenValue value;
    Span span;

    Token(TokenKind kind, const Span& span, const TokenValue& value = TokenValue())
        : kind(kind), value(value), span(span) {}

    std::string toString() const;
};

} // namespace txtcode

