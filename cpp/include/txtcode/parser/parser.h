#pragma once

#include <vector>
#include <memory>
#include "txtcode/lexer/lexer.h"
#include "txtcode/parser/ast.h"

namespace txtcode {

class Parser {
public:
    explicit Parser(const std::vector<Token>& tokens);
    
    std::unique_ptr<Program> parse();
    
private:
    std::vector<Token> tokens_;
    std::size_t current_;
    
    Token current() const;
    Token previous() const;
    bool isAtEnd() const;
    bool check(TokenKind kind) const;
    bool match(const std::vector<TokenKind>& kinds);
    Token advance();
    Token consume(TokenKind kind, const std::string& message);
    
    std::unique_ptr<Statement> statement();
    std::unique_ptr<Statement> assignment();
    std::unique_ptr<Statement> functionDef();
    std::unique_ptr<Statement> returnStmt();
    std::unique_ptr<Statement> ifStmt();
    std::unique_ptr<Statement> whileStmt();
    std::unique_ptr<Statement> forStmt();
    std::unique_ptr<Statement> repeatStmt();
    std::unique_ptr<Statement> matchStmt();
    std::unique_ptr<Statement> breakStmt();
    std::unique_ptr<Statement> continueStmt();
    std::unique_ptr<Statement> tryStmt();
    std::unique_ptr<Statement> importStmt();
    
    std::unique_ptr<Expression> expression();
    std::unique_ptr<Expression> assignmentExpr();
    std::unique_ptr<Expression> orExpr();
    std::unique_ptr<Expression> andExpr();
    std::unique_ptr<Expression> equality();
    std::unique_ptr<Expression> comparison();
    std::unique_ptr<Expression> term();
    std::unique_ptr<Expression> factor();
    std::unique_ptr<Expression> unary();
    std::unique_ptr<Expression> call();
    std::unique_ptr<Expression> primary();
    
    std::vector<std::unique_ptr<Statement>> block();
    std::vector<Parameter> parameters();
    std::unique_ptr<Type> typeAnnotation();
    Pattern pattern();
};

} // namespace txtcode

