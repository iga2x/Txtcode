use crate::parser::ast::*;
use crate::parser::parser::Parser;

/// Parse expression starting from the lowest precedence (or)
pub fn parse_expression(parser: &mut Parser) -> Result<Expression, String> {
    parse_or(parser)
}

pub fn parse_or(parser: &mut Parser) -> Result<Expression, String> {
    let mut expr = parse_and(parser)?;

    while parser.check_keyword("or") || parser.check(crate::lexer::token::TokenKind::Or) {
        parser.advance();
        let right = parse_and(parser)?;
        expr = Expression::BinaryOp {
            left: Box::new(expr),
            op: BinaryOperator::Or,
            right: Box::new(right),
            span: Span::default(),
        };
    }

    Ok(expr)
}

pub fn parse_and(parser: &mut Parser) -> Result<Expression, String> {
    let mut expr = parse_null_coalesce(parser)?;

    while parser.check_keyword("and") || parser.check(crate::lexer::token::TokenKind::And) {
        parser.advance();
        let right = parse_null_coalesce(parser)?;
        expr = Expression::BinaryOp {
            left: Box::new(expr),
            op: BinaryOperator::And,
            right: Box::new(right),
            span: Span::default(),
        };
    }

    Ok(expr)
}

pub fn parse_null_coalesce(parser: &mut Parser) -> Result<Expression, String> {
    let mut expr = parse_equality(parser)?;
    
    while parser.check(crate::lexer::token::TokenKind::NullCoalesce) {
        parser.advance();
        let right = parse_equality(parser)?;
        expr = Expression::BinaryOp {
            left: Box::new(expr),
            op: BinaryOperator::NullCoalesce,
            right: Box::new(right),
            span: Span::default(),
        };
    }
    
    Ok(expr)
}

pub fn parse_equality(parser: &mut Parser) -> Result<Expression, String> {
    let mut expr = parse_comparison(parser)?;
    
    while parser.check(crate::lexer::token::TokenKind::Equal) || 
          parser.check(crate::lexer::token::TokenKind::NotEqual) {
        let op = if parser.check(crate::lexer::token::TokenKind::Equal) {
            parser.advance();
            BinaryOperator::Equal
        } else {
            parser.advance();
            BinaryOperator::NotEqual
        };
        let right = parse_comparison(parser)?;
        expr = Expression::BinaryOp {
            left: Box::new(expr),
            op: op,
            right: Box::new(right),
            span: Span::default(),
        };
    }
    
    Ok(expr)
}

pub fn parse_comparison(parser: &mut Parser) -> Result<Expression, String> {
    let mut expr = parse_bitwise_or(parser)?;
    
    while parser.check(crate::lexer::token::TokenKind::Less) ||
          parser.check(crate::lexer::token::TokenKind::Greater) ||
          parser.check(crate::lexer::token::TokenKind::LessEqual) ||
          parser.check(crate::lexer::token::TokenKind::GreaterEqual) {
        let op = match parser.peek().kind {
            crate::lexer::token::TokenKind::Less => {
                parser.advance();
                BinaryOperator::Less
            }
            crate::lexer::token::TokenKind::Greater => {
                parser.advance();
                BinaryOperator::Greater
            }
            crate::lexer::token::TokenKind::LessEqual => {
                parser.advance();
                BinaryOperator::LessEqual
            }
            crate::lexer::token::TokenKind::GreaterEqual => {
                parser.advance();
                BinaryOperator::GreaterEqual
            }
            _ => break,
        };
        let right = parse_bitwise_or(parser)?;
        expr = Expression::BinaryOp {
            left: Box::new(expr),
            op: op,
            right: Box::new(right),
            span: Span::default(),
        };
    }
    
    Ok(expr)
}

pub fn parse_bitwise_or(parser: &mut Parser) -> Result<Expression, String> {
    let mut expr = parse_bitwise_xor(parser)?;
    
    while parser.check(crate::lexer::token::TokenKind::BitOr) {
        parser.advance();
        let right = parse_bitwise_xor(parser)?;
        expr = Expression::BinaryOp {
            left: Box::new(expr),
            op: BinaryOperator::BitwiseOr,
            right: Box::new(right),
            span: Span::default(),
        };
    }
    
    Ok(expr)
}

pub fn parse_bitwise_xor(parser: &mut Parser) -> Result<Expression, String> {
    let mut expr = parse_bitwise_and(parser)?;
    
    while parser.check(crate::lexer::token::TokenKind::BitXor) {
        parser.advance();
        let right = parse_bitwise_and(parser)?;
        expr = Expression::BinaryOp {
            left: Box::new(expr),
            op: BinaryOperator::BitwiseXor,
            right: Box::new(right),
            span: Span::default(),
        };
    }
    
    Ok(expr)
}

pub fn parse_bitwise_and(parser: &mut Parser) -> Result<Expression, String> {
    let mut expr = parse_shift(parser)?;
    
    while parser.check(crate::lexer::token::TokenKind::BitAnd) {
        parser.advance();
        let right = parse_shift(parser)?;
        expr = Expression::BinaryOp {
            left: Box::new(expr),
            op: BinaryOperator::BitwiseAnd,
            right: Box::new(right),
            span: Span::default(),
        };
    }
    
    Ok(expr)
}

