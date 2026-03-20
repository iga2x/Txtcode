use crate::lexer::keywords::{canonicalize_keyword, is_reserved, is_type_keyword};
use crate::lexer::token::Token;
use crate::parser::ast::*;
use crate::parser::utils::token_span_to_ast_span;

/// Maximum nesting depth for recursive expression/statement parsing.
/// Prevents stack overflow from deeply nested source code.
const MAX_PARSE_DEPTH: usize = 256;

/// Parser for Txt-code AST generation
pub struct Parser {
    pub(crate) tokens: Vec<Token>,
    pub(crate) position: usize,
    /// Recursion depth counter — guards against stack overflow on pathological inputs.
    pub(crate) parse_depth: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
            parse_depth: 0,
        }
    }

    /// Centralized error handler that provides clear, contextual error messages
    /// with line and column information
    pub(crate) fn error<T>(&self, msg: &str) -> Result<T, String> {
        let token = self.peek();
        let line = token.span.0;
        let column = token.span.1;
        Err(format!(
            "Parse error at line {}, column {}: {} (found {:?})",
            line, column, msg, token.kind
        ))
    }

    /// Error handler with custom context (e.g., "for function parameter")
    pub(crate) fn error_with_context<T>(&self, msg: &str, context: &str) -> Result<T, String> {
        let token = self.peek();
        let line = token.span.0;
        let column = token.span.1;
        Err(format!(
            "Parse error at line {}, column {}: {} {} (found {:?} '{}')",
            line, column, msg, context, token.kind, token.value
        ))
    }

    pub fn parse(&mut self) -> Result<Program, String> {
        let mut statements = Vec::new();

        while !self.is_at_end() {
            if let Some(stmt) = self.parse_statement()? {
                statements.push(stmt);
            }
        }

        Ok(Program { statements })
    }

    /// Parse the full program, collecting ALL syntax errors instead of stopping
    /// at the first one. Returns `(partial_program, errors)`.
    ///
    /// On a syntax error, the parser advances to the next statement-level
    /// synchronization point and continues parsing, so multiple errors per
    /// file are reported in one pass.
    ///
    /// Use this in the REPL, `txtcode check`, and future LSP diagnostics.
    pub fn parse_with_errors(&mut self) -> (Program, Vec<String>) {
        let mut statements = Vec::new();
        let mut errors = Vec::new();

        while !self.is_at_end() {
            match self.parse_statement() {
                Ok(Some(stmt)) => statements.push(stmt),
                Ok(None) => {
                    // Block terminator at top level — advance to avoid infinite loop
                    self.advance();
                }
                Err(e) => {
                    errors.push(e);
                    // Synchronize: skip tokens until we reach a likely
                    // statement boundary (newline + keyword, or EOF).
                    self.synchronize();
                }
            }
        }

        (Program { statements }, errors)
    }

    /// Advance past tokens until we find a synchronization point:
    /// a newline followed by a statement-starting keyword, or EOF.
    fn synchronize(&mut self) {
        while !self.is_at_end() {
            // Consume the current token
            self.advance();

            // Check if the NEXT token starts a new statement
            if self.is_at_end() {
                break;
            }

            let kind = self.peek().kind;
            let value = self.peek().value.clone();

            // Newline is a statement boundary
            if kind == crate::lexer::token::TokenKind::Newline {
                self.advance(); // consume the newline
                // If the token after the newline starts a statement, stop
                if !self.is_at_end() {
                    let next_val = self.peek().value.clone();
                    let next_kind = self.peek().kind;
                    if next_kind == crate::lexer::token::TokenKind::Keyword {
                        let canonical = canonicalize_keyword(&next_val);
                        match canonical.as_str() {
                            "store" | "define" | "if" | "while" | "for" | "return"
                            | "try" | "import" | "export" | "struct" | "enum" | "impl"
                            | "match" | "const" | "break" | "continue" | "repeat"
                            | "async" | "permission" | "type" | "error" => break,
                            "end" | "catch" | "finally" | "else" | "elseif" => {
                                // Block terminators — stop so caller can handle
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                continue;
            }

            // Keyword that starts a new statement is a hard boundary
            if kind == crate::lexer::token::TokenKind::Keyword {
                let canonical = canonicalize_keyword(&value);
                match canonical.as_str() {
                    "store" | "define" | "if" | "while" | "for" | "return"
                    | "try" | "import" | "export" | "struct" | "enum" | "impl"
                    | "match" | "const" | "break" | "continue" | "repeat"
                    | "async" | "permission" | "end" => break,
                    _ => {}
                }
            }
        }
    }

    pub(crate) fn parse_statement(&mut self) -> Result<Option<Statement>, String> {
        if self.is_at_end() {
            return Ok(None);
        }

        // Skip newlines and whitespace
        while !self.is_at_end()
            && (self.peek().kind == crate::lexer::token::TokenKind::Newline
                || self.peek().kind == crate::lexer::token::TokenKind::Whitespace)
        {
            self.advance();
        }

        if self.is_at_end() {
            return Ok(None);
        }

        let token_kind = self.peek().kind;
        let token_value = self.peek().value.clone();
        match token_kind {
            crate::lexer::token::TokenKind::Keyword => {
                // Canonicalize keyword aliases to their main form
                let canonical = canonicalize_keyword(&token_value);
                match canonical.as_str() {
                    "end" | "else" | "elseif" | "catch" | "finally" => {
                        // These are block terminators, not statements
                        // Return None so the caller can handle them
                        Ok(None)
                    }
                    "store" => crate::parser::statements::assignment::parse_store(self),
                    "print" => crate::parser::statements::assignment::parse_print(self),
                    "define" | "async" => crate::parser::statements::functions::parse_define(self),
                    "return" => crate::parser::statements::functions::parse_return(self),
                    "if" => crate::parser::statements::control::parse_if(self),
                    "while" => crate::parser::statements::control::parse_while(self),
                    "do" => crate::parser::statements::control::parse_do_while(self),
                    "for" | "foreach" => {
                        // Consume the keyword (for or foreach, both canonicalize to "for")
                        self.advance();
                        crate::parser::statements::control::parse_for(self)
                    }
                    "repeat" => crate::parser::statements::control::parse_repeat(self),
                    "break" => {
                        let token = self.peek();
                        let span = token_span_to_ast_span(token);
                        self.advance();
                        Ok(Some(Statement::Break { span }))
                    }
                    "continue" => {
                        let token = self.peek();
                        let span = token_span_to_ast_span(token);
                        self.advance();
                        Ok(Some(Statement::Continue { span }))
                    }
                    "const" => crate::parser::statements::assignment::parse_const(self),
                    "enum" => crate::parser::statements::types::parse_enum(self),
                    "struct" => crate::parser::statements::types::parse_struct(self),
                    "impl" => crate::parser::statements::types::parse_impl(self),
                    "match" | "switch" => {
                        // Consume the keyword (match or switch, both canonicalize to "match")
                        self.advance();
                        crate::parser::statements::control::parse_match(self)
                    }
                    "try" => crate::parser::statements::control::parse_try(self),
                    "import" => crate::parser::statements::modules::parse_import(self),
                    "export" => crate::parser::statements::modules::parse_export(self),
                    "permission" => crate::parser::statements::permissions::parse_permission(self),
                    _ => {
                        self.error_with_context("Unexpected keyword", &format!("'{}'", token_value))
                    }
                }
            }
            _ => {
                // Try parsing as expression statement
                // But first check if we're at a block terminator (shouldn't happen, but be safe)
                if self.check_keyword("catch")
                    || self.check_keyword("finally")
                    || self.check_keyword("end")
                {
                    return Ok(None);
                }

                // Check for compound assignment: x += expr, x -= expr, etc.
                if token_kind == crate::lexer::token::TokenKind::Identifier {
                    let next_pos = self.position + 1;
                    if next_pos < self.tokens.len() {
                        let next_kind = &self.tokens[next_pos].kind;
                        let compound_op = match next_kind {
                            crate::lexer::token::TokenKind::PlusAssign => Some(BinaryOperator::Add),
                            crate::lexer::token::TokenKind::MinusAssign => {
                                Some(BinaryOperator::Subtract)
                            }
                            crate::lexer::token::TokenKind::StarAssign => {
                                Some(BinaryOperator::Multiply)
                            }
                            crate::lexer::token::TokenKind::SlashAssign => {
                                Some(BinaryOperator::Divide)
                            }
                            crate::lexer::token::TokenKind::PercentAssign => {
                                Some(BinaryOperator::Modulo)
                            }
                            crate::lexer::token::TokenKind::PowerAssign => {
                                Some(BinaryOperator::Power)
                            }
                            crate::lexer::token::TokenKind::BitAndAssign => {
                                Some(BinaryOperator::BitwiseAnd)
                            }
                            crate::lexer::token::TokenKind::BitOrAssign => {
                                Some(BinaryOperator::BitwiseOr)
                            }
                            crate::lexer::token::TokenKind::BitXorAssign => {
                                Some(BinaryOperator::BitwiseXor)
                            }
                            _ => None,
                        };
                        if let Some(op) = compound_op {
                            let start_token = self.peek().clone();
                            let name = self.expect_identifier()?;
                            self.advance(); // consume compound-assign token
                            let value = self.parse_expression()?;
                            let span = token_span_to_ast_span(&start_token);
                            return Ok(Some(Statement::CompoundAssignment {
                                name,
                                op,
                                value,
                                span,
                            }));
                        }

                        // Check for type alias: type → name → target
                        if token_value == "type"
                            && *next_kind == crate::lexer::token::TokenKind::Arrow
                        {
                            let start_token = self.peek().clone();
                            self.advance(); // consume "type"
                            self.advance(); // consume →
                            let alias_name = self.expect_identifier()?;
                            self.skip_optional_arrow();
                            // Parse target as identifier or type keyword
                            let target = if self.check(crate::lexer::token::TokenKind::Identifier)
                                || self.check(crate::lexer::token::TokenKind::Keyword)
                            {
                                let t = self.peek().value.clone();
                                self.advance();
                                t
                            } else {
                                return self.error("Expected type name after type alias arrow");
                            };
                            let span = token_span_to_ast_span(&start_token);
                            return Ok(Some(Statement::TypeAlias {
                                name: alias_name,
                                target,
                                span,
                            }));
                        }

                        // Check for named error: error → name → message
                        if token_value == "error"
                            && *next_kind == crate::lexer::token::TokenKind::Arrow
                        {
                            let start_token = self.peek().clone();
                            self.advance(); // consume "error"
                            self.advance(); // consume →
                            let error_name = self.expect_identifier()?;
                            self.skip_optional_arrow();
                            let message = self.parse_expression()?;
                            let span = token_span_to_ast_span(&start_token);
                            return Ok(Some(Statement::NamedError {
                                name: error_name,
                                message,
                                span,
                            }));
                        }
                    }
                }

                if let Ok(expr) = self.parse_expression() {
                    Ok(Some(Statement::Expression(expr)))
                } else {
                    self.error(&format!("Unexpected token: {:?}", token_kind))
                }
            }
        }
    }

    pub(crate) fn parse_expression(&mut self) -> Result<Expression, String> {
        self.parse_depth += 1;
        if self.parse_depth > MAX_PARSE_DEPTH {
            self.parse_depth -= 1;
            return Err(format!(
                "Parse error: expression nesting depth exceeds maximum ({}) — \
                 simplify deeply nested expressions",
                MAX_PARSE_DEPTH
            ));
        }
        let result = crate::parser::expressions::operators::parse_expression(self);
        self.parse_depth -= 1;
        result
    }

    pub(crate) fn parse_pattern(&mut self) -> Result<Pattern, String> {
        crate::parser::patterns::parse_pattern(self)
    }

    pub(crate) fn peek(&self) -> &Token {
        if self.position < self.tokens.len() {
            &self.tokens[self.position]
        } else {
            &self.tokens[self.tokens.len() - 1]
        }
    }

    pub(crate) fn advance(&mut self) {
        if !self.is_at_end() {
            self.position += 1;
        }
    }

    pub(crate) fn is_at_end(&self) -> bool {
        self.position >= self.tokens.len()
            || self.tokens[self.position].kind == crate::lexer::token::TokenKind::Eof
    }

    pub(crate) fn check(&self, kind: crate::lexer::token::TokenKind) -> bool {
        !self.is_at_end() && self.peek().kind == kind
    }

    pub(crate) fn check_keyword(&self, keyword: &str) -> bool {
        if self.is_at_end() {
            return false;
        }
        if self.peek().kind == crate::lexer::token::TokenKind::Keyword {
            let token_value = &self.peek().value;
            // Check both the token value and its canonicalized form
            token_value == keyword || canonicalize_keyword(token_value) == keyword
        } else {
            false
        }
    }

    pub(crate) fn expect(&mut self, kind: crate::lexer::token::TokenKind) -> Result<(), String> {
        if self.check(kind) {
            self.advance();
            Ok(())
        } else {
            let token = self.peek();
            Err(format!(
                "Parse error at line {}, column {}: Expected {:?}, got {:?} '{}'",
                token.span.0, token.span.1, kind, token.kind, token.value
            ))
        }
    }

    pub(crate) fn expect_keyword(&mut self, keyword: &str) -> Result<(), String> {
        if self.check_keyword(keyword) {
            self.advance();
            Ok(())
        } else {
            let token = self.peek();
            Err(format!(
                "Parse error at line {}, column {}: Expected keyword '{}', got {:?} '{}'",
                token.span.0, token.span.1, keyword, token.kind, token.value
            ))
        }
    }

    pub(crate) fn expect_identifier(&mut self) -> Result<String, String> {
        self.expect_identifier_allow_reserved(false)
    }

    /// Expect an identifier, optionally allowing reserved words (for type annotations)
    pub(crate) fn expect_identifier_allow_reserved(
        &mut self,
        allow_reserved: bool,
    ) -> Result<String, String> {
        if self.check(crate::lexer::token::TokenKind::Identifier) {
            let name = self.peek().value.clone();

            // Check if it's a reserved word (keyword or type keyword)
            if !allow_reserved && is_reserved(&name) {
                let token = self.peek();
                return Err(format!(
                    "Parse error at line {}, column {}: '{}' is a reserved word and cannot be used as an identifier",
                    token.span.0, token.span.1, name
                ));
            }

            self.advance();
            Ok(name)
        } else if self.check(crate::lexer::token::TokenKind::Keyword) && allow_reserved {
            // Allow keywords when expecting type names (for type annotations)
            let name = self.peek().value.clone();
            if is_type_keyword(&name) {
                self.advance();
                Ok(name)
            } else {
                let token = self.peek();
                let found = format!("{:?} '{}'", token.kind, token.value);
                Err(format!(
                    "Parse error at line {}, column {}: Expected type name, found {}",
                    token.span.0, token.span.1, found
                ))
            }
        } else {
            let token = self.peek();
            let found = if self.is_at_end() {
                "end of input".to_string()
            } else {
                format!("{:?} '{}'", token.kind, token.value)
            };
            Err(format!(
                "Parse error at line {}, column {}: Expected identifier, found {}",
                token.span.0, token.span.1, found
            ))
        }
    }

    /// Convert a type keyword string to Type enum
    /// Uses is_type_keyword() to check for built-in types, otherwise returns Identifier type
    pub(crate) fn type_keyword_to_type(&self, type_name: &str) -> crate::typecheck::types::Type {
        if is_type_keyword(type_name) {
            match type_name {
                "int" => crate::typecheck::types::Type::Int,
                "float" => crate::typecheck::types::Type::Float,
                "string" => crate::typecheck::types::Type::String,
                "char" => crate::typecheck::types::Type::Char,
                "bool" => crate::typecheck::types::Type::Bool,
                // Future: handle "array" and "map" types here if needed
                _ => crate::typecheck::types::Type::Identifier(type_name.to_string()),
            }
        } else {
            crate::typecheck::types::Type::Identifier(type_name.to_string())
        }
    }

    /// Parse a type annotation, supporting:
    /// - Simple types: int, float, string, bool, char
    /// - Array types: array[T]
    /// - Map types: map[T]
    /// - Set types: set[T]
    /// - Generic types: T (when in type_params)
    pub(crate) fn parse_type(
        &mut self,
        type_params: &[String],
    ) -> Result<crate::typecheck::types::Type, String> {
        // Skip whitespace
        while !self.is_at_end()
            && (self.peek().kind == crate::lexer::token::TokenKind::Whitespace
                || self.peek().kind == crate::lexer::token::TokenKind::Newline)
        {
            self.advance();
        }

        // Check for array, map, or set types
        // These can be either Keyword or Identifier tokens, so check the value
        let is_array = self.check_keyword("array")
            || (self.check(crate::lexer::token::TokenKind::Identifier)
                && self.peek().value == "array");
        let is_map = self.check_keyword("map")
            || (self.check(crate::lexer::token::TokenKind::Identifier)
                && self.peek().value == "map");
        let is_set = self.check_keyword("set")
            || (self.check(crate::lexer::token::TokenKind::Identifier)
                && self.peek().value == "set");

        if is_array {
            self.advance();
            // Inner type is optional: array or array[int]
            if self.check(crate::lexer::token::TokenKind::LeftBracket) {
                self.advance();
                let inner_type = self.parse_type(type_params)?;
                self.expect(crate::lexer::token::TokenKind::RightBracket)?;
                return Ok(crate::typecheck::types::Type::Array(Box::new(inner_type)));
            }
            return Ok(crate::typecheck::types::Type::Array(Box::new(
                crate::typecheck::types::Type::Identifier("any".to_string()),
            )));
        }

        if is_map {
            self.advance();
            // Inner type is optional: map or map[string]
            if self.check(crate::lexer::token::TokenKind::LeftBracket) {
                self.advance();
                let inner_type = self.parse_type(type_params)?;
                self.expect(crate::lexer::token::TokenKind::RightBracket)?;
                return Ok(crate::typecheck::types::Type::Map(Box::new(inner_type)));
            }
            return Ok(crate::typecheck::types::Type::Map(Box::new(
                crate::typecheck::types::Type::Identifier("any".to_string()),
            )));
        }

        if is_set {
            self.advance();
            // Inner type is optional: set or set[int]
            if self.check(crate::lexer::token::TokenKind::LeftBracket) {
                self.advance();
                let inner_type = self.parse_type(type_params)?;
                self.expect(crate::lexer::token::TokenKind::RightBracket)?;
                return Ok(crate::typecheck::types::Type::Set(Box::new(inner_type)));
            }
            return Ok(crate::typecheck::types::Type::Set(Box::new(
                crate::typecheck::types::Type::Identifier("any".to_string()),
            )));
        }

        // Parse simple type or generic type parameter
        let type_name = self.expect_identifier_allow_reserved(true).map_err(|_| {
            let token = self.peek();
            format!(
                "Parse error at line {}, column {}: Expected type name (found {:?} '{}')",
                token.span.0, token.span.1, token.kind, token.value
            )
        })?;

        // Check if it's a generic type parameter
        let base_type = if type_params.contains(&type_name) {
            crate::typecheck::types::Type::Generic(type_name)
        } else {
            self.type_keyword_to_type(&type_name)
        };

        // Optional `?` suffix makes the type nullable: int? → Nullable(Int)
        if self.check(crate::lexer::token::TokenKind::QuestionMark) {
            self.advance();
            Ok(crate::typecheck::types::Type::Nullable(Box::new(base_type)))
        } else {
            Ok(base_type)
        }
    }

    pub(crate) fn expect_arrow(&mut self) -> Result<(), String> {
        self.expect(crate::lexer::token::TokenKind::Arrow)
    }

    /// Skip optional arrow and whitespace after it
    /// Used for hybrid syntax support (arrow-based and space-based)
    pub(crate) fn skip_optional_arrow(&mut self) {
        if self.check(crate::lexer::token::TokenKind::Arrow) {
            self.advance();
            // Skip whitespace after arrow
            while !self.is_at_end()
                && (self.peek().kind == crate::lexer::token::TokenKind::Whitespace
                    || self.peek().kind == crate::lexer::token::TokenKind::Newline)
            {
                self.advance();
            }
        }
    }
}
