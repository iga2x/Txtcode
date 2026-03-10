use crate::parser::ast::*;
use crate::parser::parser::Parser;
use crate::parser::utils::token_span_to_ast_span;

pub fn parse_if(parser: &mut Parser) -> Result<Option<Statement>, String> {
    let start_token = parser.peek().clone();
    parser.expect_keyword("if")?;
    parser.skip_optional_arrow(); // Optional arrow before condition
    let condition = crate::parser::expressions::operators::parse_expression(parser)?;
    
    // Skip optional "then" keyword
    while !parser.is_at_end() && 
          (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
           parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
        parser.advance();
    }
    if parser.check_keyword("then") {
        parser.advance();
    }
    
    let mut then_branch = Vec::new();
    let mut else_if_branches = Vec::new();
    
    // Parse then branch - stop at else, elseif, or end
    loop {
        // Skip whitespace/newlines
        while !parser.is_at_end() && 
              (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
               parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
            parser.advance();
        }
        
        if parser.is_at_end() {
            break;
        }
        
        // Check for else, elseif, or end keywords
        if parser.check_keyword("else") || parser.check_keyword("elseif") || parser.check_keyword("end") {
            break;
        }
        
        if let Some(stmt) = parser.parse_statement()? {
            then_branch.push(stmt);
        } else {
            break;
        }
    }
    
    // Parse elseif branches
    while parser.check_keyword("elseif") {
        parser.advance();
        parser.skip_optional_arrow(); // Optional arrow before condition
        let elseif_condition = crate::parser::expressions::operators::parse_expression(parser)?;
        let mut elseif_body = Vec::new();
        
        loop {
            // Skip whitespace/newlines
            while !parser.is_at_end() && 
                  (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
                   parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
                parser.advance();
            }
            
            if parser.is_at_end() {
                break;
            }
            
            if parser.check_keyword("else") || parser.check_keyword("elseif") || parser.check_keyword("end") {
                break;
            }
            
            if let Some(stmt) = parser.parse_statement()? {
                elseif_body.push(stmt);
            } else {
                break;
            }
        }
        else_if_branches.push((elseif_condition, elseif_body));
    }
    
    // Parse else branch
    let else_branch = if parser.check_keyword("else") {
        parser.advance();
        let mut else_body = Vec::new();
        
        loop {
            // Skip whitespace/newlines
            while !parser.is_at_end() && 
                  (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
                   parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
                parser.advance();
            }
            
            if parser.is_at_end() {
                break;
            }
            
            if parser.check_keyword("end") {
                break;
            }
            
            if let Some(stmt) = parser.parse_statement()? {
                else_body.push(stmt);
            } else {
                break;
            }
        }
        Some(else_body)
    } else {
        None
    };
    
    parser.expect_keyword("end")?;
    let span = token_span_to_ast_span(&start_token);
    
    Ok(Some(Statement::If {
        condition,
        then_branch,
        else_if_branches,
        else_branch,
        span,
    }))
}

pub fn parse_while(parser: &mut Parser) -> Result<Option<Statement>, String> {
    let start_token = parser.peek().clone();
    parser.expect_keyword("while")?;
    parser.skip_optional_arrow(); // Optional arrow before condition
    let condition = crate::parser::expressions::operators::parse_expression(parser)?;
    
    let mut body = Vec::new();
    while !parser.is_at_end() && !parser.check_keyword("end") {
        if let Some(stmt) = parser.parse_statement()? {
            body.push(stmt);
        } else {
            break;
        }
    }
    parser.expect_keyword("end")?;
    let span = token_span_to_ast_span(&start_token);
    
    Ok(Some(Statement::While {
        condition,
        body,
        span,
    }))
}

