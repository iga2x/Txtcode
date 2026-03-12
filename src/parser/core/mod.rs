use crate::lexer::token::{Token, TokenKind};
use crate::parser::ast::Span;

/// Core parser functionality - token management and error handling
/// This is a placeholder for future refactoring - not currently used
#[allow(dead_code)]
pub struct ParserCore {
    pub tokens: Vec<Token>,
    pub position: usize,
}

#[allow(dead_code)]
impl ParserCore {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    pub fn peek(&self) -> &Token {
        &self.tokens[self.position]
    }

    pub fn previous(&self) -> &Token {
        &self.tokens[self.position - 1]
    }

    pub fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.position += 1;
        }
        self.previous()
    }

    pub fn is_at_end(&self) -> bool {
        self.position >= self.tokens.len()
    }

    pub fn check(&self, kind: TokenKind) -> bool {
        !self.is_at_end() && self.peek().kind == kind
    }

    pub fn check_keyword(&self, keyword: &str) -> bool {
        self.check(TokenKind::Keyword) && self.peek().value == keyword
    }

    pub fn expect(&mut self, kind: TokenKind, msg: &str) -> Result<&Token, String> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            self.error(msg)
        }
    }

    pub fn expect_keyword(&mut self, keyword: &str) -> Result<&Token, String> {
        if self.check_keyword(keyword) {
            Ok(self.advance())
        } else {
            self.error(&format!("Expected keyword '{}'", keyword))
        }
    }

    pub fn expect_arrow(&mut self) -> Result<&Token, String> {
        self.expect(TokenKind::Arrow, "Expected '->'")
    }

    #[allow(dead_code)]
    pub fn error<T>(&self, msg: &str) -> Result<T, String> {
        let token = self.peek();
        let line = token.span.0;
        let column = token.span.1;
        Err(format!(
            "Parse error at line {}, column {}: {} (found {:?})",
            line, column, msg, token.kind
        ))
    }

    #[allow(dead_code)]
    pub fn error_with_context<T>(&self, msg: &str, context: &str) -> Result<T, String> {
        let token = self.peek();
        let line = token.span.0;
        let column = token.span.1;
        Err(format!(
            "Parse error at line {}, column {}: {} {} (found {:?} '{}')",
            line, column, msg, context, token.kind, token.value
        ))
    }

    pub fn token_span_to_ast_span(token: &Token) -> Span {
        Span {
            start: token.span.1,
            end: token.span.1,
            line: token.span.0,
            column: token.span.1,
        }
    }
}
