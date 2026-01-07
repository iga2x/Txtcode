use crate::lexer::{Token, TokenKind, Span};
use crate::parser::ast::*;

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at line {}, column {}", self.message, self.span.line, self.span.column)
    }
}

impl std::error::Error for ParseError {}

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    pub fn parse(&mut self) -> Result<Program, ParseError> {
        let mut statements = Vec::new();
        let start_span = self.peek().map(|t| t.span.clone()).unwrap_or_else(|| Span::new(0, 0, 1, 1));

        while !self.is_at_end() {
            // Skip newlines between statements
            if self.check(&TokenKind::Newline) {
                self.advance();
                continue;
            }
            statements.push(self.parse_statement()?);
        }

        let end_span = if self.current > 0 {
            self.previous().span.clone()
        } else {
            Span::new(0, 0, 1, 1)
        };
        let span = start_span.merge(&end_span);

        Ok(Program { statements, span })
    }

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        if self.check(&TokenKind::Store) {
            self.parse_assignment()
        } else if self.check(&TokenKind::Define) {
            self.parse_function_def()
        } else if self.check(&TokenKind::Return) {
            self.parse_return()
        } else if self.check(&TokenKind::If) {
            self.parse_if()
        } else if self.check(&TokenKind::While) {
            self.parse_while()
        } else if self.check(&TokenKind::For) {
            self.parse_for()
        } else if self.check(&TokenKind::Repeat) {
            self.parse_repeat()
        } else if self.check(&TokenKind::Match) {
            self.parse_match()
        } else if self.check(&TokenKind::Break) {
            self.parse_break()
        } else if self.check(&TokenKind::Continue) {
            self.parse_continue()
        } else if self.check(&TokenKind::Try) {
            self.parse_try()
        } else if self.check(&TokenKind::Import) {
            self.parse_import()
        } else if self.check(&TokenKind::Assert) {
            self.parse_assert()
        } else {
            // Expression statement
            let expr = self.parse_expression()?;
            Ok(Statement::Expression(expr))
        }
    }

    fn parse_assignment(&mut self) -> Result<Statement, ParseError> {
        let start_span = self.advance().span.clone(); // consume 'store'
        
        // Optional arrow
        if self.check(&TokenKind::Arrow) {
            self.advance();
        }

        let name = match &self.advance().kind {
            TokenKind::Identifier(name) => name.clone(),
            _ => return Err(self.error("Expected identifier after 'store'")),
        };

        // Check for compound assignment operators
        let compound_op = match self.current_token().kind {
            TokenKind::PlusEqual => Some(BinaryOperator::Add),
            TokenKind::MinusEqual => Some(BinaryOperator::Subtract),
            TokenKind::StarEqual => Some(BinaryOperator::Multiply),
            TokenKind::SlashEqual => Some(BinaryOperator::Divide),
            TokenKind::PercentEqual => Some(BinaryOperator::Modulo),
            TokenKind::PowerEqual => Some(BinaryOperator::Power),
            _ => None,
        };

        if let Some(op) = compound_op {
            self.advance(); // consume compound operator
            // Optional arrow
            if self.check(&TokenKind::Arrow) {
                self.advance();
            }
            let value = self.parse_expression()?;
            let span = start_span.merge(&value.span());
            return Ok(Statement::CompoundAssignment {
                name,
                op,
                value,
                span,
            });
        }

        let type_annotation = if self.check(&TokenKind::Colon) {
            self.advance(); // consume ':'
            Some(self.parse_type()?)
        } else {
            None
        };

        // Optional arrow
        if self.check(&TokenKind::Arrow) {
            self.advance();
        }

        let value = self.parse_expression()?;
        let span = start_span.merge(&value.span());

        Ok(Statement::Assignment {
            name,
            type_annotation,
            value,
            span,
        })
    }

    fn parse_function_def(&mut self) -> Result<Statement, ParseError> {
        let start_span = self.advance().span.clone(); // consume 'define'

        // Optional arrow
        if self.check(&TokenKind::Arrow) {
            self.advance();
        }

        let name = match &self.advance().kind {
            TokenKind::Identifier(name) => name.clone(),
            _ => return Err(self.error("Expected function name after 'define'")),
        };

        // Optional arrow
        if self.check(&TokenKind::Arrow) {
            self.advance();
        }

        self.consume(&TokenKind::LeftParen, "Expected '(' after function name")?;
        
        let mut params = Vec::new();
        if !self.check(&TokenKind::RightParen) {
            loop {
                let param_name = match &self.advance().kind {
                    TokenKind::Identifier(name) => name.clone(),
                    _ => return Err(self.error("Expected parameter name")),
                };

                let param_type = if self.check(&TokenKind::Colon) {
                    self.advance();
                    Some(self.parse_type()?)
                } else {
                    None
                };

                params.push(Parameter {
                    name: param_name,
                    type_annotation: param_type,
                    span: Span::new(0, 0, 1, 1), // TODO: track actual span
                });

                if !self.check(&TokenKind::Comma) {
                    break;
                }
                self.advance();
            }
        }
        self.advance(); // consume ')'

        let return_type = if self.check(&TokenKind::Arrow) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        let mut body = Vec::new();
        while !self.check(&TokenKind::End) && !self.is_at_end() {
            body.push(self.parse_statement()?);
        }

        self.consume(&TokenKind::End, "Expected 'end' after function body")?;

        let span = start_span.merge(&self.previous().span);
        Ok(Statement::FunctionDef {
            name,
            params,
            return_type,
            body,
            span,
        })
    }

    fn parse_return(&mut self) -> Result<Statement, ParseError> {
        let start_span = self.advance().span.clone(); // consume 'return'

        // Optional arrow
        if self.check(&TokenKind::Arrow) {
            self.advance();
        }

        let value = if !self.check(&TokenKind::Newline) && !self.check(&TokenKind::Eof) && !self.check(&TokenKind::End) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        let span = value.as_ref().map(|v| start_span.merge(&v.span())).unwrap_or(start_span);
        Ok(Statement::Return { value, span })
    }

    fn parse_if(&mut self) -> Result<Statement, ParseError> {
        let start_span = self.advance().span.clone(); // consume 'if'

        // Optional arrow
        if self.check(&TokenKind::Arrow) {
            self.advance();
        }

        let condition = self.parse_expression()?;
        let mut then_branch = Vec::new();

        while !self.check(&TokenKind::Else) && !self.check(&TokenKind::Elseif) && !self.check(&TokenKind::End) && !self.is_at_end() {
            then_branch.push(self.parse_statement()?);
        }

        let mut else_if_branches = Vec::new();
        while self.check(&TokenKind::Elseif) {
            self.advance(); // consume 'elseif'
            if self.check(&TokenKind::Arrow) {
                self.advance();
            }
            let cond = self.parse_expression()?;
            let mut branch = Vec::new();
            while !self.check(&TokenKind::Else) && !self.check(&TokenKind::Elseif) && !self.check(&TokenKind::End) && !self.is_at_end() {
                branch.push(self.parse_statement()?);
            }
            else_if_branches.push((cond, branch));
        }

        let else_branch = if self.check(&TokenKind::Else) {
            self.advance(); // consume 'else'
            let mut branch = Vec::new();
            while !self.check(&TokenKind::End) && !self.is_at_end() {
                branch.push(self.parse_statement()?);
            }
            Some(branch)
        } else {
            None
        };

        self.consume(&TokenKind::End, "Expected 'end' after if statement")?;

        let span = start_span.merge(&self.previous().span);
        Ok(Statement::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
            span,
        })
    }

    fn parse_while(&mut self) -> Result<Statement, ParseError> {
        let start_span = self.advance().span.clone(); // consume 'while'

        if self.check(&TokenKind::Arrow) {
            self.advance();
        }

        let condition = self.parse_expression()?;
        let mut body = Vec::new();

        while !self.check(&TokenKind::End) && !self.is_at_end() {
            body.push(self.parse_statement()?);
        }

        self.consume(&TokenKind::End, "Expected 'end' after while loop")?;

        let span = start_span.merge(&self.previous().span);
        Ok(Statement::While {
            condition,
            body,
            span,
        })
    }

    fn parse_for(&mut self) -> Result<Statement, ParseError> {
        let start_span = self.advance().span.clone(); // consume 'for'

        if self.check(&TokenKind::Arrow) {
            self.advance();
        }

        let variable = match &self.advance().kind {
            TokenKind::Identifier(name) => name.clone(),
            _ => return Err(self.error("Expected variable name after 'for'")),
        };

        self.consume(&TokenKind::In, "Expected 'in' after for variable")?;
        let iterable = self.parse_expression()?;

        let mut body = Vec::new();
        while !self.check(&TokenKind::End) && !self.is_at_end() {
            body.push(self.parse_statement()?);
        }

        self.consume(&TokenKind::End, "Expected 'end' after for loop")?;

        let span = start_span.merge(&self.previous().span);
        Ok(Statement::For {
            variable,
            iterable,
            body,
            span,
        })
    }

    fn parse_repeat(&mut self) -> Result<Statement, ParseError> {
        let start_span = self.advance().span.clone(); // consume 'repeat'

        if self.check(&TokenKind::Arrow) {
            self.advance();
        }

        let count = self.parse_expression()?;
        self.consume(&TokenKind::Times, "Expected 'times' after repeat count")?;

        let mut body = Vec::new();
        while !self.check(&TokenKind::End) && !self.is_at_end() {
            body.push(self.parse_statement()?);
        }

        self.consume(&TokenKind::End, "Expected 'end' after repeat loop")?;

        let span = start_span.merge(&self.previous().span);
        Ok(Statement::Repeat {
            count,
            body,
            span,
        })
    }

    fn parse_match(&mut self) -> Result<Statement, ParseError> {
        let start_span = self.advance().span.clone(); // consume 'match'

        if self.check(&TokenKind::Arrow) {
            self.advance();
        }

        let value = self.parse_expression()?;
        let mut cases = Vec::new();
        let mut default = None;

        while !self.check(&TokenKind::End) && !self.is_at_end() {
            if self.check(&TokenKind::Case) {
                self.advance(); // consume 'case'
                if self.check(&TokenKind::Arrow) {
                    self.advance();
                }

                let pattern = if let Some(token) = self.peek() {
                    match &token.kind {
                        TokenKind::Identifier(_) => {
                            Pattern::Identifier(match &self.advance().kind {
                                TokenKind::Identifier(name) => name.clone(),
                                _ => unreachable!(),
                            })
                        }
                        TokenKind::Integer(_) | TokenKind::Float(_) | 
                        TokenKind::String(_) | TokenKind::Boolean(_) => {
                            Pattern::Literal(self.parse_expression()?)
                        }
                        _ => {
                            return Err(self.error("Expected pattern in case"));
                        }
                    }
                } else {
                    return Err(self.error("Expected pattern in case"));
                };

                let guard = if self.check(&TokenKind::If) {
                    self.advance();
                    Some(self.parse_expression()?)
                } else {
                    None
                };

                let mut body = Vec::new();
                while !self.check(&TokenKind::Case) && !self.check(&TokenKind::End) && !self.is_at_end() {
                    body.push(self.parse_statement()?);
                }

                cases.push(MatchCase {
                    pattern,
                    guard,
                    body,
                    span: Span::new(0, 0, 1, 1), // TODO: track actual span
                });
            } else if let Some(token) = self.peek() {
                // Check for default case (wildcard)
                if let TokenKind::Identifier(name) = &token.kind {
                    if name == "_" {
                        self.advance();
                        if self.check(&TokenKind::Arrow) {
                            self.advance();
                        }
                        let mut body = Vec::new();
                        while !self.check(&TokenKind::End) && !self.is_at_end() {
                            body.push(self.parse_statement()?);
                        }
                        default = Some(body);
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        self.consume(&TokenKind::End, "Expected 'end' after match statement")?;

        let span = start_span.merge(&self.previous().span);
        Ok(Statement::Match {
            value,
            cases,
            default,
            span,
        })
    }

    fn parse_break(&mut self) -> Result<Statement, ParseError> {
        let span = self.advance().span.clone();
        Ok(Statement::Break { span })
    }

    fn parse_continue(&mut self) -> Result<Statement, ParseError> {
        let span = self.advance().span.clone();
        Ok(Statement::Continue { span })
    }

    fn parse_try(&mut self) -> Result<Statement, ParseError> {
        let start_span = self.advance().span.clone(); // consume 'try'

        let mut body = Vec::new();
        while !self.check(&TokenKind::Catch) && !self.is_at_end() {
            body.push(self.parse_statement()?);
        }

        let catch = if self.check(&TokenKind::Catch) {
            self.advance(); // consume 'catch'
            if self.check(&TokenKind::Arrow) {
                self.advance();
            }

            let error_var = match &self.advance().kind {
                TokenKind::Identifier(name) => name.clone(),
                _ => return Err(self.error("Expected error variable name after 'catch'")),
            };

            let mut catch_body = Vec::new();
            while !self.check(&TokenKind::End) && !self.is_at_end() {
                catch_body.push(self.parse_statement()?);
            }

            Some((error_var, catch_body))
        } else {
            None
        };

        if catch.is_some() {
            self.consume(&TokenKind::End, "Expected 'end' after try-catch")?;
        }

        let span = start_span.merge(&self.previous().span);
        Ok(Statement::Try {
            body,
            catch,
            span,
        })
    }

    fn parse_assert(&mut self) -> Result<Statement, ParseError> {
        let start_span = self.advance().span.clone(); // consume 'assert'
        
        // Optional arrow
        if self.check(&TokenKind::Arrow) {
            self.advance();
        }
        
        let condition = self.parse_expression()?;
        
        // Optional message (comma-separated)
        let message = if self.check(&TokenKind::Comma) {
            self.advance(); // consume ','
            Some(self.parse_expression()?)
        } else {
            None
        };
        
        let end_span = if let Some(ref msg) = message {
            msg.span()
        } else {
            condition.span()
        };
        let span = start_span.merge(&end_span);
        
        Ok(Statement::Assert {
            condition,
            message,
            span,
        })
    }

    fn parse_import(&mut self) -> Result<Statement, ParseError> {
        let start_span = self.advance().span.clone(); // consume 'import'

        if self.check(&TokenKind::Arrow) {
            self.advance();
        }

        let mut items = Vec::new();
        loop {
            match &self.advance().kind {
                TokenKind::Identifier(name) => items.push(name.clone()),
                _ => return Err(self.error("Expected identifier in import")),
            }

            if !self.check(&TokenKind::Comma) {
                break;
            }
            self.advance(); // consume ','
        }

        let from = if self.check(&TokenKind::From) {
            self.advance();
            match &self.advance().kind {
                TokenKind::Identifier(name) | TokenKind::String(name) => Some(name.clone()),
                _ => return Err(self.error("Expected module name after 'from'")),
            }
        } else {
            None
        };

        let alias = if self.check(&TokenKind::As) {
            self.advance();
            match &self.advance().kind {
                TokenKind::Identifier(name) => Some(name.clone()),
                _ => return Err(self.error("Expected alias name after 'as'")),
            }
        } else {
            None
        };

        let span = start_span.merge(&self.previous().span);
        Ok(Statement::Import {
            items,
            from,
            alias,
            span,
        })
    }

    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        self.parse_ternary()
    }

    fn parse_ternary(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_or()?;

        if self.check(&TokenKind::Question) {
            let start_span = expr.span();
            self.advance(); // consume '?'
            let true_expr = self.parse_ternary()?; // Right-associative
            self.consume(&TokenKind::Colon, "Expected ':' in ternary operator")?;
            let false_expr = self.parse_ternary()?;
            let end_span = false_expr.span();
            let span = start_span.merge(&end_span);
            expr = Expression::Ternary {
                condition: Box::new(expr),
                true_expr: Box::new(true_expr),
                false_expr: Box::new(false_expr),
                span,
            };
        }

        Ok(expr)
    }

    fn parse_or(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_and()?;

        while self.check(&TokenKind::Or) {
            let op = match self.advance().kind {
                TokenKind::Or => BinaryOperator::Or,
                _ => unreachable!(),
            };
            let right = self.parse_and()?;
            let span = expr.span().merge(&right.span());
            expr = Expression::BinaryOp {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span,
            };
        }

        Ok(expr)
    }

    fn parse_and(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_equality()?;

        while self.check(&TokenKind::And) {
            let op = match self.advance().kind {
                TokenKind::And => BinaryOperator::And,
                _ => unreachable!(),
            };
            let right = self.parse_equality()?;
            let span = expr.span().merge(&right.span());
            expr = Expression::BinaryOp {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span,
            };
        }

        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_comparison()?;

        while self.check(&TokenKind::Equal) || self.check(&TokenKind::NotEqual) {
            let op = match self.advance().kind {
                TokenKind::Equal => BinaryOperator::Equal,
                TokenKind::NotEqual => BinaryOperator::NotEqual,
                _ => unreachable!(),
            };
            let right = self.parse_comparison()?;
            let span = expr.span().merge(&right.span());
            expr = Expression::BinaryOp {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span,
            };
        }

        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_bitwise_or()?;

        while self.check(&TokenKind::Less) || self.check(&TokenKind::Greater) ||
              self.check(&TokenKind::LessEqual) || self.check(&TokenKind::GreaterEqual) {
            let op = match self.advance().kind {
                TokenKind::Less => BinaryOperator::Less,
                TokenKind::Greater => BinaryOperator::Greater,
                TokenKind::LessEqual => BinaryOperator::LessEqual,
                TokenKind::GreaterEqual => BinaryOperator::GreaterEqual,
                _ => unreachable!(),
            };
            let right = self.parse_bitwise_or()?;
            let span = expr.span().merge(&right.span());
            expr = Expression::BinaryOp {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span,
            };
        }

        Ok(expr)
    }

    fn parse_bitwise_or(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_bitwise_xor()?;

        while self.check(&TokenKind::BitOr) {
            let op = BinaryOperator::BitOr;
            self.advance();
            let right = self.parse_bitwise_xor()?;
            let span = expr.span().merge(&right.span());
            expr = Expression::BinaryOp {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span,
            };
        }

        Ok(expr)
    }

    fn parse_bitwise_xor(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_bitwise_and()?;

        while self.check(&TokenKind::BitXor) {
            let op = BinaryOperator::BitXor;
            self.advance();
            let right = self.parse_bitwise_and()?;
            let span = expr.span().merge(&right.span());
            expr = Expression::BinaryOp {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span,
            };
        }

        Ok(expr)
    }

    fn parse_bitwise_and(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_shift()?;

        while self.check(&TokenKind::BitAnd) {
            let op = BinaryOperator::BitAnd;
            self.advance();
            let right = self.parse_shift()?;
            let span = expr.span().merge(&right.span());
            expr = Expression::BinaryOp {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span,
            };
        }

        Ok(expr)
    }

    fn parse_shift(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_additive()?;

        while self.check(&TokenKind::LeftShift) || self.check(&TokenKind::RightShift) {
            let op = match self.advance().kind {
                TokenKind::LeftShift => BinaryOperator::LeftShift,
                TokenKind::RightShift => BinaryOperator::RightShift,
                _ => unreachable!(),
            };
            let right = self.parse_additive()?;
            let span = expr.span().merge(&right.span());
            expr = Expression::BinaryOp {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span,
            };
        }

        Ok(expr)
    }

    fn parse_additive(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_multiplicative()?;

        while !self.check(&TokenKind::Newline) && (self.check(&TokenKind::Plus) || self.check(&TokenKind::Minus)) {
            let op = match self.advance().kind {
                TokenKind::Plus => BinaryOperator::Add,
                TokenKind::Minus => BinaryOperator::Subtract,
                _ => unreachable!(),
            };
            // Stop if we hit a newline before parsing the right side
            if self.check(&TokenKind::Newline) || self.check(&TokenKind::Eof) {
                return Err(ParseError {
                    message: "Expected expression after operator".to_string(),
                    span: self.tokens[self.current].span.clone(),
                });
            }
            let right = self.parse_multiplicative()?;
            let span = expr.span().merge(&right.span());
            expr = Expression::BinaryOp {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span,
            };
        }

        Ok(expr)
    }

    fn parse_multiplicative(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_unary()?;

        while !self.check(&TokenKind::Newline) && (self.check(&TokenKind::Star) || self.check(&TokenKind::Slash) || self.check(&TokenKind::Percent)) {
            let op = match self.advance().kind {
                TokenKind::Star => BinaryOperator::Multiply,
                TokenKind::Slash => BinaryOperator::Divide,
                TokenKind::Percent => BinaryOperator::Modulo,
                _ => unreachable!(),
            };
            // Stop if we hit a newline before parsing the right side
            if self.check(&TokenKind::Newline) || self.check(&TokenKind::Eof) {
                return Err(ParseError {
                    message: "Expected expression after operator".to_string(),
                    span: self.tokens[self.current].span.clone(),
                });
            }
            let right = self.parse_unary()?;
            let span = expr.span().merge(&right.span());
            expr = Expression::BinaryOp {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span,
            };
        }

        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expression, ParseError> {
        if self.check(&TokenKind::Not) || self.check(&TokenKind::Minus) || self.check(&TokenKind::BitNot) {
            let op = match self.advance().kind {
                TokenKind::Not => UnaryOperator::Not,
                TokenKind::Minus => UnaryOperator::Minus,
                TokenKind::BitNot => UnaryOperator::BitNot,
                _ => unreachable!(),
            };
            let operand = self.parse_unary()?;
            let span = operand.span();
            Ok(Expression::UnaryOp {
                op,
                operand: Box::new(operand),
                span,
            })
        } else {
            self.parse_power()
        }
    }

    fn parse_power(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_call()?;

        while self.check(&TokenKind::Power) {
            let op = BinaryOperator::Power;
            self.advance();
            let right = self.parse_unary()?; // Right-associative
            let span = expr.span().merge(&right.span());
            expr = Expression::BinaryOp {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span,
            };
        }

        Ok(expr)
    }

    fn parse_call(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_primary()?;

        loop {
            // Stop if we hit a newline
            if self.check(&TokenKind::Newline) || self.check(&TokenKind::Eof) {
                break;
            }
            if self.check(&TokenKind::LeftParen) {
                expr = self.finish_call(expr)?;
            } else if self.check(&TokenKind::Arrow) {
                // Arrow-based function call: identifier -> expression
                // This supports syntax like: print -> "Hello"
                if let Expression::Identifier(name) = expr {
                    // Get span from the identifier token (consumed in parse_primary)
                    let start_span = self.previous().span.clone();
                    expr = self.finish_arrow_call(name, start_span)?;
                    // After arrow call, always break (arrow calls are complete expressions)
                    break;
                } else {
                    // Arrow as binary operator (for other contexts)
                    break;
                }
            } else if self.check(&TokenKind::LeftBracket) {
                expr = self.parse_index(expr)?;
            } else if self.check(&TokenKind::Dot) {
                expr = self.parse_member(expr)?;
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn finish_call(&mut self, callee: Expression) -> Result<Expression, ParseError> {
        let start_span = self.advance().span.clone(); // consume '('

        let mut arguments = Vec::new();
        if !self.check(&TokenKind::RightParen) {
            loop {
                arguments.push(self.parse_expression()?);
                if !self.check(&TokenKind::Comma) {
                    break;
                }
                self.advance();
            }
        }

        let end_span = self.consume(&TokenKind::RightParen, "Expected ')' after arguments")?.span.clone();
        let span = start_span.merge(&end_span);

        let name = match &callee {
            Expression::Identifier(name) => name.clone(),
            _ => return Err(self.error("Expected function name")),
        };

        Ok(Expression::FunctionCall {
            name,
            arguments,
            span,
        })
    }

    fn finish_arrow_call(&mut self, name: String, start_span: Span) -> Result<Expression, ParseError> {
        self.advance(); // consume '->'
        
        // Stop if we hit a newline (no argument provided)
        if self.check(&TokenKind::Newline) || self.check(&TokenKind::Eof) {
            return Err(ParseError {
                message: "Expected expression after '->'".to_string(),
                span: self.tokens[self.current].span.clone(),
            });
        }
        
        // Parse comma-separated arguments (like Python's print)
        let mut arguments = Vec::new();
        
        // Parse first argument
        arguments.push(self.parse_expression()?);
        
        // Parse additional comma-separated arguments
        while self.check(&TokenKind::Comma) {
            self.advance(); // consume ','
            // Stop if we hit a newline after comma (trailing comma not allowed)
            if self.check(&TokenKind::Newline) || self.check(&TokenKind::Eof) {
                return Err(ParseError {
                    message: "Expected expression after ','".to_string(),
                    span: self.tokens[self.current].span.clone(),
                });
            }
            arguments.push(self.parse_expression()?);
        }
        
        let end_span = if let Some(last_arg) = arguments.last() {
            last_arg.span()
        } else {
            start_span.clone()
        };
        let span = start_span.merge(&end_span);

        Ok(Expression::FunctionCall {
            name,
            arguments,
            span,
        })
    }

    fn parse_index(&mut self, target: Expression) -> Result<Expression, ParseError> {
        let start_span = self.advance().span.clone(); // consume '['
        
        // Check if this is a slice (contains colon)
        if self.check(&TokenKind::Colon) {
            // Slice: [:] or [:end] or [start:] or [start:end]
            self.advance(); // consume ':'
            let end = if !self.check(&TokenKind::RightBracket) {
                Some(Box::new(self.parse_expression()?))
            } else {
                None
            };
            let end_span = self.consume(&TokenKind::RightBracket, "Expected ']' after slice")?.span.clone();
            let span = start_span.merge(&end_span);
            return Ok(Expression::Slice {
                target: Box::new(target),
                start: None,
                end,
                span,
            });
        }
        
        // Regular index or slice starting with start
        let first_expr = self.parse_expression()?;
        
        if self.check(&TokenKind::Colon) {
            // Slice: [start:] or [start:end]
            self.advance(); // consume ':'
            let end = if !self.check(&TokenKind::RightBracket) {
                Some(Box::new(self.parse_expression()?))
            } else {
                None
            };
            let end_span = self.consume(&TokenKind::RightBracket, "Expected ']' after slice")?.span.clone();
            let span = start_span.merge(&end_span);
            Ok(Expression::Slice {
                target: Box::new(target),
                start: Some(Box::new(first_expr)),
                end,
                span,
            })
        } else {
            // Regular index
            let end_span = self.consume(&TokenKind::RightBracket, "Expected ']' after index")?.span.clone();
            let span = start_span.merge(&end_span);
            Ok(Expression::Index {
                target: Box::new(target),
                index: Box::new(first_expr),
                span,
            })
        }
    }

    fn parse_member(&mut self, target: Expression) -> Result<Expression, ParseError> {
        let start_span = self.advance().span.clone(); // consume '.'
        let member = match &self.advance().kind {
            TokenKind::Identifier(name) => name.clone(),
            _ => return Err(self.error("Expected member name after '.'")),
        };
        let span = start_span.merge(&self.previous().span);

        Ok(Expression::Member {
            target: Box::new(target),
            member,
            span,
        })
    }

    fn parse_primary(&mut self) -> Result<Expression, ParseError> {
        let token = self.advance();

        match &token.kind {
            TokenKind::Integer(n) => Ok(Expression::Literal(Literal::Integer(*n))),
            TokenKind::Float(n) => Ok(Expression::Literal(Literal::Float(*n))),
            TokenKind::String(s) => Ok(Expression::Literal(Literal::String(s.clone()))),
            TokenKind::Boolean(b) => Ok(Expression::Literal(Literal::Boolean(*b))),
            TokenKind::Null => Ok(Expression::Literal(Literal::Null)),
            TokenKind::Identifier(name) => Ok(Expression::Identifier(name.clone())),
            TokenKind::LeftParen => {
                let expr = self.parse_expression()?;
                self.consume(&TokenKind::RightParen, "Expected ')' after expression")?;
                Ok(expr)
            }
            TokenKind::LeftBracket => {
                let start_span = token.span.clone();
                let mut elements = Vec::new();
                if !self.check(&TokenKind::RightBracket) {
                    loop {
                        elements.push(self.parse_expression()?);
                        if !self.check(&TokenKind::Comma) {
                            break;
                        }
                        self.advance();
                    }
                }
                let end_span = self.consume(&TokenKind::RightBracket, "Expected ']' after array")?.span.clone();
                let span = start_span.merge(&end_span);
                Ok(Expression::Array { elements, span })
            }
            TokenKind::LeftBrace => {
                let start_span = token.span.clone();
                let mut entries = Vec::new();
                if !self.check(&TokenKind::RightBrace) {
                    loop {
                        let key = match &self.advance().kind {
                            TokenKind::String(s) | TokenKind::Identifier(s) => s.clone(),
                            _ => return Err(self.error("Expected string or identifier as map key")),
                        };
                        self.consume(&TokenKind::Colon, "Expected ':' after map key")?;
                        let value = self.parse_expression()?;
                        entries.push((key, value));
                        if !self.check(&TokenKind::Comma) {
                            break;
                        }
                        self.advance();
                    }
                }
                let end_span = self.consume(&TokenKind::RightBrace, "Expected '}' after map")?.span.clone();
                let span = start_span.merge(&end_span);
                Ok(Expression::Map { entries, span })
            }
            _ => Err(ParseError {
                message: format!("Unexpected token: {:?}", token.kind),
                span: token.span.clone(),
            }),
        }
    }

    fn parse_type(&mut self) -> Result<Type, ParseError> {
        match &self.advance().kind {
            TokenKind::Identifier(name) => {
                match name.as_str() {
                    "int" => Ok(Type::Int),
                    "float" => Ok(Type::Float),
                    "string" => Ok(Type::String),
                    "bool" => Ok(Type::Bool),
                    "array" => {
                        if self.check(&TokenKind::LeftBracket) {
                            self.advance();
                            let inner = self.parse_type()?;
                            self.consume(&TokenKind::RightBracket, "Expected ']' after array type")?;
                            Ok(Type::Array(Box::new(inner)))
                        } else {
                            Ok(Type::Array(Box::new(Type::Int))) // Default to int array
                        }
                    }
                    "map" => {
                        if self.check(&TokenKind::LeftBracket) {
                            self.advance();
                            let inner = self.parse_type()?;
                            self.consume(&TokenKind::RightBracket, "Expected ']' after map type")?;
                            Ok(Type::Map(Box::new(inner)))
                        } else {
                            Ok(Type::Map(Box::new(Type::String))) // Default to string map
                        }
                    }
                    _ => Ok(Type::Identifier(name.clone())),
                }
            }
            _ => Err(self.error("Expected type name")),
        }
    }

    // Helper methods
    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.current)
    }

    fn check(&self, kind: &TokenKind) -> bool {
        if self.is_at_end() {
            false
        } else {
            std::mem::discriminant(&self.tokens[self.current].kind) == std::mem::discriminant(kind)
        }
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len() || matches!(self.tokens[self.current].kind, TokenKind::Eof)
    }

    fn consume(&mut self, kind: &TokenKind, message: &str) -> Result<&Token, ParseError> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err(self.error(message))
        }
    }

    fn error(&self, message: &str) -> ParseError {
        let span = if self.is_at_end() {
            self.previous().span.clone()
        } else {
            self.tokens[self.current].span.clone()
        };
        ParseError {
            message: message.to_string(),
            span,
        }
    }
}

