use crate::parser::ast::*;
use crate::parser::parser::Parser;
use crate::parser::utils::token_span_to_ast_span;

pub fn parse_permission(parser: &mut Parser) -> Result<Option<Statement>, String> {
    let start_token = parser.peek().clone();
    parser.expect_keyword("permission")?;
    parser.expect_arrow()?;

    // Parse resource.action (e.g., "fs.read")
    let resource = parser.expect_identifier()?; // "fs"
    parser.expect(crate::lexer::token::TokenKind::Dot)?;
    let action = parser.expect_identifier()?; // "read"

    // Optional scope after arrow
    let scope = if parser.check(crate::lexer::token::TokenKind::Arrow) {
        parser.advance();
        // Can be string or identifier
        if parser.check(crate::lexer::token::TokenKind::String) {
            let value = parser.peek().value.clone();
            parser.advance();
            Some(value)
        } else {
            Some(parser.expect_identifier()?)
        }
    } else {
        None
    };

    let span = token_span_to_ast_span(&start_token);
    Ok(Some(Statement::Permission {
        resource,
        action,
        scope,
        span,
    }))
}
