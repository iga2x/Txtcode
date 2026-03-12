use crate::parser::ast::*;
use crate::parser::expressions::primary;
use crate::parser::parser::Parser;
use crate::parser::utils::token_span_to_ast_span;

pub fn parse_call(parser: &mut Parser) -> Result<Expression, String> {
    let mut expr = primary::parse_primary(parser)?;

    loop {
        // Check for generic type arguments: func<T, U>(...)
        // Only parse if followed by '(' to avoid conflicts with comparison operators
        let type_args = if parser.check(crate::lexer::token::TokenKind::Less) {
            // Look ahead to see if this is a generic call pattern: <T> ( or <T, U> (
            let start_pos = parser.position;
            let mut pos = start_pos + 1; // skip '<'
            let mut found_generic = false;

            // Skip whitespace after '<'
            while pos < parser.tokens.len()
                && (parser.tokens[pos].kind == crate::lexer::token::TokenKind::Whitespace
                    || parser.tokens[pos].kind == crate::lexer::token::TokenKind::Newline)
            {
                pos += 1;
            }

            // Check if we have at least one type name
            if pos < parser.tokens.len() {
                let token = &parser.tokens[pos];
                if token.kind == crate::lexer::token::TokenKind::Identifier
                    || (token.kind == crate::lexer::token::TokenKind::Keyword
                        && crate::lexer::keywords::is_type_keyword(&token.value))
                {
                    // Found potential type, continue scanning
                    pos += 1;

                    // Skip whitespace
                    while pos < parser.tokens.len()
                        && (parser.tokens[pos].kind == crate::lexer::token::TokenKind::Whitespace
                            || parser.tokens[pos].kind == crate::lexer::token::TokenKind::Newline)
                    {
                        pos += 1;
                    }

                    // Look for comma (more types) or '>'
                    while pos < parser.tokens.len() {
                        if parser.tokens[pos].kind == crate::lexer::token::TokenKind::Comma {
                            pos += 1;
                            // Skip whitespace after comma
                            while pos < parser.tokens.len()
                                && (parser.tokens[pos].kind
                                    == crate::lexer::token::TokenKind::Whitespace
                                    || parser.tokens[pos].kind
                                        == crate::lexer::token::TokenKind::Newline)
                            {
                                pos += 1;
                            }
                            // Check for another type
                            if pos < parser.tokens.len()
                                && (parser.tokens[pos].kind
                                    == crate::lexer::token::TokenKind::Identifier
                                    || (parser.tokens[pos].kind
                                        == crate::lexer::token::TokenKind::Keyword
                                        && crate::lexer::keywords::is_type_keyword(
                                            &parser.tokens[pos].value,
                                        )))
                            {
                                pos += 1;
                                // Skip whitespace after type
                                while pos < parser.tokens.len()
                                    && (parser.tokens[pos].kind
                                        == crate::lexer::token::TokenKind::Whitespace
                                        || parser.tokens[pos].kind
                                            == crate::lexer::token::TokenKind::Newline)
                                {
                                    pos += 1;
                                }
                            } else {
                                break; // Invalid generic syntax
                            }
                        } else if parser.tokens[pos].kind == crate::lexer::token::TokenKind::Greater
                        {
                            pos += 1;
                            // Skip whitespace after '>'
                            while pos < parser.tokens.len()
                                && (parser.tokens[pos].kind
                                    == crate::lexer::token::TokenKind::Whitespace
                                    || parser.tokens[pos].kind
                                        == crate::lexer::token::TokenKind::Newline)
                            {
                                pos += 1;
                            }
                            // Check if followed by '('
                            if pos < parser.tokens.len()
                                && parser.tokens[pos].kind
                                    == crate::lexer::token::TokenKind::LeftParen
                            {
                                found_generic = true;
                            }
                            break;
                        } else {
                            break; // Not a valid generic pattern
                        }
                    }
                }
            }

            if found_generic {
                // Parse the generic type arguments
                parser.advance(); // consume '<'
                let mut types = Vec::new();

                loop {
                    // Skip whitespace
                    while !parser.is_at_end()
                        && (parser.peek().kind == crate::lexer::token::TokenKind::Whitespace
                            || parser.peek().kind == crate::lexer::token::TokenKind::Newline)
                    {
                        parser.advance();
                    }

                    let type_name =
                        parser.expect_identifier_allow_reserved(true).map_err(|_| {
                            let token = parser.peek();
                            format!(
                                "Expected type name in generic type arguments at line {}:{}",
                                token.span.0, token.span.1
                            )
                        })?;
                    types.push(parser.type_keyword_to_type(&type_name));

                    // Skip whitespace
                    while !parser.is_at_end()
                        && (parser.peek().kind == crate::lexer::token::TokenKind::Whitespace
                            || parser.peek().kind == crate::lexer::token::TokenKind::Newline)
                    {
                        parser.advance();
                    }

                    if parser.check(crate::lexer::token::TokenKind::Comma) {
                        parser.advance();
                    } else {
                        break;
                    }
                }

                parser.expect(crate::lexer::token::TokenKind::Greater)?;
                // Skip whitespace after '>'
                while !parser.is_at_end()
                    && (parser.peek().kind == crate::lexer::token::TokenKind::Whitespace
                        || parser.peek().kind == crate::lexer::token::TokenKind::Newline)
                {
                    parser.advance();
                }
                Some(types)
            } else {
                None
            }
        } else {
            None
        };

        // Check for struct literal: StructName { field: value, ... }
        // Only when expr is an Identifier, next is '{', and lookahead shows 'ident:'
        if parser.check(crate::lexer::token::TokenKind::LeftBrace) {
            if let Expression::Identifier(ref struct_name) = expr {
                // Lookahead: is this { identifier : ... }? (struct literal vs map/set/block)
                let brace_pos = parser.position; // points to '{'
                let mut peek = brace_pos + 1;
                // skip whitespace/newlines
                while peek < parser.tokens.len()
                    && matches!(
                        parser.tokens[peek].kind,
                        crate::lexer::token::TokenKind::Whitespace
                            | crate::lexer::token::TokenKind::Newline
                    )
                {
                    peek += 1;
                }
                let is_struct_literal = {
                    // Empty struct: { }
                    let is_empty = peek < parser.tokens.len()
                        && parser.tokens[peek].kind == crate::lexer::token::TokenKind::RightBrace;
                    // Non-empty struct: { identifier : ... }
                    let has_field = peek + 1 < parser.tokens.len()
                        && parser.tokens[peek].kind == crate::lexer::token::TokenKind::Identifier
                        && parser.tokens[peek + 1].kind == crate::lexer::token::TokenKind::Colon;
                    is_empty || has_field
                };
                if is_struct_literal {
                    let name = struct_name.clone();
                    parser.advance(); // consume '{'
                                      // skip whitespace
                    while !parser.is_at_end()
                        && matches!(
                            parser.peek().kind,
                            crate::lexer::token::TokenKind::Whitespace
                                | crate::lexer::token::TokenKind::Newline
                        )
                    {
                        parser.advance();
                    }
                    let mut fields = Vec::new();
                    while !parser.check(crate::lexer::token::TokenKind::RightBrace)
                        && !parser.is_at_end()
                    {
                        let field_name = parser.expect_identifier()?;
                        parser.expect(crate::lexer::token::TokenKind::Colon)?;
                        let value =
                            crate::parser::expressions::operators::parse_expression(parser)?;
                        fields.push((field_name, value));
                        // skip whitespace
                        while !parser.is_at_end()
                            && matches!(
                                parser.peek().kind,
                                crate::lexer::token::TokenKind::Whitespace
                                    | crate::lexer::token::TokenKind::Newline
                            )
                        {
                            parser.advance();
                        }
                        if parser.check(crate::lexer::token::TokenKind::Comma) {
                            parser.advance();
                            // skip whitespace after comma
                            while !parser.is_at_end()
                                && matches!(
                                    parser.peek().kind,
                                    crate::lexer::token::TokenKind::Whitespace
                                        | crate::lexer::token::TokenKind::Newline
                                )
                            {
                                parser.advance();
                            }
                        } else {
                            break;
                        }
                    }
                    parser.expect(crate::lexer::token::TokenKind::RightBrace)?;
                    expr = Expression::StructLiteral {
                        name,
                        fields,
                        span: Span::default(),
                    };
                    continue;
                }
            }
        }

        if parser.check(crate::lexer::token::TokenKind::LeftParen) {
            expr = finish_call(parser, expr, type_args)?;
        } else if parser.check(crate::lexer::token::TokenKind::LeftBracket) {
            parser.advance();
            // Check if it's a slice (starts with colon or has colon after expression)
            if parser.check(crate::lexer::token::TokenKind::Colon) {
                // Slice starting with colon: [:end:step]
                parser.advance(); // consume colon
                let end = if parser.check(crate::lexer::token::TokenKind::Colon)
                    || parser.check(crate::lexer::token::TokenKind::RightBracket)
                {
                    None
                } else if parser.check_keyword("end") {
                    // Special 'end' keyword means "to the end" (same as omitting)
                    parser.advance();
                    None
                } else {
                    Some(Box::new(
                        crate::parser::expressions::operators::parse_expression(parser)?,
                    ))
                };

                let step = if parser.check(crate::lexer::token::TokenKind::Colon) {
                    parser.advance();
                    if parser.check(crate::lexer::token::TokenKind::RightBracket) {
                        None
                    } else {
                        Some(Box::new(
                            crate::parser::expressions::operators::parse_expression(parser)?,
                        ))
                    }
                } else {
                    None
                };

                parser.expect(crate::lexer::token::TokenKind::RightBracket)?;
                expr = Expression::Slice {
                    target: Box::new(expr),
                    start: None,
                    end,
                    step,
                    span: Span::default(),
                };
            } else {
                // Parse first expression (could be start index or just index)
                let first_expr = crate::parser::expressions::operators::parse_expression(parser)?;

                if parser.check(crate::lexer::token::TokenKind::Colon) {
                    // It's a slice: [start:end:step]
                    parser.advance(); // consume colon
                    let start = Some(Box::new(first_expr));

                    let end = if parser.check(crate::lexer::token::TokenKind::Colon)
                        || parser.check(crate::lexer::token::TokenKind::RightBracket)
                    {
                        None
                    } else if parser.check_keyword("end") {
                        // Special 'end' keyword means "to the end" (same as omitting)
                        parser.advance();
                        None
                    } else {
                        Some(Box::new(
                            crate::parser::expressions::operators::parse_expression(parser)?,
                        ))
                    };

                    let step = if parser.check(crate::lexer::token::TokenKind::Colon) {
                        parser.advance();
                        if parser.check(crate::lexer::token::TokenKind::RightBracket) {
                            None
                        } else {
                            Some(Box::new(
                                crate::parser::expressions::operators::parse_expression(parser)?,
                            ))
                        }
                    } else {
                        None
                    };

                    parser.expect(crate::lexer::token::TokenKind::RightBracket)?;
                    expr = Expression::Slice {
                        target: Box::new(expr),
                        start,
                        end,
                        step,
                        span: Span::default(),
                    };
                } else {
                    // Regular index
                    parser.expect(crate::lexer::token::TokenKind::RightBracket)?;
                    expr = Expression::Index {
                        target: Box::new(expr),
                        index: Box::new(first_expr),
                        span: Span::default(),
                    };
                }
            }
        } else if parser.check(crate::lexer::token::TokenKind::Dot) {
            parser.advance();
            let name = parser.expect_identifier()?;
            expr = Expression::Member {
                target: Box::new(expr),
                name,
                span: Span::default(),
            };
        } else if parser.check(crate::lexer::token::TokenKind::OptionalChain) {
            // Optional chaining: obj?.member, obj?.(args), obj?.[index]
            parser.advance(); // consume ?.
            let token = parser.peek().clone();
            let span = token_span_to_ast_span(&token);

            // Check what comes after ?.
            if parser.check(crate::lexer::token::TokenKind::LeftParen) {
                // Optional call: obj?.(args)
                parser.expect(crate::lexer::token::TokenKind::LeftParen)?;
                let mut arguments = Vec::new();
                if !parser.check(crate::lexer::token::TokenKind::RightParen) {
                    loop {
                        arguments.push(crate::parser::expressions::operators::parse_expression(
                            parser,
                        )?);
                        if !parser.check(crate::lexer::token::TokenKind::Comma) {
                            break;
                        }
                        parser.advance();
                    }
                }
                parser.expect(crate::lexer::token::TokenKind::RightParen)?;
                expr = Expression::OptionalCall {
                    target: Box::new(expr),
                    arguments,
                    span,
                };
            } else if parser.check(crate::lexer::token::TokenKind::LeftBracket) {
                // Optional index: obj?.[index]
                parser.advance();
                let index = crate::parser::expressions::operators::parse_expression(parser)?;
                parser.expect(crate::lexer::token::TokenKind::RightBracket)?;
                expr = Expression::OptionalIndex {
                    target: Box::new(expr),
                    index: Box::new(index),
                    span,
                };
            } else {
                // Optional member: obj?.member
                let name = parser.expect_identifier()?;
                expr = Expression::OptionalMember {
                    target: Box::new(expr),
                    name,
                    span,
                };
            }
        } else {
            break;
        }
    }

    Ok(expr)
}