pub fn parse_do_while(parser: &mut Parser) -> Result<Option<Statement>, String> {
    let start_token = parser.peek().clone();
    parser.expect_keyword("do")?;
    
    let mut body = Vec::new();
    loop {
        // Skip newlines/whitespace before checking for the closing `while` keyword
        while !parser.is_at_end() &&
              (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
               parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
            parser.advance();
        }
        if parser.is_at_end() || parser.check_keyword("while") {
            break;
        }
        if let Some(stmt) = parser.parse_statement()? {
            body.push(stmt);
        } else {
            break;
        }
    }

    parser.expect_keyword("while")?;
    parser.skip_optional_arrow(); // Optional arrow before condition
    let condition = crate::parser::expressions::operators::parse_expression(parser)?;
    // Skip newlines between condition and closing `end`
    while !parser.is_at_end() &&
          (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
           parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
        parser.advance();
    }
    parser.expect_keyword("end")?;
    
    let span = token_span_to_ast_span(&start_token);
    
    Ok(Some(Statement::DoWhile {
        body,
        condition,
        span,
    }))
}

pub fn parse_for(parser: &mut Parser) -> Result<Option<Statement>, String> {
    let start_token = parser.peek().clone();
    // Handle both "for" and "foreach" (canonicalized to "for")
    // The keyword is already consumed by the match in parse_statement
    // Just need to expect arrow
    parser.skip_optional_arrow(); // Optional arrow before variable
    let var_name = parser.expect_identifier()?;
    parser.expect_keyword("in")?;
    let iterable = crate::parser::expressions::operators::parse_expression(parser)?;
    
    let mut body = Vec::new();
    while !parser.is_at_end() && !parser.check_keyword("end") {
        if let Some(stmt) = parser.parse_statement()? {
            body.push(stmt);
        } else {
            break;
        }
    }
    parser.expect_keyword("end")?;
    let span = token_span_to_ast_span(&start_token);
    
    Ok(Some(Statement::For {
        variable: var_name,
        iterable,
        body,
        span,
    }))
}

pub fn parse_repeat(parser: &mut Parser) -> Result<Option<Statement>, String> {
    let start_token = parser.peek().clone();
    parser.expect_keyword("repeat")?;
    parser.skip_optional_arrow(); // Optional arrow before count
    let count = crate::parser::expressions::operators::parse_expression(parser)?;
    parser.expect_keyword("times")?;
    
    let mut body = Vec::new();
    while !parser.is_at_end() && !parser.check_keyword("end") {
        if let Some(stmt) = parser.parse_statement()? {
            body.push(stmt);
        } else {
            break;
        }
    }
    parser.expect_keyword("end")?;
    let span = token_span_to_ast_span(&start_token);
    
    Ok(Some(Statement::Repeat {
        count,
        body,
        span,
    }))
}

pub fn parse_match(parser: &mut Parser) -> Result<Option<Statement>, String> {
    let start_token = parser.peek().clone();
    // Keyword already consumed by parse_statement
    parser.skip_optional_arrow(); // Optional arrow before value
    let value = crate::parser::expressions::operators::parse_expression(parser)?;
    
    // Skip whitespace/newlines after match expression
    while !parser.is_at_end() && 
          (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
           parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
        parser.advance();
    }
    
    let mut cases = Vec::new();
    let mut default = None;
    
    while !parser.is_at_end() && !parser.check_keyword("end") {
        if parser.check_keyword("case") {
            parser.advance();
            // Arrow is optional after case
            if parser.check(crate::lexer::token::TokenKind::Arrow) {
                parser.advance();
            }
            
            // Parse pattern
            let pattern = if parser.check_keyword("_") {
                parser.advance();
                // Default case
                let mut default_body = Vec::new();
                while !parser.is_at_end() && !parser.check_keyword("end") && !parser.check_keyword("case") {
                    if let Some(stmt) = parser.parse_statement()? {
                        default_body.push(stmt);
                    } else {
                        break;
                    }
                }
                default = Some(default_body);
                continue; // Skip to next iteration
            } else {
                parser.parse_pattern()?
            };
            
            // Optional guard: if condition
            let guard = if parser.check_keyword("if") {
                parser.advance();
                // Arrow is optional after if in guard
                if parser.check(crate::lexer::token::TokenKind::Arrow) {
                    parser.advance();
                }
                Some(crate::parser::expressions::operators::parse_expression(parser)?)
            } else {
                None
            };
            
            // Parse case body
            let mut case_body = Vec::new();
            // Skip whitespace/newlines before parsing body
            while !parser.is_at_end() && 
                  (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
                   parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
                parser.advance();
            }
            while !parser.is_at_end() && !parser.check_keyword("end") && !parser.check_keyword("case") {
                // Skip whitespace/newlines
                while !parser.is_at_end() && 
                      (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
                       parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
                    parser.advance();
                }
                if parser.check_keyword("end") || parser.check_keyword("case") {
                    break;
                }
                if let Some(stmt) = parser.parse_statement()? {
                    case_body.push(stmt);
                } else {
                    break;
                }
            }
            
            cases.push((pattern, guard, case_body));
        } else {
            break;
        }
    }
    
    parser.expect_keyword("end")?;
    let span = token_span_to_ast_span(&start_token);
    Ok(Some(Statement::Match {
        value,
        cases,
        default,
        span,
    }))
}

