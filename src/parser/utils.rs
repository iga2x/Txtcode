use crate::lexer::token::Token;
use crate::parser::ast::*;

/// Parse a string as an expression (used for string interpolation)
fn parse_interpolated_expr(expr_str: &str) -> Expression {
    // Tokenize and parse the expression string
    let mut lexer = crate::lexer::lexer::Lexer::new(expr_str.to_string());
    match lexer.tokenize() {
        Ok(tokens) => {
            let mut parser = crate::parser::parser::Parser::new(tokens);
            match parser.parse_expression() {
                Ok(expr) => expr,
                Err(_) => Expression::Identifier(expr_str.to_string()),
            }
        }
        Err(_) => Expression::Identifier(expr_str.to_string()),
    }
}

pub fn token_span_to_ast_span(token: &Token) -> Span {
    // Token span is (line, column)
    Span {
        start: token.span.1,  // column as start position
        end: token.span.1,    // column as end position (same for single token)
        line: token.span.0,   // line number
        column: token.span.1, // column number
    }
}

pub fn parse_interpolated_string(
    _parser: &mut crate::parser::parser::Parser,
    value: &str,
    span: Span,
) -> Result<Expression, String> {
    use crate::parser::ast::InterpolatedSegment;

    let mut segments = Vec::new();
    let mut current_text = String::new();
    let mut chars = value.chars().peekable();
    let mut in_expression = false;
    let mut expr_chars = String::new();
    // Track nested brace depth inside an interpolated expression so that
    // `f"{fn({k: v})}"` doesn't close on the inner `}`.
    let mut brace_depth: usize = 0;

    while let Some(ch) = chars.next() {
        if ch == '\x01' {
            // Sentinel for \{ (escaped literal brace) — always a literal {, never
            // an interpolation start.
            if in_expression {
                expr_chars.push('{');
            } else {
                current_text.push('{');
            }
        } else if ch == '\x02' {
            // Sentinel for \} (escaped literal brace) — always a literal }, never
            // an interpolation end.
            if in_expression {
                expr_chars.push('}');
            } else {
                current_text.push('}');
            }
        } else if ch == '\\' {
            // Handle remaining escape sequences (non-brace)
            if let Some(next) = chars.next() {
                if in_expression {
                    expr_chars.push('\\');
                    expr_chars.push(next);
                } else {
                    current_text.push(match next {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        '\\' => '\\',
                        '"' => '"',
                        '\'' => '\'',
                        _ => next,
                    });
                }
            }
        } else if ch == '{' && !in_expression {
            // Start of interpolation
            if !current_text.is_empty() {
                segments.push(InterpolatedSegment::Text(current_text.clone()));
                current_text.clear();
            }
            in_expression = true;
            brace_depth = 0;
            expr_chars.clear();
        } else if ch == '{' && in_expression {
            // Nested brace inside an expression (e.g. map literal or struct)
            brace_depth += 1;
            expr_chars.push(ch);
        } else if ch == '}' && in_expression {
            if brace_depth > 0 {
                // Still inside a nested brace group — not the end of interpolation
                brace_depth -= 1;
                expr_chars.push(ch);
            } else {
                // End of interpolation - parse expression
                let expr_str = expr_chars.trim();
                if !expr_str.is_empty() {
                    let parsed_expr = parse_interpolated_expr(expr_str);
                    segments.push(InterpolatedSegment::Expression(parsed_expr));
                }
                in_expression = false;
                expr_chars.clear();
            }
        } else if in_expression {
            expr_chars.push(ch);
        } else {
            current_text.push(ch);
        }
    }

    // Add remaining text
    if !current_text.is_empty() {
        segments.push(InterpolatedSegment::Text(current_text));
    }

    // If no segments or only text, return as regular string
    if segments.is_empty() {
        Ok(Expression::Literal(Literal::String(String::new())))
    } else if segments.len() == 1 {
        match &segments[0] {
            InterpolatedSegment::Text(s) => Ok(Expression::Literal(Literal::String(s.clone()))),
            _ => Ok(Expression::InterpolatedString { segments, span }),
        }
    } else {
        Ok(Expression::InterpolatedString { segments, span })
    }
}
