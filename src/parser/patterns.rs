use crate::parser::ast::Pattern;
use crate::parser::parser::Parser;

pub fn parse_pattern(parser: &mut Parser) -> Result<Pattern, String> {
    // Check for literal patterns: integers, floats, strings, booleans, null
    let token = parser.peek();
    
    // Handle literal value patterns
    if matches!(token.kind, crate::lexer::token::TokenKind::Integer | 
                            crate::lexer::token::TokenKind::Float |
                            crate::lexer::token::TokenKind::String |
                            crate::lexer::token::TokenKind::Char) {
        // For literals in patterns, we'll store them as special identifier patterns
        // The runtime will handle matching literal values
        let value = token.value.clone();
        parser.advance();
        return Ok(Pattern::Identifier(format!("__literal_{}", value)));
    }
    
    // Handle boolean/null literal keywords
    if token.kind == crate::lexer::token::TokenKind::Keyword {
        if token.value == "true" || token.value == "false" || token.value == "null" {
            let value = token.value.clone();
            parser.advance();
            return Ok(Pattern::Identifier(format!("__literal_{}", value)));
        }
    }
    
    // Check for array pattern: [a, b, c]
    if parser.check(crate::lexer::token::TokenKind::LeftBracket) {
        parser.advance();
        let mut patterns = Vec::new();
        
        if !parser.check(crate::lexer::token::TokenKind::RightBracket) {
            loop {
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
            while !parser.is_at_end() && 
                  (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
                   parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
                parser.advance();
            }
            
            let mut args = Vec::new();
            
            // Parse constructor arguments
            if !parser.check(crate::lexer::token::TokenKind::RightParen) {
                loop {
                    args.push(parse_pattern(parser)?);
                    
                    // Skip whitespace after argument
                    while !parser.is_at_end() && 
                          (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
                           parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
                        parser.advance();
                    }
                    
                    if !parser.check(crate::lexer::token::TokenKind::Comma) {
                        break;
                    }
                    parser.advance();
                    
                    // Skip whitespace after comma
                    while !parser.is_at_end() && 
                          (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
                           parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
                        parser.advance();
                    }
                }
            }
            
            parser.expect(crate::lexer::token::TokenKind::RightParen)?;
            return Ok(Pattern::Constructor {
                type_name,
                args,
            });
        }
        
        // Check if there's a dot followed by a variant name (enum member access)
        if parser.check(crate::lexer::token::TokenKind::Dot) {
            parser.advance();
            
            // Skip whitespace after dot
            while !parser.is_at_end() && 
                  (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
                   parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
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
                return Err(format!("Expected enum variant name after '.' in pattern at line {}:{}", token.span.0, token.span.1));
            };
            
            // For now, we'll treat enum member access patterns as identifiers
            // The runtime will handle matching enum values
            // Use a special format: "EnumName.VariantName" as the identifier
            return Ok(Pattern::Identifier(format!("{}.{}", type_name, variant_name)));
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
    return Err(format!("Expected pattern at line {}:{}", token.span.0, token.span.1));
}

