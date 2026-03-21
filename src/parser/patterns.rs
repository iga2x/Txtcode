use crate::parser::ast::{Expression, Literal, Pattern, Span};
use crate::parser::parser::Parser;

pub fn parse_pattern(parser: &mut Parser) -> Result<Pattern, String> {
    let first = parse_single_pattern(parser)?;

    // Or-pattern: first_pat | second_pat | ...
    if parser.check(crate::lexer::token::TokenKind::BitOr) {
        let mut pats = vec![first];
        while parser.check(crate::lexer::token::TokenKind::BitOr) {
            parser.advance(); // consume '|'
            pats.push(parse_single_pattern(parser)?);
        }
        return Ok(Pattern::Or(pats));
    }

    Ok(first)
}

fn parse_single_pattern(parser: &mut Parser) -> Result<Pattern, String> {
    // Check for literal patterns: integers, floats, strings, booleans, null
    let token = parser.peek().clone();

    // Handle literal value patterns — also check for range `lit..=lit`
    if matches!(
        token.kind,
        crate::lexer::token::TokenKind::Integer
            | crate::lexer::token::TokenKind::Float
    ) {
        let value = token.value.clone();
        let tok_kind = token.kind;
        parser.advance();

        // Check for range inclusive: 1..=5
        if parser.check(crate::lexer::token::TokenKind::RangeInclusive) {
            parser.advance(); // consume ..=
            let end_tok = parser.peek().clone();
            if matches!(
                end_tok.kind,
                crate::lexer::token::TokenKind::Integer | crate::lexer::token::TokenKind::Float
            ) {
                let end_val = end_tok.value.clone();
                parser.advance();
                let start_expr = if tok_kind == crate::lexer::token::TokenKind::Integer {
                    Expression::Literal(Literal::Integer(
                        value.parse::<i64>().map_err(|_| format!("Invalid integer '{}'", value))?,
                    ))
                } else {
                    Expression::Literal(Literal::Float(
                        value.parse::<f64>().map_err(|_| format!("Invalid float '{}'", value))?,
                    ))
                };
                let end_expr = if end_tok.kind == crate::lexer::token::TokenKind::Integer {
                    Expression::Literal(Literal::Integer(
                        end_val.parse::<i64>().map_err(|_| format!("Invalid integer '{}'", end_val))?,
                    ))
                } else {
                    Expression::Literal(Literal::Float(
                        end_val.parse::<f64>().map_err(|_| format!("Invalid float '{}'", end_val))?,
                    ))
                };
                return Ok(Pattern::Range(Box::new(start_expr), Box::new(end_expr)));
            } else {
                return Err(format!(
                    "Expected number after '..=' in range pattern at line {}:{}",
                    end_tok.span.0, end_tok.span.1
                ));
            }
        }
        return Ok(Pattern::Identifier(format!("__literal_{}", value)));
    }

    if matches!(
        token.kind,
        crate::lexer::token::TokenKind::String | crate::lexer::token::TokenKind::Char
    ) {
        let value = token.value.clone();
        parser.advance();
        return Ok(Pattern::Identifier(format!("__literal_{}", value)));
    }

    // Handle boolean/null literal keywords
    if token.kind == crate::lexer::token::TokenKind::Keyword
        && (token.value == "true" || token.value == "false" || token.value == "null")
    {
        let value = token.value.clone();
        parser.advance();
        return Ok(Pattern::Identifier(format!("__literal_{}", value)));
    }

    // Check for array pattern: [a, b, c] or [head, ...rest]
    if parser.check(crate::lexer::token::TokenKind::LeftBracket) {
        parser.advance();
        let mut patterns = Vec::new();

        if !parser.check(crate::lexer::token::TokenKind::RightBracket) {
            loop {
                // Check for rest pattern: ...name (must be last element)
                if parser.check(crate::lexer::token::TokenKind::Spread) {
                    parser.advance(); // consume `...`
                    let rest_name = parser.expect_identifier()?;
                    patterns.push(Pattern::Rest(rest_name));
                    // rest must be the last element — skip optional trailing comma then stop
                    if parser.check(crate::lexer::token::TokenKind::Comma) {
                        parser.advance();
                    }
                    break;
                }
                patterns.push(parse_pattern(parser)?);
                if parser.check(crate::lexer::token::TokenKind::Comma) {
                    parser.advance();
                } else {
                    break;
                }
            }
        }

        parser.expect(crate::lexer::token::TokenKind::RightBracket)?;
        return Ok(Pattern::Array(patterns));
    }

    // Check for struct pattern: {x, y} or {x: a, y: b}
    if parser.check(crate::lexer::token::TokenKind::LeftBrace) {
        parser.advance();
        let mut fields = Vec::new();
        let mut rest = None;

        if !parser.check(crate::lexer::token::TokenKind::RightBrace) {
            loop {
                // Check for rest pattern: ...rest
                if parser.check(crate::lexer::token::TokenKind::Dot) {
                    let saved_pos = parser.position;
                    parser.advance();
                    if parser.check(crate::lexer::token::TokenKind::Dot) {
                        parser.advance();
                        if parser.check(crate::lexer::token::TokenKind::Dot) {
                            parser.advance();
                            let rest_name = parser.expect_identifier()?;
                            rest = Some(rest_name);
                        } else {
                            parser.position = saved_pos;
                            return Err("Invalid pattern syntax".to_string());
                        }
                    } else {
                        parser.position = saved_pos;
                        return Err("Invalid pattern syntax".to_string());
                    }
                } else {
                    let field_name = parser.expect_identifier()?;

                    // Check for field: pattern syntax (e.g., x: a)
                    if parser.check(crate::lexer::token::TokenKind::Colon) {
                        parser.advance();
                        let pattern = parse_pattern(parser)?;
                        fields.push((field_name, pattern));
                    } else {
                        // Shorthand: {x} means {x: x}
                        fields.push((field_name.clone(), Pattern::Identifier(field_name)));
                    }
                }

                if parser.check(crate::lexer::token::TokenKind::Comma) {
                    parser.advance();
                } else {
                    break;
                }
            }
        }

        parser.expect(crate::lexer::token::TokenKind::RightBrace)?;
        return Ok(Pattern::Struct { fields, rest });
    }

    // Check for ignore pattern: _
    if parser.check_keyword("_") {
        parser.advance();
        return Ok(Pattern::Ignore);
    }

    // Check for struct constructor pattern: Point(10, 20) or Point(x, y)
    // This must come before enum member access check
    if parser.check(crate::lexer::token::TokenKind::Identifier) {
        let type_name = parser.expect_identifier()?;

        // Check if there's a left paren (constructor call syntax)
        if parser.check(crate::lexer::token::TokenKind::LeftParen) {
            parser.advance(); // consume '('

            // Skip whitespace
            while !parser.is_at_end()
                && (parser.peek().kind == crate::lexer::token::TokenKind::Newline
                    || parser.peek().kind == crate::lexer::token::TokenKind::Whitespace)
            {
                parser.advance();
            }

            let mut args = Vec::new();

            // Parse constructor arguments
            if !parser.check(crate::lexer::token::TokenKind::RightParen) {
                loop {
                    args.push(parse_pattern(parser)?);

                    // Skip whitespace after argument
                    while !parser.is_at_end()
                        && (parser.peek().kind == crate::lexer::token::TokenKind::Newline
                            || parser.peek().kind == crate::lexer::token::TokenKind::Whitespace)
                    {
                        parser.advance();
                    }

                    if !parser.check(crate::lexer::token::TokenKind::Comma) {
                        break;
                    }
                    parser.advance();

                    // Skip whitespace after comma
                    while !parser.is_at_end()
                        && (parser.peek().kind == crate::lexer::token::TokenKind::Newline
                            || parser.peek().kind == crate::lexer::token::TokenKind::Whitespace)
                    {
                        parser.advance();
                    }
                }
            }

            parser.expect(crate::lexer::token::TokenKind::RightParen)?;
            return Ok(Pattern::Constructor { type_name, args });
        }

        // Check if there's a dot followed by a variant name (enum member access)
        if parser.check(crate::lexer::token::TokenKind::Dot) {
            parser.advance();

            // Skip whitespace after dot
            while !parser.is_at_end()
                && (parser.peek().kind == crate::lexer::token::TokenKind::Newline
                    || parser.peek().kind == crate::lexer::token::TokenKind::Whitespace)
            {
                parser.advance();
            }

            // Parse variant name (can be a keyword like Red, Green, Blue, etc.)
            let variant_name = if parser.check(crate::lexer::token::TokenKind::Identifier) {
                parser.expect_identifier()?
            } else if parser.peek().kind == crate::lexer::token::TokenKind::Keyword {
                // Keywords can be enum variant names
                let name = parser.peek().value.clone();
                parser.advance();
                name
            } else {
                let token = parser.peek();
                return Err(format!(
                    "Expected enum variant name after '.' in pattern at line {}:{}",
                    token.span.0, token.span.1
                ));
            };

            // For now, we'll treat enum member access patterns as identifiers
            // The runtime will handle matching enum values
            // Use a special format: "EnumName.VariantName" as the identifier
            return Ok(Pattern::Identifier(format!(
                "{}.{}",
                type_name, variant_name
            )));
        }

        // Regular identifier pattern
        return Ok(Pattern::Identifier(type_name));
    }

    // Fallback: try to parse as identifier (handles keywords that might be identifiers)
    if parser.peek().kind == crate::lexer::token::TokenKind::Keyword {
        let name = parser.peek().value.clone();
        parser.advance();
        return Ok(Pattern::Identifier(name));
    }

    let token = parser.peek();
    Err(format!(
        "Expected pattern at line {}:{}",
        token.span.0, token.span.1
    ))
}
