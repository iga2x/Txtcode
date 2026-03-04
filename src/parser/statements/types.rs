use crate::parser::ast::*;
use crate::parser::parser::Parser;
use crate::parser::utils::token_span_to_ast_span;

pub fn parse_enum(parser: &mut Parser) -> Result<Option<Statement>, String> {
    let start_token = parser.peek().clone();
    parser.expect_keyword("enum")?;
    parser.skip_optional_arrow(); // Optional arrow after enum
    let name = parser.expect_identifier()?;
    parser.skip_optional_arrow(); // Optional arrow before variants (if any)
    
    let mut variants = Vec::new();
    
    // Parse enum variants (comma-separated list of identifiers)
    loop {
        // Skip whitespace/newlines
        while !parser.is_at_end() && 
              (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
               parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
            parser.advance();
        }
        
        if parser.is_at_end() || 
           parser.check(crate::lexer::token::TokenKind::Eof) ||
           parser.check_keyword("enum") ||
           parser.check_keyword("struct") ||
           parser.check_keyword("define") ||
           parser.check_keyword("store") ||
           parser.check_keyword("print") ||
           parser.check_keyword("const") {
            break;
        }
        
        // Try to parse a variant name
        let variant_name = if parser.check(crate::lexer::token::TokenKind::Identifier) {
            parser.expect_identifier()?
        } else if parser.check(crate::lexer::token::TokenKind::Keyword) {
            // Keywords can be enum variant names (like Red, Green, Blue)
            let name = parser.peek().value.clone();
            parser.advance();
            name
        } else {
            // Not a variant, break
            break;
        };
        
        // Check for value assignment (e.g., Active = 1)
        let variant_value = if parser.check(crate::lexer::token::TokenKind::Assignment) {
            parser.advance();
            Some(crate::parser::expressions::operators::parse_expression(parser)?)
        } else {
            None
        };
        
        variants.push((variant_name, variant_value));
        
        // Check for comma
        if parser.check(crate::lexer::token::TokenKind::Comma) {
            parser.advance();
        } else {
            break;
        }
    }
    
    let span = token_span_to_ast_span(&start_token);
    Ok(Some(Statement::Enum {
        name,
        variants,
        span,
    }))
}

pub fn parse_struct(parser: &mut Parser) -> Result<Option<Statement>, String> {
    let start_token = parser.peek().clone();
    parser.expect_keyword("struct")?;
    parser.skip_optional_arrow(); // Optional arrow after struct
    let name = parser.expect_identifier()?;
    parser.skip_optional_arrow(); // Optional arrow before fields (if any)
    
    let mut fields = Vec::new();
    
    // Parse struct fields
    if parser.check(crate::lexer::token::TokenKind::LeftParen) {
        parser.advance();
        
        // Skip whitespace/newlines
        while !parser.is_at_end() && 
              (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
               parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
            parser.advance();
        }
        
        while !parser.check(crate::lexer::token::TokenKind::RightParen) && !parser.is_at_end() {
            // Parse field definition
            if parser.check(crate::lexer::token::TokenKind::Identifier) {
                let field_name = parser.expect_identifier()?;
                
                    if parser.check(crate::lexer::token::TokenKind::Colon) {
                        parser.advance();
                        // Parse type - struct fields don't have generic params
                        let field_type = parser.parse_type(&[])?;
                        fields.push((field_name, field_type));
                } else {
                    return parser.error("Expected colon after field name in struct");
                }
            } else {
                return parser.error("Expected field name in struct");
            }
            
            // Skip whitespace/newlines
            while !parser.is_at_end() && 
                  (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
                   parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
                parser.advance();
            }
            
            if parser.check(crate::lexer::token::TokenKind::Comma) {
                parser.advance();
                // Skip whitespace/newlines after comma
                while !parser.is_at_end() && 
                      (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
                       parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
                    parser.advance();
                }
            } else {
                break;
            }
        }
        
        if parser.check(crate::lexer::token::TokenKind::RightParen) {
            parser.advance();
        } else {
            return parser.error("Expected closing parenthesis for struct fields");
        }
    } else {
        return parser.error("Expected opening parenthesis for struct fields");
    }
    
    let span = token_span_to_ast_span(&start_token);
    Ok(Some(Statement::Struct {
        name,
        fields,
        span,
    }))
}
