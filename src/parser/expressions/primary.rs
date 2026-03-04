use crate::parser::ast::*;
use crate::parser::parser::Parser;
use crate::parser::utils::{token_span_to_ast_span, parse_interpolated_string};

pub fn parse_primary(parser: &mut Parser) -> Result<Expression, String> {
    let token_kind = parser.peek().kind.clone();
    let token_value = parser.peek().value.clone();
    
    match token_kind {
        crate::lexer::token::TokenKind::Integer => {
            parser.advance();
            Ok(Expression::Literal(Literal::Integer(
                token_value.parse().unwrap_or(0)
            )))
        }
        crate::lexer::token::TokenKind::Float => {
            parser.advance();
            Ok(Expression::Literal(Literal::Float(
                token_value.parse().unwrap_or(0.0)
            )))
        }
        crate::lexer::token::TokenKind::String => {
            parser.advance();
            Ok(Expression::Literal(Literal::String(token_value)))
        }
        crate::lexer::token::TokenKind::InterpolatedString => {
            let token = parser.peek().clone();
            let span = token_span_to_ast_span(&token);
            parser.advance();
            parse_interpolated_string(parser, &token_value, span)
        }
        crate::lexer::token::TokenKind::Char => {
            parser.advance();
            // Parse char from string (token value is the char as string)
            let char_val = if token_value.len() == 1 {
                token_value.chars().next().unwrap()
            } else {
                // Handle escape sequences
                match token_value.as_str() {
                    "\\n" => '\n',
                    "\\t" => '\t',
                    "\\r" => '\r',
                    "\\\\" => '\\',
                    "\\'" => '\'',
                    "\\\"" => '"',
                    _ => token_value.chars().next().unwrap_or('\0'),
                }
            };
            Ok(Expression::Literal(Literal::Char(char_val)))
        }
        crate::lexer::token::TokenKind::Keyword => {
            match token_value.as_str() {
                "true" => {
                    parser.advance();
                    Ok(Expression::Literal(Literal::Boolean(true)))
                }
                "false" => {
                    parser.advance();
                    Ok(Expression::Literal(Literal::Boolean(false)))
                }
                "null" => {
                    parser.advance();
                    Ok(Expression::Literal(Literal::Null))
                }
                "catch" | "finally" | "end" | "else" | "elseif" => {
                    // These are block terminators, not valid in expressions
                    parser.error_with_context("Unexpected keyword in expression", &format!("'{}'", token_value))
                }
                _ => parser.error_with_context("Unexpected keyword", &format!("'{}'", token_value)),
            }
        }
        crate::lexer::token::TokenKind::Identifier => {
            parser.advance();
            Ok(Expression::Identifier(token_value))
        }
        crate::lexer::token::TokenKind::LeftParen => {
            // Check if this is a lambda: (params) -> expression
            // Look ahead to see if there's an arrow after params
            let saved_pos = parser.position;
            let mut is_lambda = false;
            
            // Advance past LeftParen
            parser.advance();
            
            // Try to parse parameters
            let mut params = Vec::new();
            if !parser.check(crate::lexer::token::TokenKind::RightParen) {
                // Try parsing as parameters
                loop {
                    if parser.check(crate::lexer::token::TokenKind::Identifier) {
                        let param_name = parser.expect_identifier()?;
                        
                        // Check for type annotation: param: type
                        let type_annotation = if parser.check(crate::lexer::token::TokenKind::Colon) {
                            parser.advance();
                            Some(parser.parse_type(&[])?) // Lambdas don't have generic params
                        } else {
                            None
                        };
                        
                        params.push(crate::parser::ast::Parameter {
                            name: param_name,
                            type_annotation,
                            is_variadic: false,
                            default_value: None,
                        });
                        
                        if parser.check(crate::lexer::token::TokenKind::Comma) {
                            parser.advance();
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
            
            // Check if next is RightParen followed by Arrow
            if parser.check(crate::lexer::token::TokenKind::RightParen) {
                parser.advance();
                if parser.check(crate::lexer::token::TokenKind::Arrow) {
                    is_lambda = true;
                } else {
                    // Not a lambda, restore to before LeftParen and parse as regular expression
                    parser.position = saved_pos;
                }
            } else {
                // Not a lambda, restore to before LeftParen
                parser.position = saved_pos;
            }
            
            if is_lambda {
                // Parse lambda: (params) -> expression
                // We've already advanced past LeftParen, params, RightParen, and checked for Arrow
                parser.expect(crate::lexer::token::TokenKind::Arrow)?;
                let body = crate::parser::expressions::operators::parse_expression(parser)?;
                Ok(Expression::Lambda {
                    params,
                    body: Box::new(body),
                    span: Span::default(),
                })
            } else {
                // Regular parenthesized expression - restore to before LeftParen
                parser.position = saved_pos;
                parser.advance(); // Advance past LeftParen
                let expr = crate::parser::expressions::operators::parse_expression(parser)?;
                parser.expect(crate::lexer::token::TokenKind::RightParen)?;
                Ok(expr)
            }
        }
        crate::lexer::token::TokenKind::LeftBracket => {
            parser.advance();
            let mut elements = Vec::new();
            if !parser.check(crate::lexer::token::TokenKind::RightBracket) {
                loop {
                    elements.push(crate::parser::expressions::operators::parse_expression(parser)?);
                    if !parser.check(crate::lexer::token::TokenKind::Comma) {
                        break;
                    }
                    parser.advance();
                }
            }
            parser.expect(crate::lexer::token::TokenKind::RightBracket)?;
            Ok(Expression::Array { elements, span: Span::default() })
        }
        crate::lexer::token::TokenKind::LeftBrace => {
            parser.advance();
            // Skip whitespace and newlines after opening brace
            while !parser.is_at_end() && 
                  (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
                   parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
                parser.advance();
            }
            
            // Check if it's a map (key: value) or set (just values)
            let mut entries = Vec::new();
            let mut elements = Vec::new();
            let mut is_map = false;
            
            if !parser.check(crate::lexer::token::TokenKind::RightBrace) {
                loop {
                    // Skip whitespace and newlines before key/element
                    while !parser.is_at_end() && 
                          (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
                           parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
                        parser.advance();
                    }
                    
                    let key_or_elem = crate::parser::expressions::operators::parse_expression(parser)?;
                    
                    // Skip whitespace and newlines after key/element
                    while !parser.is_at_end() && 
                          (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
                           parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
                        parser.advance();
                    }
                    
                    // Check if next token is colon (map) or comma/brace (set)
                    if parser.check(crate::lexer::token::TokenKind::Colon) {
                        is_map = true;
                        parser.advance();
                        
                        // Skip whitespace after colon
                        while !parser.is_at_end() && 
                              (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
                               parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
                            parser.advance();
                        }
                        
                        // Convert identifier keys to string literals (e.g., {name: "Alice"} -> {"name": "Alice"})
                        let key_expr = match &key_or_elem {
                            Expression::Identifier(name) => {
                                Expression::Literal(Literal::String(name.clone()))
                            }
                            _ => key_or_elem,
                        };
                        
                        let value = crate::parser::expressions::operators::parse_expression(parser)?;
                        entries.push((key_expr, value));
                    } else {
                        elements.push(key_or_elem);
                    }
                    
                    // Skip whitespace after value/element
                    while !parser.is_at_end() && 
                          (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
                           parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
                        parser.advance();
                    }
                    
                    if !parser.check(crate::lexer::token::TokenKind::Comma) {
                        break;
                    }
                    parser.advance();
                }
            }
            
            // Skip whitespace before closing brace
            while !parser.is_at_end() && 
                  (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
                   parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
                parser.advance();
            }
            
            parser.expect(crate::lexer::token::TokenKind::RightBrace)?;
            
            if is_map {
                Ok(Expression::Map { entries, span: Span::default() })
            } else {
                Ok(Expression::Set { elements, span: Span::default() })
            }
        }
        _ => parser.error(&format!("Unexpected token: {:?}", token_kind)),
    }
}

