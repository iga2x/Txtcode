#include "txtcode/lexer/lexer.h"
#include "txtcode/lexer/keywords.h"
#include <cctype>
#include <stdexcept>

namespace txtcode {

Lexer::Lexer(const std::string& source)
    : source_(source), position_(0), line_(1), column_(1) {}

std::vector<Token> Lexer::tokenize() {
    std::vector<Token> tokens;
    
    while (!isAtEnd()) {
        std::size_t start_pos = position_;
        std::size_t start_line = line_;
        std::size_t start_col = column_;
        
        Token token = nextToken();
        
        // Skip whitespace and comments
        if (token.kind != TokenKind::Whitespace && 
            token.kind != TokenKind::Comment && 
            token.kind != TokenKind::MultiLineComment) {
            tokens.push_back(token);
        }
        
        if (token.kind == TokenKind::Eof) {
            break;
        }
    }
    
    return tokens;
}

Token Lexer::nextToken() {
    if (isAtEnd()) {
        return Token(TokenKind::Eof, currentSpan(0));
    }
    
    std::size_t start_pos = position_;
    std::size_t start_line = line_;
    std::size_t start_col = column_;
    
    char ch = current();
    
    // Arrow operator (→) - UTF-8: 0xE2 0x86 0x92
    if (static_cast<unsigned char>(ch) == 0xE2 && 
        position_ + 2 < source_.length() &&
        static_cast<unsigned char>(source_[position_ + 1]) == 0x86 &&
        static_cast<unsigned char>(source_[position_ + 2]) == 0x92) {
        advance(3);
        return Token(TokenKind::Arrow, currentSpan(start_pos));
    }
    
    // Single character operators
    switch (ch) {
        case '+':
            advance();
            return Token(TokenKind::Plus, currentSpan(start_pos));
        case '-':
            advance();
            return Token(TokenKind::Minus, currentSpan(start_pos));
        case '*':
            advance();
            if (match('*')) {
                return Token(TokenKind::Power, currentSpan(start_pos));
            }
            return Token(TokenKind::Star, currentSpan(start_pos));
        case '/':
            advance();
            return Token(TokenKind::Slash, currentSpan(start_pos));
        case '%':
            advance();
            return Token(TokenKind::Percent, currentSpan(start_pos));
        case '=':
            advance();
            if (match('=')) {
                return Token(TokenKind::Equal, currentSpan(start_pos));
            }
            return Token(TokenKind::Assign, currentSpan(start_pos));
        case '!':
            advance();
            if (match('=')) {
                return Token(TokenKind::NotEqual, currentSpan(start_pos));
            }
            return Token(TokenKind::Exclamation, currentSpan(start_pos));
        case '<':
            advance();
            if (match('=')) {
                return Token(TokenKind::LessEqual, currentSpan(start_pos));
            } else if (match('<')) {
                return Token(TokenKind::LeftShift, currentSpan(start_pos));
            }
            return Token(TokenKind::Less, currentSpan(start_pos));
        case '>':
            advance();
            if (match('=')) {
                return Token(TokenKind::GreaterEqual, currentSpan(start_pos));
            } else if (match('>')) {
                return Token(TokenKind::RightShift, currentSpan(start_pos));
            }
            return Token(TokenKind::Greater, currentSpan(start_pos));
        case '&':
            advance();
            return Token(TokenKind::BitAnd, currentSpan(start_pos));
        case '|':
            advance();
            return Token(TokenKind::BitOr, currentSpan(start_pos));
        case '^':
            advance();
            return Token(TokenKind::BitXor, currentSpan(start_pos));
        case '~':
            advance();
            return Token(TokenKind::BitNot, currentSpan(start_pos));
        case '(':
            advance();
            return Token(TokenKind::LeftParen, currentSpan(start_pos));
        case ')':
            advance();
            return Token(TokenKind::RightParen, currentSpan(start_pos));
        case '[':
            advance();
            return Token(TokenKind::LeftBracket, currentSpan(start_pos));
        case ']':
            advance();
            return Token(TokenKind::RightBracket, currentSpan(start_pos));
        case '{':
            advance();
            return Token(TokenKind::LeftBrace, currentSpan(start_pos));
        case '}':
            advance();
            return Token(TokenKind::RightBrace, currentSpan(start_pos));
        case ',':
            advance();
            return Token(TokenKind::Comma, currentSpan(start_pos));
        case ':':
            advance();
            return Token(TokenKind::Colon, currentSpan(start_pos));
        case ';':
            advance();
            return Token(TokenKind::Semicolon, currentSpan(start_pos));
        case '.':
            advance();
            return Token(TokenKind::Dot, currentSpan(start_pos));
        case '?':
            advance();
            return Token(TokenKind::Question, currentSpan(start_pos));
        case '#':
            advance();
            if (match('#')) {
                skipMultiLineComment();
                return Token(TokenKind::MultiLineComment, currentSpan(start_pos), 
                           TokenValue(""));
            } else {
                skipComment();
                return Token(TokenKind::Comment, currentSpan(start_pos), TokenValue(""));
            }
        case '"':
        case '\'':
            return scanString();
        case '\n':
            advance();
            return Token(TokenKind::Newline, currentSpan(start_pos));
        default:
            if (std::isspace(ch)) {
                skipWhitespace();
                return Token(TokenKind::Whitespace, currentSpan(start_pos));
            } else if (std::isdigit(ch)) {
                return scanNumber();
            } else if (std::isalpha(ch) || ch == '_') {
                return scanIdentifier();
            } else {
                advance();
                throw std::runtime_error("Unexpected character: " + std::string(1, ch));
            }
    }
}

char Lexer::current() const {
    if (isAtEnd()) return '\0';
    return source_[position_];
}

char Lexer::peek(std::size_t offset) const {
    if (position_ + offset >= source_.length()) return '\0';
    return source_[position_ + offset];
}

void Lexer::advance(std::size_t count) {
    for (std::size_t i = 0; i < count; ++i) {
        if (position_ < source_.length()) {
            if (source_[position_] == '\n') {
                line_++;
                column_ = 1;
            } else {
                column_++;
            }
            position_++;
        }
    }
}

bool Lexer::isAtEnd() const {
    return position_ >= source_.length();
}

bool Lexer::match(char expected) {
    if (isAtEnd() || current() != expected) return false;
    advance();
    return true;
}

void Lexer::skipWhitespace() {
    while (!isAtEnd() && std::isspace(current()) && current() != '\n') {
        advance();
    }
}

void Lexer::skipComment() {
    while (!isAtEnd() && current() != '\n') {
        advance();
    }
}

void Lexer::skipMultiLineComment() {
    while (!isAtEnd()) {
        if (current() == '#' && peek() == '#') {
            advance(2);
            break;
        }
        advance();
    }
}

Token Lexer::scanString() {
    char quote = current();
    std::size_t start_pos = position_;
    advance(); // Skip opening quote
    
    std::string value;
    while (!isAtEnd() && current() != quote) {
        if (current() == '\\') {
            advance();
            switch (current()) {
                case 'n': value += '\n'; break;
                case 't': value += '\t'; break;
                case 'r': value += '\r'; break;
                case '\\': value += '\\'; break;
                case '"': value += '"'; break;
                case '\'': value += '\''; break;
                default: value += current(); break;
            }
            advance();
        } else {
            value += current();
            advance();
        }
    }
    
    if (isAtEnd()) {
        throw std::runtime_error("Unterminated string");
    }
    
    advance(); // Skip closing quote
    return Token(TokenKind::String, currentSpan(start_pos), TokenValue(value));
}

Token Lexer::scanNumber() {
    std::size_t start_pos = position_;
    std::string num_str;
    bool is_float = false;
    
    while (!isAtEnd() && (std::isdigit(current()) || current() == '.')) {
        if (current() == '.') {
            if (is_float) break; // Already saw a dot
            is_float = true;
        }
        num_str += current();
        advance();
    }
    
    if (is_float) {
        double value = std::stod(num_str);
        return Token(TokenKind::Float, currentSpan(start_pos), TokenValue(value));
    } else {
        std::int64_t value = std::stoll(num_str);
        return Token(TokenKind::Integer, currentSpan(start_pos), TokenValue(value));
    }
}

Token Lexer::scanIdentifier() {
    std::size_t start_pos = position_;
    std::string ident;
    
    while (!isAtEnd() && (std::isalnum(current()) || current() == '_')) {
        ident += current();
        advance();
    }
    
    TokenKind kind = KeywordMap::getKeyword(ident);
    if (kind == TokenKind::Identifier) {
        return Token(TokenKind::Identifier, currentSpan(start_pos), TokenValue(ident));
    } else {
        return Token(kind, currentSpan(start_pos));
    }
}

Span Lexer::currentSpan(std::size_t start) const {
    return Span(start, position_, line_, column_);
}

} // namespace txtcode

