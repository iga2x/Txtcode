use crate::lexer::token::{Token, TokenKind};
use crate::parser::ast::{Expression, Pattern, Span};

/// Trait for parser methods needed by statement and expression parsers
pub trait ParserTrait {
    fn peek(&self) -> &Token;
    fn previous(&self) -> &Token;
    fn advance(&mut self) -> &Token;
    fn is_at_end(&self) -> bool;
    fn check(&self, kind: TokenKind) -> bool;
    fn check_keyword(&self, keyword: &str) -> bool;
    fn expect(&mut self, kind: TokenKind, msg: &str) -> Result<&Token, String>;
    fn expect_keyword(&mut self, keyword: &str) -> Result<&Token, String>;
    fn expect_arrow(&mut self) -> Result<&Token, String>;
    fn expect_identifier(&mut self) -> Result<String, String>;
    fn error<T>(&self, msg: &str) -> Result<T, String>;
    fn error_with_context<T>(&self, msg: &str, context: &str) -> Result<T, String>;
    fn parse_expression(&mut self) -> Result<Expression, String>;
    fn parse_pattern(&mut self) -> Result<Pattern, String>;
    fn token_span_to_ast_span(token: &Token) -> Span;
}