fn finish_call(
    parser: &mut Parser,
    callee: Expression,
    type_args: Option<Vec<crate::typecheck::types::Type>>,
) -> Result<Expression, String> {
    // Handle method calls on complex expressions (e.g., arr[0].trim(), map["k"].split(","))
    if let Expression::Member {
        target,
        name: method_name,
        span,
    } = &callee
    {
        if !matches!(target.as_ref(), Expression::Identifier(_)) {
            // Complex expression method call — use MethodCall node
            let obj_expr = target.as_ref().clone();
            parser.expect(crate::lexer::token::TokenKind::LeftParen)?;
            let mut arguments = Vec::new();
            while !parser.check(crate::lexer::token::TokenKind::RightParen) && !parser.is_at_end() {
                arguments.push(crate::parser::expressions::operators::parse_expression(
                    parser,
                )?);
                if parser.check(crate::lexer::token::TokenKind::Comma) {
                    parser.advance();
                } else {
                    break;
                }
            }
            parser.expect(crate::lexer::token::TokenKind::RightParen)?;
            return Ok(Expression::MethodCall {
                object: Box::new(obj_expr),
                method: method_name.clone(),
                type_arguments: type_args,
                arguments,
                span: span.clone(),
            });
        }
    }

    // Extract function name from callee
    let function_name = match &callee {
        Expression::Identifier(name) => name.clone(),
        Expression::Member { target, name, .. } => {
            // For member calls like obj.method(), construct the full name
            let obj_name = match target.as_ref() {
                Expression::Identifier(obj) => obj.clone(),
                _ => "obj".to_string(),
            };
            format!("{}.{}", obj_name, name)
        }
        _ => return parser.error("Function call must be on an identifier or member"),
    };

    parser.expect(crate::lexer::token::TokenKind::LeftParen)?;

    let mut arguments = Vec::new();
    if !parser.check(crate::lexer::token::TokenKind::RightParen) {
        loop {
            arguments.push(crate::parser::expressions::operators::parse_expression(
                parser,
            )?);
            if !parser.check(crate::lexer::token::TokenKind::Comma) {
                break;
            }
            parser.advance();
        }
    }

    parser.expect(crate::lexer::token::TokenKind::RightParen)?;

    Ok(Expression::FunctionCall {
        name: function_name,
        type_arguments: type_args,
        arguments,
        span: Span::default(),
    })
}
