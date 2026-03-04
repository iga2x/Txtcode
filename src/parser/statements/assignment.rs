use crate::lexer::token::Token;
use crate::parser::ast::*;
use crate::parser::parser::Parser;

/// Parse assignment statement (store)
pub fn parse_store(parser: &mut Parser) -> Result<Option<Statement>, String> {
    let start_token = parser.peek().clone();
    parser.expect_keyword("store")?;
    
    // Optional arrow after 'store' (for compatibility with both syntaxes)
    // Standard: store name -> value
    // Alternative: store -> name -> value
    if parser.check(crate::lexer::token::TokenKind::Arrow) {
        parser.advance();
        // Skip whitespace after optional arrow
        while !parser.is_at_end() && 
              (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
               parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
            parser.advance();
        }
    }
    
    // Check if this is an index assignment: store → target[key] → value
    // Peek ahead: identifier followed by '[' means index assignment
    let is_index_assign = {
        let pos = parser.position;
        let mut p = pos;
        // skip whitespace
        while p < parser.tokens.len() && matches!(parser.tokens[p].kind,
            crate::lexer::token::TokenKind::Whitespace | crate::lexer::token::TokenKind::Newline) {
            p += 1;
        }
        // check identifier
        let is_ident = p < parser.tokens.len() && parser.tokens[p].kind == crate::lexer::token::TokenKind::Identifier;
        if is_ident {
            p += 1;
            // skip whitespace
            while p < parser.tokens.len() && matches!(parser.tokens[p].kind,
                crate::lexer::token::TokenKind::Whitespace | crate::lexer::token::TokenKind::Newline) {
                p += 1;
            }
            p < parser.tokens.len() && parser.tokens[p].kind == crate::lexer::token::TokenKind::LeftBracket
        } else {
            false
        }
    };

    if is_index_assign {
        // Parse: identifier[key] → value
        let obj_name = parser.expect_identifier()?;
        let target = Expression::Identifier(obj_name);
        parser.expect(crate::lexer::token::TokenKind::LeftBracket)?;
        let index = parser.parse_expression()?;
        parser.expect(crate::lexer::token::TokenKind::RightBracket)?;
        parser.skip_optional_arrow();
        let value = parser.parse_expression()?;
        let span = token_span_to_ast_span(&start_token);
        return Ok(Some(Statement::IndexAssignment { target, index, value, span }));
    }

    // Parse pattern (identifier, array, or struct)
    let pattern = parser.parse_pattern()?;
    parser.skip_optional_arrow(); // Optional arrow before value
    let value = parser.parse_expression()?;
    let span = token_span_to_ast_span(&start_token);
    Ok(Some(Statement::Assignment {
        pattern,
        type_annotation: None,
        value,
        span,
    }))
}

/// Parse const statement
pub fn parse_const(parser: &mut Parser) -> Result<Option<Statement>, String> {
    let start_token = parser.peek().clone();
    parser.expect_keyword("const")?;
    parser.skip_optional_arrow(); // Optional arrow after const
    let name = parser.expect_identifier()?;
    parser.skip_optional_arrow(); // Optional arrow before value
    let value = parser.parse_expression()?;
    let span = token_span_to_ast_span(&start_token);
    Ok(Some(Statement::Const {
        name,
        value,
        span,
    }))
}

/// Parse print statement
pub fn parse_print(parser: &mut Parser) -> Result<Option<Statement>, String> {
    let start_token = parser.peek().clone();
    parser.expect_keyword("print")?;
    parser.skip_optional_arrow(); // Optional arrow before value
    let value = parser.parse_expression()?;
    let span = token_span_to_ast_span(&start_token);
    // Print is implemented as a function call to stdlib
    Ok(Some(Statement::Expression(Expression::FunctionCall {
        name: "print".to_string(),
        type_arguments: None,
        arguments: vec![value],
        span,
    })))
}

fn token_span_to_ast_span(token: &Token) -> Span {
    Span {
        start: token.span.1,
        end: token.span.1,
        line: token.span.0,
        column: token.span.1,
    }
}

