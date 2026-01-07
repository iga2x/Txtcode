#pragma once

#include <string>
#include <vector>
#include <memory>
#include "txtcode/lexer/token.h"

namespace txtcode {

class Lexer {
public:
    explicit Lexer(const std::string& source);
    
    std::vector<Token> tokenize();
    Token nextToken();
    
private:
    std::string source_;
    std::size_t position_;
    std::size_t line_;
    std::size_t column_;
    
    char current() const;
    char peek(std::size_t offset = 1) const;
    void advance(std::size_t count = 1);
    bool isAtEnd() const;
    bool match(char expected);
    
    void skipWhitespace();
    void skipComment();
    void skipMultiLineComment();
    
    Token scanString();
    Token scanNumber();
    Token scanIdentifier();
    Token scanOperator();
    
    Span currentSpan(std::size_t start) const;
};

} // namespace txtcode