pub fn parse_try(parser: &mut Parser) -> Result<Option<Statement>, String> {
    let start_token = parser.peek().clone();
    parser.expect_keyword("try")?;
    
    // Skip whitespace/newlines after try
    while !parser.is_at_end() && 
          (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
           parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
        parser.advance();
    }
    
    let mut body = Vec::new();
    loop {
        // Skip whitespace/newlines before checking for statements
        while !parser.is_at_end() && 
              (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
               parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
            parser.advance();
        }
        
        // Check for catch, finally, or end BEFORE trying to parse a statement
        if parser.check_keyword("catch") || parser.check_keyword("finally") || parser.check_keyword("end") {
            break;
        }
        
        // Try to parse a statement
        match parser.parse_statement()? {
            Some(stmt) => body.push(stmt),
            None => {
                // parse_statement returned None - check if it's because of a terminating keyword
                // (parse_statement returns None for catch/finally/end without advancing)
                if parser.check_keyword("catch") || parser.check_keyword("finally") || parser.check_keyword("end") {
                    break;
                }
                // Otherwise, it might be end of input or an error
                break;
            }
        }
    }
    
    let mut catch = None;
    if parser.check_keyword("catch") {
        parser.advance();
        // Arrow is optional after catch
        if parser.check(crate::lexer::token::TokenKind::Arrow) {
            parser.advance();
        }
        // Skip whitespace after arrow
        while !parser.is_at_end() && 
              (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
               parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
            parser.advance();
        }
        let error_var = parser.expect_identifier()?;
        
        // Skip whitespace after identifier
        while !parser.is_at_end() && 
              (parser.peek().kind == crate::lexer::token::TokenKind::Newline ||
               parser.peek().kind == crate::lexer::token::TokenKind::Whitespace) {
            parser.advance();
        }
        
        let mut catch_body = Vec::new();
        while !parser.is_at_end() && !parser.check_keyword("finally") && !parser.check_keyword("end") {
            if let Some(stmt) = parser.parse_statement()? {
                catch_body.push(stmt);
            } else {
                break;
            }
        }
        catch = Some((error_var, catch_body));
    }
    
    let mut finally = None;
    if parser.check_keyword("finally") {
        parser.advance();
        let mut finally_body = Vec::new();
        while !parser.is_at_end() && !parser.check_keyword("end") {
            if let Some(stmt) = parser.parse_statement()? {
                finally_body.push(stmt);
            } else {
                break;
            }
        }
        finally = Some(finally_body);
    }
    
    parser.expect_keyword("end")?;
    let span = token_span_to_ast_span(&start_token);
    Ok(Some(Statement::Try {
        body,
        catch,
        finally,
        span,
    }))
}