pub fn parse_shift(parser: &mut Parser) -> Result<Expression, String> {
    let mut expr = parse_term(parser)?;
    
    while parser.check(crate::lexer::token::TokenKind::LeftShift) ||
          parser.check(crate::lexer::token::TokenKind::RightShift) {
        let op = if parser.check(crate::lexer::token::TokenKind::LeftShift) {
            parser.advance();
            BinaryOperator::LeftShift
        } else {
            parser.advance();
            BinaryOperator::RightShift
        };
        let right = parse_term(parser)?;
        expr = Expression::BinaryOp {
            left: Box::new(expr),
            op: op,
            right: Box::new(right),
            span: Span::default(),
        };
    }
    
    Ok(expr)
}

pub fn parse_term(parser: &mut Parser) -> Result<Expression, String> {
    let mut expr = parse_factor(parser)?;
    
    while parser.check(crate::lexer::token::TokenKind::Plus) ||
          parser.check(crate::lexer::token::TokenKind::Minus) {
        let op = if parser.check(crate::lexer::token::TokenKind::Plus) {
            parser.advance();
            BinaryOperator::Add
        } else {
            parser.advance();
            BinaryOperator::Subtract
        };
        let right = parse_factor(parser)?;
        expr = Expression::BinaryOp {
            left: Box::new(expr),
            op: op,
            right: Box::new(right),
            span: Span::default(),
        };
    }
    
    Ok(expr)
}

pub fn parse_factor(parser: &mut Parser) -> Result<Expression, String> {
    // Power operator has higher precedence and is right-associative
    let mut expr = parse_unary(parser)?;
    
    // Parse power operator (right-associative: 2 ** 3 ** 4 = 2 ** (3 ** 4))
    if parser.check(crate::lexer::token::TokenKind::Power) {
        parser.advance();
        let right = parse_factor(parser)?; // Right-associative: recurse
        expr = Expression::BinaryOp {
            left: Box::new(expr),
            op: BinaryOperator::Power,
            right: Box::new(right),
            span: Span::default(),
        };
    }
    
    // Parse multiplication, division, modulo (left-associative)
    while parser.check(crate::lexer::token::TokenKind::Star) ||
          parser.check(crate::lexer::token::TokenKind::Slash) ||
          parser.check(crate::lexer::token::TokenKind::Percent) {
        let op = match parser.peek().kind {
            crate::lexer::token::TokenKind::Star => {
                parser.advance();
                BinaryOperator::Multiply
            }
            crate::lexer::token::TokenKind::Slash => {
                parser.advance();
                BinaryOperator::Divide
            }
            crate::lexer::token::TokenKind::Percent => {
                parser.advance();
                BinaryOperator::Modulo
            }
            _ => break,
        };
        let right = parse_unary(parser)?;
        expr = Expression::BinaryOp {
            left: Box::new(expr),
            op: op,
            right: Box::new(right),
            span: Span::default(),
        };
    }
    
    Ok(expr)
}

pub fn parse_unary(parser: &mut Parser) -> Result<Expression, String> {
    use crate::parser::utils::token_span_to_ast_span;
    
    // Parse await expression: await -> expression
    if parser.check_keyword("await") {
        let token = parser.peek().clone();
        let span = token_span_to_ast_span(&token);
        parser.advance();
        
        // Optional arrow after await
        if parser.check(crate::lexer::token::TokenKind::Arrow) {
            parser.advance();
        }
        
        let expr = parse_unary(parser)?; // Recursive to allow await await (though unlikely)
        return Ok(Expression::Await {
            expression: Box::new(expr),
            span,
        });
    }
    
    if parser.check_keyword("not") {
        parser.advance();
        let expr = parse_unary(parser)?;
        return Ok(Expression::UnaryOp {
            op: UnaryOperator::Not,
            operand: Box::new(expr),
            span: Span::default(),
        });
    }
    
    if parser.check(crate::lexer::token::TokenKind::Minus) {
        parser.advance();
        let expr = parse_unary(parser)?;
        return Ok(Expression::UnaryOp {
            op: UnaryOperator::Minus,
            operand: Box::new(expr),
            span: Span::default(),
        });
    }
    
    if parser.check(crate::lexer::token::TokenKind::BitNot) {
        parser.advance();
        let expr = parse_unary(parser)?;
        return Ok(Expression::UnaryOp {
            op: UnaryOperator::BitNot,
            operand: Box::new(expr),
            span: Span::default(),
        });
    }
    
    // Parse increment/decrement (prefix: ++x, --x)
    if parser.check(crate::lexer::token::TokenKind::Increment) {
        parser.advance();
        let expr = parse_unary(parser)?;
        return Ok(Expression::UnaryOp {
            op: UnaryOperator::Increment,
            operand: Box::new(expr),
            span: Span::default(),
        });
    }
    
    if parser.check(crate::lexer::token::TokenKind::Decrement) {
        parser.advance();
        let expr = parse_unary(parser)?;
        return Ok(Expression::UnaryOp {
            op: UnaryOperator::Decrement,
            operand: Box::new(expr),
            span: Span::default(),
        });
    }
    
    crate::parser::expressions::calls::parse_call(parser)
}

