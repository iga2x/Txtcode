use crate::parser::ast::*;
use crate::parser::ast::capabilities::CapabilityExpr;
use crate::parser::parser::Parser;
use crate::parser::utils::token_span_to_ast_span;
use crate::tools::logger::log_debug;

pub fn parse_define(parser: &mut Parser) -> Result<Option<Statement>, String> {
    let start_token = parser.peek().clone();
    
    // Check for async keyword before define
    let is_async = if parser.check_keyword("async") {
        parser.advance(); // consume "async"
        // Skip whitespace/arrow after async
        while !parser.is_at_end() && 
              (parser.peek().kind == crate::lexer::token::TokenKind::Whitespace ||
               parser.peek().kind == crate::lexer::token::TokenKind::Newline) {
            parser.advance();
        }
        // Optional arrow after async
        if parser.check(crate::lexer::token::TokenKind::Arrow) {
            parser.advance();
        }
        true
    } else {
        false
    };
    
    parser.expect_keyword("define")?;
    
    // Check for old syntax (deprecated): "define name (" without arrow
    let has_arrow_after_define = parser.check(crate::lexer::token::TokenKind::Arrow);
    parser.skip_optional_arrow(); // Optional arrow after define
    let name = parser.expect_identifier()?;
    
    // Warn about old syntax: "define name (" should be "define -> name -> ("
    if !has_arrow_after_define {
        // Skip whitespace to check if next is opening paren
        let saved_pos = parser.position;
        while !parser.is_at_end() && 
              (parser.peek().kind == crate::lexer::token::TokenKind::Whitespace ||
               parser.peek().kind == crate::lexer::token::TokenKind::Newline) {
            parser.advance();
        }
        if parser.check(crate::lexer::token::TokenKind::LeftParen) {
            log_debug(&format!(
                "Warning: Deprecated syntax at line {}: Use 'define -> {} -> (...)' instead of 'define {} (...)'",
                parser.peek().span.0, name, name
            ));
        }
        parser.position = saved_pos; // Restore position
    }
    
    // Parse optional generic type parameters: <T, U, ...>
    let mut type_params = Vec::new();
    if parser.check(crate::lexer::token::TokenKind::Less) {
        parser.advance();
        loop {
            let param_name = parser.expect_identifier()?;
            type_params.push(param_name);
            
            if parser.check(crate::lexer::token::TokenKind::Comma) {
                parser.advance();
            } else {
                break;
            }
        }
        parser.expect(crate::lexer::token::TokenKind::Greater)?;
    }
    
    // Optional arrow before parameter list (for compatibility with both syntaxes)
    // Standard: define -> name (params) -> return_type
    // Alternative: define -> name -> (params) -> return_type
    if parser.check(crate::lexer::token::TokenKind::Arrow) {
        parser.advance();
        // Skip whitespace after optional arrow
        while !parser.is_at_end() && 
              (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
               parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
            parser.advance();
        }
    }
    
    parser.expect(crate::lexer::token::TokenKind::LeftParen)?;
    
    // Parse parameter list with support for:
    // - Variadic parameters (...args)
    // - Type annotations (name: type)
    // - Default values (name = expr or name: type = expr)
    // - Destructured map params ({x, y}) → synthetic __dest_N__ param + prepended assignments
    let mut params = Vec::new();
    let mut seen_variadic = false;
    let mut dest_stmts: Vec<Statement> = Vec::new(); // prepended to body for destructured params

    if !parser.check(crate::lexer::token::TokenKind::RightParen) {
        loop {
            // Skip whitespace/newlines before parameter
            while !parser.is_at_end() &&
                  (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
                   parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
                parser.advance();
            }

            // Check if we're at the end of parameters
            if parser.check(crate::lexer::token::TokenKind::RightParen) {
                break;
            }

            // Destructured map parameter: {x, y} or {x: alias, y}
            if parser.check(crate::lexer::token::TokenKind::LeftBrace) {
                parser.advance(); // consume `{`
                let dest_name = format!("__dest_{}__", params.len());
                let mut fields: Vec<String> = Vec::new();
                while !parser.check(crate::lexer::token::TokenKind::RightBrace) && !parser.is_at_end() {
                    let field = parser.expect_identifier()?;
                    fields.push(field);
                    if parser.check(crate::lexer::token::TokenKind::Comma) {
                        parser.advance();
                    } else {
                        break;
                    }
                }
                parser.expect(crate::lexer::token::TokenKind::RightBrace)?;
                // Generate: store → field → __dest_N__["field"] for each field
                for field in &fields {
                    let key_expr = crate::parser::ast::Expression::Literal(
                        crate::parser::ast::Literal::String(field.clone())
                    );
                    let index_expr = crate::parser::ast::Expression::Index {
                        target: Box::new(crate::parser::ast::Expression::Identifier(dest_name.clone())),
                        index: Box::new(key_expr),
                        span: crate::parser::ast::Span::default(),
                    };
                    dest_stmts.push(Statement::Assignment {
                        pattern: crate::parser::ast::Pattern::Identifier(field.clone()),
                        type_annotation: None,
                        value: index_expr,
                        span: crate::parser::ast::Span::default(),
                    });
                }
                params.push(Parameter {
                    name: dest_name,
                    type_annotation: None,
                    is_variadic: false,
                    default_value: None,
                });
                // Handle comma separator
                while !parser.is_at_end() &&
                      (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
                       parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
                    parser.advance();
                }
                if parser.check(crate::lexer::token::TokenKind::Comma) {
                    parser.advance();
                } else {
                    // No comma - check if we should break (likely end of params)
                    // Don't break, fall through to check RightParen at top of loop
                }
                continue;
            }

            // 1. Check for variadic ... (must be before parameter name)
            let mut is_variadic = false;
            if parser.check(crate::lexer::token::TokenKind::Dot) {
                // Check if we have three consecutive dots
                if parser.position + 2 < parser.tokens.len() &&
                   parser.tokens[parser.position].kind == crate::lexer::token::TokenKind::Dot &&
                   parser.tokens[parser.position + 1].kind == crate::lexer::token::TokenKind::Dot &&
                   parser.tokens[parser.position + 2].kind == crate::lexer::token::TokenKind::Dot {
                    // It's ... (three dots) - consume all three
                    parser.advance(); // first .
                    parser.advance(); // second .
                    parser.advance(); // third .
                    is_variadic = true;
                    seen_variadic = true;
                    
                    // Skip whitespace after ...
                    while !parser.is_at_end() && 
                          (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
                           parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
                        parser.advance();
                    }
                }
                // If it's not three dots, it's just a regular dot (not variadic)
                // We'll handle it as a regular parameter name or error later
            }
            
            // 2. Expect parameter name (required after variadic dots)
            let param_name = if is_variadic {
                parser.expect_identifier()
                    .map_err(|_| {
                        let token = parser.peek();
                        format!(
                            "Parse error at line {}, column {}: Expected identifier after variadic dots for function parameter (found {:?} '{}')",
                            token.span.0, token.span.1, token.kind, token.value
                        )
                    })?
            } else {
                parser.expect_identifier()
                    .map_err(|_| {
                        let token = parser.peek();
                        format!(
                            "Parse error at line {}, column {}: Expected identifier for function parameter (found {:?} '{}')",
                            token.span.0, token.span.1, token.kind, token.value
                        )
                    })?
            };
            
            // 3. Optional type annotation (name: type)
            let mut type_annotation = None;
            if parser.check(crate::lexer::token::TokenKind::Colon) {
                parser.advance();
                type_annotation = Some(parser.parse_type(&type_params)?);
            }
            
            // 4. Optional default value (name = expr or name: type = expr)
            let mut default_value = None;
            if parser.check(crate::lexer::token::TokenKind::Assignment) {
                if is_variadic {
                    return parser.error("Variadic parameter cannot have a default value");
                }
                parser.advance();
                default_value = Some(crate::parser::expressions::operators::parse_expression(parser)?);
            }
            
            // 5. Add parameter to list
            log_debug(&format!(
                "Parsing parameter: name={}, is_variadic={}, has_type={}, has_default={}",
                param_name,
                is_variadic,
                type_annotation.is_some(),
                default_value.is_some()
            ));
            params.push(Parameter {
                name: param_name,
                type_annotation,
                is_variadic,
                default_value,
            });
            
            // 6. Variadic parameter must be the last parameter
            if is_variadic {
                // Skip whitespace before checking for closing paren
                while !parser.is_at_end() && 
                      (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
                       parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
                    parser.advance();
                }
                // Variadic must be immediately followed by closing paren
                if !parser.check(crate::lexer::token::TokenKind::RightParen) {
                    return parser.error_with_context(
                        "Variadic parameter must be the last parameter",
                        "in function definition"
                    );
                }
                break;
            }
            
            // 7. Comma separator (skip whitespace around it)
            while !parser.is_at_end() && 
                  (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
                   parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
                parser.advance();
            }
            
            if parser.check(crate::lexer::token::TokenKind::Comma) {
                parser.advance();
            } else {
                break;
            }
        }
    }
    
    parser.expect(crate::lexer::token::TokenKind::RightParen)?;
    
    // Ensure variadic is last (check after parsing all parameters)
    if seen_variadic {
        // This should have been caught in the loop, but double-check
        if !params.is_empty() {
            let last_param = &params[params.len() - 1];
            if !last_param.is_variadic {
                return parser.error_with_context(
                    "Variadic parameter must be the last parameter",
                    "in function definition"
                );
            }
        }
    }
    
    // Skip whitespace before checking for return type
    while !parser.is_at_end() && 
          (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
           parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
        parser.advance();
    }
    
    // Check for return type annotation (optional -> type or just type)
    let mut return_type = None;
    
    // Check if we have a doc/hint declaration instead of return type
    // Both canonical names (doc, hint) and legacy aliases (intent, ai_hint) are accepted
    let has_intent = parser.check_keyword("doc") || parser.check_keyword("intent") ||
                     parser.check_keyword("hint") || parser.check_keyword("ai_hint") ||
                     parser.check_keyword("allowed") || parser.check_keyword("forbidden");
    
    // Only try to parse return type if:
    // 1. Not at "end" keyword
    // 2. Not at end of input
    // 3. Not an intent declaration
    // 4. There's an arrow (indicating return type) AND we can peek ahead to check for type
    if !parser.check_keyword("end") && !parser.is_at_end() && !has_intent {
        // Check if there's an arrow (optional before return type)
        if parser.check(crate::lexer::token::TokenKind::Arrow) {
            // Peek ahead after arrow to see if it's followed by a type
            // We'll peek by temporarily advancing and checking
            let saved_pos = parser.position;
            
            // Skip arrow
            parser.advance();
            
            // Skip whitespace after arrow
            while !parser.is_at_end() && 
                  (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
                   parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
                parser.advance();
            }
            
            // Check if next token looks like a type
            let looks_like_type = parser.check_keyword("int") || 
                                 parser.check_keyword("float") || 
                                 parser.check_keyword("string") || 
                                 parser.check_keyword("bool") || 
                                 parser.check_keyword("char") ||
                                 parser.check_keyword("array") ||
                                 parser.check_keyword("map") ||
                                 parser.check_keyword("set") ||
                                 (parser.check(crate::lexer::token::TokenKind::Identifier) && 
                                  // Check it's not a statement keyword
                                  !parser.check_keyword("return") && 
                                  !parser.check_keyword("ret") &&
                                  !parser.check_keyword("print") && 
                                  !parser.check_keyword("out") &&
                                  !parser.check_keyword("end") &&
                                  !parser.check_keyword("if") &&
                                  !parser.check_keyword("while") &&
                                  !parser.check_keyword("for") &&
                                  !parser.check_keyword("match") &&
                                  !parser.check_keyword("store") &&
                                  !parser.check_keyword("let") &&
                                  !parser.check_keyword("const") &&
                                  !parser.check_keyword("define") &&
                                  !parser.check_keyword("def") &&
                                  !parser.check_keyword("break") &&
                                  !parser.check_keyword("continue"));
            
            if looks_like_type && !parser.check_keyword("end") {
                // Arrow is followed by something that looks like a type - parse it
                match parser.parse_type(&type_params) {
                    Ok(ty) => {
                        return_type = Some(ty);
                    }
                    Err(_) => {
                        // Parse failed, restore position and assume no return type
                        parser.position = saved_pos;
                        return_type = None;
                    }
                }
            } else {
                // Arrow not followed by a type - restore position, no return type
                parser.position = saved_pos;
                return_type = None;
            }
        } else {
            // No arrow - check if we directly have a type keyword (uncommon syntax)
            // Check both Keyword and Identifier tokens for type keywords
            // (type keywords might be tokenized as either)
            let next_token_value = if !parser.is_at_end() && !parser.check_keyword("end") {
                Some(parser.peek().value.clone())
            } else {
                None
            };
            
            let is_type_keyword = if let Some(ref token_val) = next_token_value {
                use crate::lexer::keywords::is_type_keyword;
                is_type_keyword(token_val)
            } else {
                false
            };
            
            if is_type_keyword {
                // Try to parse type directly (no arrow syntax)
                let saved_pos = parser.position;
                match parser.parse_type(&type_params) {
                    Ok(ty) => {
                        return_type = Some(ty);
                    }
                    Err(_) => {
                        // Parse failed - restore position
                        parser.position = saved_pos;
                        // But we detected a type keyword, so consume it
                        // to prevent it from being treated as a variable later
                        if let Some(ref token_val) = next_token_value {
                            use crate::lexer::keywords::is_type_keyword;
                            if is_type_keyword(token_val) && !parser.check_keyword("end") {
                                // Consume the type token (either Keyword or Identifier)
                                parser.advance();
                                // Skip whitespace after the consumed token
                                while !parser.is_at_end() && 
                                      (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
                                       parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
                                    parser.advance();
                                }
                            }
                        }
                        return_type = None;
                    }
                }
            }
        }
    }
    
    // Parse optional intent declarations (before body)
    let mut intent = None;
    let mut ai_hint = None;
    let mut allowed_actions: Vec<CapabilityExpr> = Vec::new();
    let mut forbidden_actions: Vec<CapabilityExpr> = Vec::new();
    
    // Skip whitespace before checking for intent declarations
    while !parser.is_at_end() && 
          (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
           parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
        parser.advance();
    }
    
    // Parse intent, ai_hint, allowed, forbidden declarations
    while !parser.check_keyword("end") && !parser.is_at_end() {
        // Check for doc/intent declaration (doc is canonical; intent is legacy alias)
        if parser.check_keyword("doc") || parser.check_keyword("intent") {
            parser.advance();
            parser.expect_arrow()?;
            if parser.check(crate::lexer::token::TokenKind::String) {
                intent = Some(parser.peek().value.clone());
                parser.advance();
            } else {
                return parser.error("doc requires a string value");
            }
        }
        // Check for hint/ai_hint declaration (hint is canonical; ai_hint is legacy alias)
        else if parser.check_keyword("hint") || parser.check_keyword("ai_hint") || parser.check_keyword("ai-hint") || parser.check_keyword("aihint") {
            parser.advance();
            parser.expect_arrow()?;
            if parser.check(crate::lexer::token::TokenKind::String) {
                ai_hint = Some(parser.peek().value.clone());
                parser.advance();
            } else {
                return parser.error("hint requires a string value");
            }
        }
        // Check for allowed actions declaration
        else if parser.check_keyword("allowed") {
            parser.advance();
            parser.expect_arrow()?;
            // Parse array of strings: ["fs.read", "fs.write"]
            parser.expect(crate::lexer::token::TokenKind::LeftBracket)?;
            while !parser.check(crate::lexer::token::TokenKind::RightBracket) {
                while !parser.is_at_end() && 
                      (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
                       parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
                    parser.advance();
                }
                if parser.check(crate::lexer::token::TokenKind::String) {
                    let cap_str = parser.peek().value.clone();
                    let token = parser.peek().clone();
                    parser.advance();
                    
                    // Convert token span to AST span
                    let span = token_span_to_ast_span(&token);
                    
                    // Parse capability from string to AST
                    match CapabilityExpr::from_string(&cap_str, span) {
                        Ok(cap_expr) => {
                            allowed_actions.push(cap_expr);
                        }
                        Err(e) => {
                            return parser.error(&format!("Invalid capability format in allowed: {}", e));
                        }
                    }
                } else {
                    return parser.error("allowed requires an array of capability strings (e.g., \"fs.read\", \"net.connect\")");
                }
                if parser.check(crate::lexer::token::TokenKind::Comma) {
                    parser.advance();
                } else {
                    break;
                }
            }
            parser.expect(crate::lexer::token::TokenKind::RightBracket)?;
        }
        // Check for forbidden actions declaration
        else if parser.check_keyword("forbidden") {
            parser.advance();
            parser.expect_arrow()?;
            // Parse array of strings: ["fs.write", "process.exec"]
            parser.expect(crate::lexer::token::TokenKind::LeftBracket)?;
            while !parser.check(crate::lexer::token::TokenKind::RightBracket) {
                while !parser.is_at_end() && 
                      (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
                       parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
                    parser.advance();
                }
                if parser.check(crate::lexer::token::TokenKind::String) {
                    let cap_str = parser.peek().value.clone();
                    let token = parser.peek().clone();
                    parser.advance();
                    
                    // Convert token span to AST span
                    let span = token_span_to_ast_span(&token);
                    
                    // Parse capability from string to AST
                    match CapabilityExpr::from_string(&cap_str, span) {
                        Ok(cap_expr) => {
                            forbidden_actions.push(cap_expr);
                        }
                        Err(e) => {
                            return parser.error(&format!("Invalid capability format in forbidden: {}", e));
                        }
                    }
                } else {
                    return parser.error("forbidden requires an array of capability strings (e.g., \"fs.write\", \"process.exec\")");
                }
                if parser.check(crate::lexer::token::TokenKind::Comma) {
                    parser.advance();
                } else {
                    break;
                }
            }
            parser.expect(crate::lexer::token::TokenKind::RightBracket)?;
        }
        else {
            // Not an intent declaration, start parsing body
            break;
        }
        
        // Skip whitespace after intent declaration
        while !parser.is_at_end() && 
              (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
               parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
            parser.advance();
        }
    }
    
    // Prepend destructured-param assignments to the body
    let mut body: Vec<Statement> = dest_stmts;
    loop {
        // Skip newlines and whitespace before checking for end
        while !parser.is_at_end() &&
              (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
               parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
            parser.advance();
        }

        if parser.check_keyword("end") {
            break;
        }

        if let Some(stmt) = parser.parse_statement()? {
            body.push(stmt);
        } else {
            break;
        }
    }
    parser.expect_keyword("end")?;
    let span = token_span_to_ast_span(&start_token);
    
    log_debug(&format!(
        "Parsed function '{}' with {} parameters (variadic: {}), return_type: {:?}, body statements: {}",
        name,
        params.len(),
        seen_variadic,
        return_type,
        body.len()
    ));
    
    Ok(Some(Statement::FunctionDef {
        name,
        type_params,
        params,
        return_type,
        body,
        is_async,
        intent,
        ai_hint,
        allowed_actions,
        forbidden_actions,
        span,
    }))
}

pub fn parse_return(parser: &mut Parser) -> Result<Option<Statement>, String> {
    use crate::parser::ast::{Expression, Span};
    let start_token = parser.peek().clone();
    parser.expect_keyword("return")?;
    let value = if !parser.check_keyword("end") && !parser.is_at_end() {
        parser.skip_optional_arrow(); // Optional arrow before value
        let first = crate::parser::expressions::operators::parse_expression(parser)?;
        // Multi-return: `return → a, b` auto-wraps as an array
        if parser.check(crate::lexer::token::TokenKind::Comma) {
            let mut elements = vec![first];
            while parser.check(crate::lexer::token::TokenKind::Comma) {
                parser.advance(); // consume comma
                elements.push(crate::parser::expressions::operators::parse_expression(parser)?);
            }
            Some(Expression::Array { elements, span: Span::default() })
        } else {
            Some(first)
        }
    } else {
        None
    };
    let span = token_span_to_ast_span(&start_token);
    Ok(Some(Statement::Return {
        value,
        span,
    }))
}

