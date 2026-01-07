use super::token::{Token, TokenKind, Span};
use super::keywords::get_keyword;

#[derive(Debug)]
pub struct Lexer {
    input: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
}

#[derive(Debug, Clone)]
pub struct LexError {
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at line {}, column {}", self.message, self.span.line, self.span.column)
    }
}

impl std::error::Error for LexError {}

impl Lexer {
    pub fn new(input: String) -> Self {
        Self {
            input: input.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();

        while let Some(token) = self.next_token()? {
            match token.kind {
                TokenKind::Whitespace | TokenKind::Comment(_) | TokenKind::MultiLineComment(_) => {
                    // Skip whitespace and comments
                }
                TokenKind::Eof => {
                    tokens.push(token);
                    break;
                }
                _ => tokens.push(token),
            }
        }

        Ok(tokens)
    }

    fn next_token(&mut self) -> Result<Option<Token>, LexError> {
        if self.position >= self.input.len() {
            return Ok(Some(Token::new(
                TokenKind::Eof,
                self.current_span(0),
            )));
        }

        let ch = self.input[self.position];

        let start_pos = self.position;
        let start_line = self.line;
        let start_col = self.column;

        let kind = match ch {
            // Single character operators
            '+' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::PlusEqual
                } else {
                    TokenKind::Plus
                }
            },
            '-' => {
                // Check for arrow operator (->)
                if self.peek() == Some('>') {
                    self.advance();
                    TokenKind::Arrow
                } else if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::MinusEqual
                } else {
                    TokenKind::Minus
                }
            },
            '*' => {
                if self.peek() == Some('*') {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::PowerEqual
                    } else {
                        TokenKind::Power
                    }
                } else if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::StarEqual
                } else {
                    TokenKind::Star
                }
            }
            '/' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::SlashEqual
                } else {
                    TokenKind::Slash
                }
            },
            '%' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::PercentEqual
                } else {
                    TokenKind::Percent
                }
            },
            '=' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::Equal
                } else {
                    TokenKind::Assign
                }
            }
            '!' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::NotEqual
                } else {
                    TokenKind::Exclamation
                }
            }
            '<' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::LessEqual
                } else if self.peek() == Some('<') {
                    self.advance();
                    TokenKind::LeftShift
                } else {
                    TokenKind::Less
                }
            }
            '>' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::GreaterEqual
                } else if self.peek() == Some('>') {
                    self.advance();
                    TokenKind::RightShift
                } else {
                    TokenKind::Greater
                }
            }
            '&' => TokenKind::BitAnd,
            '|' => TokenKind::BitOr,
            '^' => TokenKind::BitXor,
            '~' => TokenKind::BitNot,

            // Punctuation
            '(' => TokenKind::LeftParen,
            ')' => TokenKind::RightParen,
            '[' => TokenKind::LeftBracket,
            ']' => TokenKind::RightBracket,
            '{' => TokenKind::LeftBrace,
            '}' => TokenKind::RightBrace,
            ',' => TokenKind::Comma,
            ':' => TokenKind::Colon,
            ';' => TokenKind::Semicolon,
            '.' => TokenKind::Dot,
            '?' => TokenKind::Question,

            // Comments
            '#' => {
                if self.peek() == Some('#') {
                    self.advance();
                    self.read_multiline_comment()?
                } else {
                    self.read_single_comment()?
                }
            }

            // Strings
            '"' | '\'' => self.read_string(ch)?,

            // Numbers
            '0'..='9' => {
                // Don't advance yet, read_number will handle it
                self.read_number()?
            }

            // Identifiers and keywords
            'a'..='z' | 'A'..='Z' | '_' => {
                // Don't advance yet, read_identifier will handle it
                self.read_identifier()?
            }

            // Whitespace
            ' ' | '\t' => {
                self.skip_whitespace();
                return self.next_token();
            }

            // Newlines (with line continuation support)
            '\n' => {
                // Check if previous non-whitespace character was a backslash (line continuation)
                let mut check_pos = self.position;
                while check_pos > 0 {
                    check_pos -= 1;
                    let prev_ch = self.input[check_pos];
                    if prev_ch == '\\' {
                        // Line continuation - skip the newline and continue
                        self.line += 1;
                        self.column = 1;
                        self.advance(); // consume newline
                        // Skip whitespace on the next line
                        self.skip_whitespace();
                        return self.next_token();
                    } else if prev_ch != ' ' && prev_ch != '\t' {
                        // Not a line continuation
                        break;
                    }
                }
                self.line += 1;
                self.column = 1;
                TokenKind::Newline
            }
            '\r' => {
                // Check if previous non-whitespace character was a backslash (line continuation)
                let mut check_pos = self.position;
                while check_pos > 0 {
                    check_pos -= 1;
                    let prev_ch = self.input[check_pos];
                    if prev_ch == '\\' {
                        // Line continuation - skip the newline and continue
                        if self.peek() == Some('\n') {
                            self.advance();
                        }
                        self.line += 1;
                        self.column = 1;
                        self.advance(); // consume \r
                        // Skip whitespace on the next line
                        self.skip_whitespace();
                        return self.next_token();
                    } else if prev_ch != ' ' && prev_ch != '\t' {
                        // Not a line continuation
                        break;
                    }
                }
                if self.peek() == Some('\n') {
                    self.advance();
                }
                self.line += 1;
                self.column = 1;
                TokenKind::Newline
            }

            _ => {
                return Err(LexError {
                    message: format!("Unexpected character: {}", ch),
                    span: self.current_span(1),
                });
            }
        };

        // Only advance if we haven't already (numbers, identifiers, and strings advance themselves)
        if !matches!(kind, TokenKind::Identifier(_) | TokenKind::Integer(_) | TokenKind::Float(_) | TokenKind::String(_)) {
            self.advance();
        }
        let span = Span::new(start_pos, self.position, start_line, start_col);

        Ok(Some(Token::new(kind, span)))
    }

    fn read_string(&mut self, quote: char) -> Result<TokenKind, LexError> {
        let mut value = String::new();
        let mut escaped = false;

        self.advance(); // Skip opening quote

        loop {
            if self.position >= self.input.len() {
                return Err(LexError {
                    message: "Unterminated string literal".to_string(),
                    span: self.current_span(1),
                });
            }

            let ch = self.input[self.position];
            self.advance();

            if escaped {
                match ch {
                    'n' => value.push('\n'),
                    't' => value.push('\t'),
                    'r' => value.push('\r'),
                    '\\' => value.push('\\'),
                    '"' => value.push('"'),
                    '\'' => value.push('\''),
                    _ => {
                        return Err(LexError {
                            message: format!("Invalid escape sequence: \\{}", ch),
                            span: self.current_span(1),
                        });
                    }
                }
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == quote {
                break;
            } else {
                value.push(ch);
            }
        }

        Ok(TokenKind::String(value))
    }

    fn read_number(&mut self) -> Result<TokenKind, LexError> {
        let mut value = String::new();
        let mut is_float = false;
        let mut is_hex = false;
        let mut is_binary = false;

        // Add the first digit (current position)
        if self.position < self.input.len() {
            value.push(self.input[self.position]);
            self.advance();
        }

        // Check for hex (0x) or binary (0b) prefix
        if value == "0" && self.position < self.input.len() {
            match self.input[self.position] {
                'x' | 'X' => {
                    value.push(self.input[self.position]);
                    self.advance();
                    is_hex = true;
                }
                'b' | 'B' => {
                    value.push(self.input[self.position]);
                    self.advance();
                    is_binary = true;
                }
                _ => {}
            }
        }

        // Read digits
        while self.position < self.input.len() {
            let ch = self.input[self.position];
            match ch {
                '0'..='9' | 'a'..='f' | 'A'..='F' if is_hex => {
                    value.push(ch);
                    self.advance();
                }
                '0'..='1' if is_binary => {
                    value.push(ch);
                    self.advance();
                }
                '0'..='9' if !is_hex && !is_binary => {
                    value.push(ch);
                    self.advance();
                }
                '.' if !is_float && !is_hex && !is_binary => {
                    value.push(ch);
                    self.advance();
                    is_float = true;
                }
                'e' | 'E' if !is_hex && !is_binary => {
                    value.push(ch);
                    self.advance();
                    if self.position < self.input.len() {
                        let next = self.input[self.position];
                        if next == '+' || next == '-' {
                            value.push(next);
                            self.advance();
                        }
                    }
                    is_float = true;
                }
                _ => break,
            }
        }

        if is_hex {
            let num = i64::from_str_radix(&value[2..], 16)
                .map_err(|_| LexError {
                    message: format!("Invalid hex number: {}", value),
                    span: self.current_span(value.len()),
                })?;
            Ok(TokenKind::Integer(num))
        } else if is_binary {
            let num = i64::from_str_radix(&value[2..], 2)
                .map_err(|_| LexError {
                    message: format!("Invalid binary number: {}", value),
                    span: self.current_span(value.len()),
                })?;
            Ok(TokenKind::Integer(num))
        } else if is_float {
            let num = value.parse::<f64>()
                .map_err(|_| LexError {
                    message: format!("Invalid float number: {}", value),
                    span: self.current_span(value.len()),
                })?;
            Ok(TokenKind::Float(num))
        } else {
            let num = value.parse::<i64>()
                .map_err(|_| LexError {
                    message: format!("Invalid integer: {}", value),
                    span: self.current_span(value.len()),
                })?;
            Ok(TokenKind::Integer(num))
        }
    }

    fn read_identifier(&mut self) -> Result<TokenKind, LexError> {
        let mut value = String::new();

        // Add the first character (current position)
        if self.position < self.input.len() {
            value.push(self.input[self.position]);
            self.advance();
        }

        while self.position < self.input.len() {
            let ch = self.input[self.position];
            if ch.is_alphanumeric() || ch == '_' {
                value.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        if let Some(keyword) = get_keyword(&value) {
            Ok(keyword)
        } else {
            Ok(TokenKind::Identifier(value))
        }
    }

    fn read_single_comment(&mut self) -> Result<TokenKind, LexError> {
        let mut value = String::new();

        // Skip the '#' character
        if self.position < self.input.len() {
            self.advance();
        }

        while self.position < self.input.len() {
            let ch = self.input[self.position];
            if ch == '\n' || ch == '\r' {
                break;
            }
            value.push(ch);
            self.advance();
        }

        Ok(TokenKind::Comment(value))
    }

    fn read_multiline_comment(&mut self) -> Result<TokenKind, LexError> {
        let mut value = String::new();
        let mut last_hash = false;

        loop {
            if self.position >= self.input.len() {
                return Err(LexError {
                    message: "Unterminated multi-line comment".to_string(),
                    span: self.current_span(1),
                });
            }

            let ch = self.input[self.position];
            self.advance();

            if last_hash && ch == '#' {
                value.pop(); // Remove the last '#'
                break;
            }
            last_hash = ch == '#';
            value.push(ch);
        }

        Ok(TokenKind::MultiLineComment(value))
    }

    fn skip_whitespace(&mut self) {
        while self.position < self.input.len() {
            let ch = self.input[self.position];
            if ch == ' ' || ch == '\t' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn peek(&self) -> Option<char> {
        if self.position + 1 < self.input.len() {
            Some(self.input[self.position + 1])
        } else {
            None
        }
    }

    fn advance(&mut self) {
        if self.position < self.input.len() {
            self.position += 1;
            self.column += 1;
        }
    }

    fn current_span(&self, length: usize) -> Span {
        Span::new(
            self.position,
            self.position + length,
            self.line,
            self.column,
        )
    }
}

