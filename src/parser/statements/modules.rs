use crate::parser::ast::*;
use crate::parser::parser::Parser;
use crate::parser::utils::token_span_to_ast_span;

pub fn parse_import(parser: &mut Parser) -> Result<Option<Statement>, String> {
    let start_token = parser.peek().clone();
    parser.expect_keyword("import")?;
    parser.skip_optional_arrow(); // Optional arrow after import

    let mut modules = Vec::new();
    loop {
        let module_name = parser.expect_identifier()?;
        modules.push(module_name);

        if parser.check(crate::lexer::token::TokenKind::Comma) {
            parser.advance();
        } else {
            break;
        }
    }

    let mut from = None;
    if parser.check_keyword("from") {
        parser.advance();
        parser.skip_optional_arrow(); // Optional arrow after from
        if parser.check(crate::lexer::token::TokenKind::String) {
            from = Some(parser.peek().value.clone());
            parser.advance();
        } else {
            from = Some(parser.expect_identifier()?);
        }
    }

    let mut alias = None;
    if parser.check_keyword("as") {
        parser.advance();
        parser.skip_optional_arrow(); // Optional arrow after as
        alias = Some(parser.expect_identifier()?);
    }

    let span = token_span_to_ast_span(&start_token);
    Ok(Some(Statement::Import {
        modules,
        from,
        alias,
        span,
    }))
}

pub fn parse_export(parser: &mut Parser) -> Result<Option<Statement>, String> {
    let start_token = parser.peek().clone();
    parser.expect_keyword("export")?;
    parser.skip_optional_arrow(); // Optional arrow after export

    let mut names = Vec::new();
    loop {
        let name = parser.expect_identifier()?;
        names.push(name);

        if parser.check(crate::lexer::token::TokenKind::Comma) {
            parser.advance();
        } else {
            break;
        }
    }

    let span = token_span_to_ast_span(&start_token);
    Ok(Some(Statement::Export { names, span }))
}
