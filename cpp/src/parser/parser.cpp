#include "txtcode/parser/parser.h"
#include <stdexcept>

namespace txtcode {

Parser::Parser(const std::vector<Token>& tokens)
    : tokens_(tokens), current_(0) {}

std::unique_ptr<Program> Parser::parse() {
    auto program = std::make_unique<Program>();
    program->span = Span(0, 0, 1, 1);
    
    while (!isAtEnd()) {
        try {
            auto stmt = statement();
            if (stmt) {
                program->statements.push_back(std::move(stmt));
            }
        } catch (const std::exception& e) {
            // Error recovery - skip to next statement
            while (!isAtEnd() && current().kind != TokenKind::Newline) {
                advance();
            }
        }
    }
    
    return program;
}

Token Parser::current() const {
    if (isAtEnd()) return Token(TokenKind::Eof, Span());
    return tokens_[current_];
}

Token Parser::previous() const {
    if (current_ == 0) return Token(TokenKind::Eof, Span());
    return tokens_[current_ - 1];
}

bool Parser::isAtEnd() const {
    return current_ >= tokens_.size() || 
           (current_ < tokens_.size() && tokens_[current_].kind == TokenKind::Eof);
}

bool Parser::check(TokenKind kind) const {
    if (isAtEnd()) return false;
    return current().kind == kind;
}

bool Parser::match(const std::vector<TokenKind>& kinds) {
    for (auto kind : kinds) {
        if (check(kind)) {
            advance();
            return true;
        }
    }
    return false;
}

Token Parser::advance() {
    if (!isAtEnd()) current_++;
    return previous();
}

Token Parser::consume(TokenKind kind, const std::string& message) {
    if (check(kind)) return advance();
    throw std::runtime_error(message);
}

std::unique_ptr<Statement> Parser::statement() {
    if (match({TokenKind::Store})) {
        return assignment();
    }
    if (match({TokenKind::Define})) {
        return functionDef();
    }
    if (match({TokenKind::Return})) {
        return returnStmt();
    }
    if (match({TokenKind::If})) {
        return ifStmt();
    }
    if (match({TokenKind::While})) {
        return whileStmt();
    }
    if (match({TokenKind::For})) {
        return forStmt();
    }
    if (match({TokenKind::Repeat})) {
        return repeatStmt();
    }
    if (match({TokenKind::Match})) {
        return matchStmt();
    }
    if (match({TokenKind::Break})) {
        return breakStmt();
    }
    if (match({TokenKind::Continue})) {
        return continueStmt();
    }
    if (match({TokenKind::Try})) {
        return tryStmt();
    }
    if (match({TokenKind::Import})) {
        return importStmt();
    }
    
    // Expression statement
    auto expr = expression();
    auto stmt = std::make_unique<Statement>(StatementType::Expression, expr->span);
    stmt->data = std::move(expr);
    return stmt;
}

std::unique_ptr<Statement> Parser::assignment() {
    // Simplified - full implementation would parse: store → name → value
    consume(TokenKind::Arrow, "Expected '→' after 'store'");
    auto name_token = consume(TokenKind::Identifier, "Expected identifier");
    std::string name = std::get<std::string>(name_token.value.value);
    
    consume(TokenKind::Arrow, "Expected '→' after identifier");
    auto value = expression();
    
    auto stmt = std::make_unique<Statement>(StatementType::Assignment, name_token.span);
    AssignmentData data;
    data.name = name;
    data.value = std::move(value);
    stmt->data = std::move(data);
    
    return stmt;
}

std::unique_ptr<Statement> Parser::functionDef() {
    // Simplified implementation
    consume(TokenKind::Arrow, "Expected '→' after 'define'");
    auto name_token = consume(TokenKind::Identifier, "Expected function name");
    std::string name = std::get<std::string>(name_token.value.value);
    
    consume(TokenKind::Arrow, "Expected '→' after function name");
    consume(TokenKind::LeftParen, "Expected '('");
    auto params = parameters();
    consume(TokenKind::RightParen, "Expected ')'");
    
    auto body = block();
    consume(TokenKind::End, "Expected 'end'");
    
    auto stmt = std::make_unique<Statement>(StatementType::FunctionDef, name_token.span);
    FunctionDefData data;
    data.name = name;
    data.params = std::move(params);
    data.body = std::move(body);
    stmt->data = std::move(data);
    
    return stmt;
}

std::unique_ptr<Statement> Parser::returnStmt() {
    auto token = previous();
    std::unique_ptr<Expression> value = nullptr;
    
    if (!check(TokenKind::Newline) && !check(TokenKind::End)) {
        value = expression();
    }
    
    auto stmt = std::make_unique<Statement>(StatementType::Return, token.span);
    ReturnData data;
    data.value = std::move(value);
    stmt->data = std::move(data);
    
    return stmt;
}

std::unique_ptr<Statement> Parser::ifStmt() {
    // Simplified - full implementation would handle else-if chains
    auto token = previous();
    auto condition = expression();
    auto then_branch = block();
    
    std::vector<std::unique_ptr<Statement>> else_branch;
    if (match({TokenKind::Else})) {
        else_branch = block();
    }
    
    consume(TokenKind::End, "Expected 'end'");
    
    auto stmt = std::make_unique<Statement>(StatementType::If, token.span);
    IfData data;
    data.condition = std::move(condition);
    data.then_branch = std::move(then_branch);
    data.else_branch = std::move(else_branch);
    stmt->data = std::move(data);
    
    return stmt;
}

std::unique_ptr<Statement> Parser::whileStmt() {
    auto token = previous();
    auto condition = expression();
    auto body = block();
    consume(TokenKind::End, "Expected 'end'");
    
    auto stmt = std::make_unique<Statement>(StatementType::While, token.span);
    WhileData data;
    data.condition = std::move(condition);
    data.body = std::move(body);
    stmt->data = std::move(data);
    
    return stmt;
}

std::unique_ptr<Statement> Parser::forStmt() {
    auto token = previous();
    auto var_token = consume(TokenKind::Identifier, "Expected variable name");
    std::string var = std::get<std::string>(var_token.value.value);
    consume(TokenKind::In, "Expected 'in'");
    auto iterable = expression();
    auto body = block();
    consume(TokenKind::End, "Expected 'end'");
    
    auto stmt = std::make_unique<Statement>(StatementType::For, token.span);
    ForData data;
    data.variable = var;
    data.iterable = std::move(iterable);
    data.body = std::move(body);
    stmt->data = std::move(data);
    
    return stmt;
}

std::unique_ptr<Statement> Parser::repeatStmt() {
    auto token = previous();
    auto count = expression();
    consume(TokenKind::Times, "Expected 'times'");
    auto body = block();
    consume(TokenKind::End, "Expected 'end'");
    
    auto stmt = std::make_unique<Statement>(StatementType::Repeat, token.span);
    RepeatData data;
    data.count = std::move(count);
    data.body = std::move(body);
    stmt->data = std::move(data);
    
    return stmt;
}

std::unique_ptr<Statement> Parser::matchStmt() {
    // Simplified implementation
    auto token = previous();
    auto value = expression();
    std::vector<MatchCase> cases;
    // ... parse cases ...
    consume(TokenKind::End, "Expected 'end'");
    
    auto stmt = std::make_unique<Statement>(StatementType::Match, token.span);
    MatchData data;
    data.value = std::move(value);
    data.cases = std::move(cases);
    stmt->data = std::move(data);
    
    return stmt;
}

std::unique_ptr<Statement> Parser::breakStmt() {
    auto token = previous();
    return std::make_unique<Statement>(StatementType::Break, token.span);
}

std::unique_ptr<Statement> Parser::continueStmt() {
    auto token = previous();
    return std::make_unique<Statement>(StatementType::Continue, token.span);
}

std::unique_ptr<Statement> Parser::tryStmt() {
    // Simplified implementation
    auto token = previous();
    auto body = block();
    std::pair<std::string, std::vector<std::unique_ptr<Statement>>> catch_block;
    if (match({TokenKind::Catch})) {
        // Parse catch block
    }
    consume(TokenKind::End, "Expected 'end'");
    
    auto stmt = std::make_unique<Statement>(StatementType::Try, token.span);
    TryData data;
    data.body = std::move(body);
    data.catch_block = std::move(catch_block);
    stmt->data = std::move(data);
    
    return stmt;
}

std::unique_ptr<Statement> Parser::importStmt() {
    // Simplified implementation
    auto token = previous();
    auto stmt = std::make_unique<Statement>(StatementType::Import, token.span);
    ImportData data;
    stmt->data = std::move(data);
    return stmt;
}

std::unique_ptr<Expression> Parser::expression() {
    return assignmentExpr();
}

std::unique_ptr<Expression> Parser::assignmentExpr() {
    return orExpr();
}

std::unique_ptr<Expression> Parser::orExpr() {
    auto expr = andExpr();
    while (match({TokenKind::Or})) {
        auto op = previous().kind;
        auto right = andExpr();
        auto binary = std::make_unique<Expression>(ExpressionType::Binary, expr->span);
        BinaryData data;
        data.op = op;
        data.left = std::move(expr);
        data.right = std::move(right);
        binary->data = std::move(data);
        expr = std::move(binary);
    }
    return expr;
}

std::unique_ptr<Expression> Parser::andExpr() {
    auto expr = equality();
    while (match({TokenKind::And})) {
        auto op = previous().kind;
        auto right = equality();
        auto binary = std::make_unique<Expression>(ExpressionType::Binary, expr->span);
        BinaryData data;
        data.op = op;
        data.left = std::move(expr);
        data.right = std::move(right);
        binary->data = std::move(data);
        expr = std::move(binary);
    }
    return expr;
}

std::unique_ptr<Expression> Parser::equality() {
    auto expr = comparison();
    while (match({TokenKind::Equal, TokenKind::NotEqual})) {
        auto op = previous().kind;
        auto right = comparison();
        auto binary = std::make_unique<Expression>(ExpressionType::Binary, expr->span);
        BinaryData data;
        data.op = op;
        data.left = std::move(expr);
        data.right = std::move(right);
        binary->data = std::move(data);
        expr = std::move(binary);
    }
    return expr;
}

std::unique_ptr<Expression> Parser::comparison() {
    auto expr = term();
    while (match({TokenKind::Less, TokenKind::LessEqual, 
                  TokenKind::Greater, TokenKind::GreaterEqual})) {
        auto op = previous().kind;
        auto right = term();
        auto binary = std::make_unique<Expression>(ExpressionType::Binary, expr->span);
        BinaryData data;
        data.op = op;
        data.left = std::move(expr);
        data.right = std::move(right);
        binary->data = std::move(data);
        expr = std::move(binary);
    }
    return expr;
}

std::unique_ptr<Expression> Parser::term() {
    auto expr = factor();
    while (match({TokenKind::Plus, TokenKind::Minus})) {
        auto op = previous().kind;
        auto right = factor();
        auto binary = std::make_unique<Expression>(ExpressionType::Binary, expr->span);
        BinaryData data;
        data.op = op;
        data.left = std::move(expr);
        data.right = std::move(right);
        binary->data = std::move(data);
        expr = std::move(binary);
    }
    return expr;
}

std::unique_ptr<Expression> Parser::factor() {
    auto expr = unary();
    while (match({TokenKind::Star, TokenKind::Slash, TokenKind::Percent, TokenKind::Power})) {
        auto op = previous().kind;
        auto right = unary();
        auto binary = std::make_unique<Expression>(ExpressionType::Binary, expr->span);
        BinaryData data;
        data.op = op;
        data.left = std::move(expr);
        data.right = std::move(right);
        binary->data = std::move(data);
        expr = std::move(binary);
    }
    return expr;
}

std::unique_ptr<Expression> Parser::unary() {
    if (match({TokenKind::Not, TokenKind::Minus, TokenKind::BitNot})) {
        auto op = previous().kind;
        auto operand = unary();
        auto unary_expr = std::make_unique<Expression>(ExpressionType::Unary, operand->span);
        UnaryData data;
        data.op = op;
        data.operand = std::move(operand);
        unary_expr->data = std::move(data);
        return unary_expr;
    }
    return call();
}

std::unique_ptr<Expression> Parser::call() {
    auto expr = primary();
    
    while (true) {
        if (match({TokenKind::LeftParen})) {
            std::vector<std::unique_ptr<Expression>> arguments;
            if (!check(TokenKind::RightParen)) {
                do {
                    arguments.push_back(expression());
                } while (match({TokenKind::Comma}));
            }
            consume(TokenKind::RightParen, "Expected ')' after arguments");
            
            auto call_expr = std::make_unique<Expression>(ExpressionType::Call, expr->span);
            CallData data;
            data.callee = std::move(expr);
            data.arguments = std::move(arguments);
            call_expr->data = std::move(data);
            expr = std::move(call_expr);
        } else if (match({TokenKind::LeftBracket})) {
            auto index = expression();
            consume(TokenKind::RightBracket, "Expected ']' after index");
            
            auto index_expr = std::make_unique<Expression>(ExpressionType::Index, expr->span);
            IndexData data;
            data.object = std::move(expr);
            data.index = std::move(index);
            index_expr->data = std::move(data);
            expr = std::move(index_expr);
        } else if (match({TokenKind::Dot})) {
            auto member_token = consume(TokenKind::Identifier, "Expected property name");
            std::string member = std::get<std::string>(member_token.value.value);
            
            auto member_expr = std::make_unique<Expression>(ExpressionType::Member, expr->span);
            MemberData data;
            data.object = std::move(expr);
            data.member = member;
            member_expr->data = std::move(data);
            expr = std::move(member_expr);
        } else {
            break;
        }
    }
    
    return expr;
}

std::unique_ptr<Expression> Parser::primary() {
    if (match({TokenKind::Integer})) {
        auto token = previous();
        std::int64_t value = std::get<std::int64_t>(token.value.value);
        auto expr = std::make_unique<Expression>(ExpressionType::Literal, token.span);
        LiteralData data;
        data.value = value;
        expr->data = std::move(data);
        return expr;
    }
    
    if (match({TokenKind::Float})) {
        auto token = previous();
        double value = std::get<double>(token.value.value);
        auto expr = std::make_unique<Expression>(ExpressionType::Literal, token.span);
        LiteralData data;
        data.value = value;
        expr->data = std::move(data);
        return expr;
    }
    
    if (match({TokenKind::String})) {
        auto token = previous();
        std::string value = std::get<std::string>(token.value.value);
        auto expr = std::make_unique<Expression>(ExpressionType::Literal, token.span);
        LiteralData data;
        data.value = value;
        expr->data = std::move(data);
        return expr;
    }
    
    if (match({TokenKind::Boolean})) {
        auto token = previous();
        bool value = std::get<bool>(token.value.value);
        auto expr = std::make_unique<Expression>(ExpressionType::Literal, token.span);
        LiteralData data;
        data.value = value;
        expr->data = std::move(data);
        return expr;
    }
    
    if (match({TokenKind::Null})) {
        auto token = previous();
        auto expr = std::make_unique<Expression>(ExpressionType::Literal, token.span);
        LiteralData data;
        data.value = std::nullptr_t{};
        expr->data = std::move(data);
        return expr;
    }
    
    if (match({TokenKind::Identifier})) {
        auto token = previous();
        std::string name = std::get<std::string>(token.value.value);
        auto expr = std::make_unique<Expression>(ExpressionType::Identifier, token.span);
        expr->data = name;
        return expr;
    }
    
    if (match({TokenKind::LeftParen})) {
        auto expr = expression();
        consume(TokenKind::RightParen, "Expected ')' after expression");
        return expr;
    }
    
    throw std::runtime_error("Expected expression");
}

std::vector<std::unique_ptr<Statement>> Parser::block() {
    std::vector<std::unique_ptr<Statement>> statements;
    
    while (!isAtEnd() && !check(TokenKind::End) && 
           !check(TokenKind::Else) && !check(TokenKind::Catch)) {
        statements.push_back(statement());
    }
    
    return statements;
}

std::vector<Parameter> Parser::parameters() {
    std::vector<Parameter> params;
    
    if (!check(TokenKind::RightParen)) {
        do {
            auto name_token = consume(TokenKind::Identifier, "Expected parameter name");
            std::string name = std::get<std::string>(name_token.value.value);
            
            Parameter param;
            param.name = name;
            param.span = name_token.span;
            
            if (match({TokenKind::Colon})) {
                param.type_annotation = typeAnnotation();
            }
            
            params.push_back(std::move(param));
        } while (match({TokenKind::Comma}));
    }
    
    return params;
}

std::unique_ptr<Type> Parser::typeAnnotation() {
    // Simplified type parsing
    auto type = std::make_unique<Type>();
    // ... parse type ...
    return type;
}

Pattern Parser::pattern() {
    // Simplified pattern parsing
    Pattern p;
    p.type = Pattern::PatternType::Wildcard;
    return p;
}

} // namespace txtcode

