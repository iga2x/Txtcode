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
        while !parser.is_at_end()
            && (parser.peek().kind == crate::lexer::token::TokenKind::Newline
                || parser.peek().kind == crate::lexer::token::TokenKind::Whitespace)
        {
            parser.advance();
        }

        if parser.is_at_end()
            || parser.check(crate::lexer::token::TokenKind::Eof)
            || parser.check_keyword("enum")
            || parser.check_keyword("struct")
            || parser.check_keyword("define")
            || parser.check_keyword("store")
            || parser.check_keyword("print")
            || parser.check_keyword("const")
        {
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
            Some(crate::parser::expressions::operators::parse_expression(
                parser,
            )?)
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

    // Task E.2: Optional generic type parameters `<T, U>`
    let mut type_params = Vec::new();
    if parser.check(crate::lexer::token::TokenKind::Less) {
        parser.advance(); // consume `<`
        loop {
            while parser.check(crate::lexer::token::TokenKind::Whitespace) { parser.advance(); }
            if parser.check(crate::lexer::token::TokenKind::Identifier) {
                type_params.push(parser.expect_identifier()?);
            } else {
                break;
            }
            while parser.check(crate::lexer::token::TokenKind::Whitespace) { parser.advance(); }
            if parser.check(crate::lexer::token::TokenKind::Comma) {
                parser.advance();
            } else {
                break;
            }
        }
        if parser.check(crate::lexer::token::TokenKind::Greater) {
            parser.advance(); // consume `>`
        }
    }

    let mut fields = Vec::new();

    // Parse struct fields
    if parser.check(crate::lexer::token::TokenKind::LeftParen) {
        parser.advance();

        // Skip whitespace/newlines
        while !parser.is_at_end()
            && (parser.peek().kind == crate::lexer::token::TokenKind::Newline
                || parser.peek().kind == crate::lexer::token::TokenKind::Whitespace)
        {
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
            while !parser.is_at_end()
                && (parser.peek().kind == crate::lexer::token::TokenKind::Newline
                    || parser.peek().kind == crate::lexer::token::TokenKind::Whitespace)
            {
                parser.advance();
            }

            if parser.check(crate::lexer::token::TokenKind::Comma) {
                parser.advance();
                // Skip whitespace/newlines after comma
                while !parser.is_at_end()
                    && (parser.peek().kind == crate::lexer::token::TokenKind::Newline
                        || parser.peek().kind == crate::lexer::token::TokenKind::Whitespace)
                {
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

    // Optional: `implements → [Protocol1, Protocol2]` or `implements Protocol1, Protocol2`
    let mut implements = Vec::new();
    // Skip horizontal whitespace (not newlines) before checking implements
    while parser.check(crate::lexer::token::TokenKind::Whitespace) {
        parser.advance();
    }
    if parser.check_keyword("implements") {
        parser.advance(); // consume `implements`
        parser.skip_optional_arrow();
        // Skip optional `[`
        let bracketed = parser.check(crate::lexer::token::TokenKind::LeftBracket);
        if bracketed { parser.advance(); }
        loop {
            while parser.check(crate::lexer::token::TokenKind::Whitespace) { parser.advance(); }
            if parser.check(crate::lexer::token::TokenKind::Identifier) {
                implements.push(parser.expect_identifier()?);
            } else {
                break;
            }
            while parser.check(crate::lexer::token::TokenKind::Whitespace) { parser.advance(); }
            if parser.check(crate::lexer::token::TokenKind::Comma) {
                parser.advance();
            } else {
                break;
            }
        }
        if bracketed && parser.check(crate::lexer::token::TokenKind::RightBracket) {
            parser.advance();
        }
    }

    let span = token_span_to_ast_span(&start_token);
    Ok(Some(Statement::Struct { name, type_params, fields, implements, span }))
}

/// Parse `impl → StructName\n  define → method → (self, ...) ... end\nend`
pub fn parse_impl(parser: &mut Parser) -> Result<Option<Statement>, String> {
    let start_token = parser.peek().clone();
    parser.expect_keyword("impl")?;
    parser.skip_optional_arrow();
    let struct_name = parser.expect_identifier()?;

    // Skip whitespace / newlines before the method definitions
    while !parser.is_at_end()
        && (parser.peek().kind == crate::lexer::token::TokenKind::Newline
            || parser.peek().kind == crate::lexer::token::TokenKind::Whitespace)
    {
        parser.advance();
    }

    let mut methods = Vec::new();
    // Collect define (and async define) statements until the matching `end`
    while !parser.is_at_end() && !parser.check_keyword("end") {
        // Skip blank lines
        while !parser.is_at_end()
            && (parser.peek().kind == crate::lexer::token::TokenKind::Newline
                || parser.peek().kind == crate::lexer::token::TokenKind::Whitespace)
        {
            parser.advance();
        }
        if parser.check_keyword("end") {
            break;
        }
        if parser.check_keyword("define") || parser.check_keyword("async") || parser.check_keyword("def") {
            if let Some(stmt) = crate::parser::statements::functions::parse_define(parser)? {
                methods.push(stmt);
            }
        } else {
            return parser.error("Expected 'define' inside impl block");
        }
    }
    parser.expect_keyword("end")?;

    let span = token_span_to_ast_span(&start_token);
    Ok(Some(Statement::Impl { struct_name, methods, span }))
}

/// Task E.1 — Parse `protocol → Name\n  method_name(params) → return\nend`
///
/// Syntax:
/// ```text
/// protocol → Serializable
///   serialize(self) → string
///   deserialize(s: string) → Self
/// end
/// ```
pub fn parse_protocol(parser: &mut Parser) -> Result<Option<Statement>, String> {
    use crate::lexer::token::TokenKind;
    let start_token = parser.peek().clone();
    parser.expect_keyword("protocol")?;
    parser.skip_optional_arrow();
    let name = parser.expect_identifier()?;

    let mut methods = Vec::new();

    // Skip newlines before body
    while !parser.is_at_end() && matches!(parser.peek().kind, TokenKind::Newline | TokenKind::Whitespace) {
        parser.advance();
    }

    // Parse method signatures until `end`
    while !parser.is_at_end() && !parser.check_keyword("end") {
        // Skip blank lines
        while !parser.is_at_end() && matches!(parser.peek().kind, TokenKind::Newline | TokenKind::Whitespace) {
            parser.advance();
        }
        if parser.check_keyword("end") || parser.is_at_end() { break; }

        // method_name(param: type, ...) → return_type
        if parser.check(TokenKind::Identifier) {
            let method_name = parser.expect_identifier()?;
            let mut param_types = Vec::new();

            // Optional parameter list in parens
            if parser.check(TokenKind::LeftParen) {
                parser.advance();
                while !parser.check(TokenKind::RightParen) && !parser.is_at_end() {
                    // param_name: type  OR  just a type name
                    let _param_name = if parser.check(TokenKind::Identifier) {
                        let id = parser.expect_identifier()?;
                        if parser.check(TokenKind::Colon) { parser.advance(); }
                        id
                    } else {
                        "arg".to_string()
                    };
                    // Collect type name (identifier or built-in type keyword like string/int)
                    // Use allow_reserved=true so type keywords ("string", "int", etc.) are accepted
                    if parser.check(TokenKind::Identifier) {
                        let type_name = parser.expect_identifier_allow_reserved(true)?;
                        param_types.push(type_name);
                    }
                    while parser.check(TokenKind::Comma) { parser.advance(); }
                    // skip whitespace
                    while matches!(parser.peek().kind, TokenKind::Whitespace) { parser.advance(); }
                }
                if parser.check(TokenKind::RightParen) { parser.advance(); }
            }

            // Optional return type after → (accept identifiers, type keywords, and other keywords like null)
            let return_type = if parser.check(TokenKind::Arrow) {
                parser.advance();
                if parser.check(TokenKind::Identifier) {
                    Some(parser.expect_identifier_allow_reserved(true)?)
                } else if parser.check(TokenKind::Keyword) {
                    // Accept keywords like `null`, `bool`, etc. as return type names
                    let kw = parser.peek().value.clone();
                    parser.advance();
                    Some(kw)
                } else {
                    None
                }
            } else {
                None
            };

            methods.push((method_name, param_types, return_type));

            // Consume any remaining tokens on this line (safety: avoids infinite loop on unexpected tokens)
            while !parser.is_at_end() && !matches!(parser.peek().kind, TokenKind::Newline | TokenKind::Whitespace) && !parser.check_keyword("end") {
                parser.advance();
            }
        }

        // Consume newline
        while !parser.is_at_end() && matches!(parser.peek().kind, TokenKind::Newline | TokenKind::Whitespace) {
            parser.advance();
        }
    }

    parser.expect_keyword("end")?;
    let span = token_span_to_ast_span(&start_token);
    Ok(Some(Statement::Protocol { name, methods, span }))
}
