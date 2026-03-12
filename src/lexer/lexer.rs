use crate::lexer::keywords::is_keyword;
use crate::lexer::token::{Token, TokenKind};

/// Lexer for tokenizing Txt-code source code
pub struct Lexer {
    source: String,
    position: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    pub fn new(source: String) -> Self {
        Self {
            source,
            position: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();

        while self.position < self.source.len() {
            if let Some(token) = self.next_token()? {
                if token.kind != TokenKind::Whitespace && token.kind != TokenKind::Comment {
                    tokens.push(token);
                }
            } else {
                break;
            }
        }

        tokens.push(Token::new(
            TokenKind::Eof,
            String::new(),
            (self.line, self.column),
        ));
        Ok(tokens)
    }

    fn next_token(&mut self) -> Result<Option<Token>, String> {
        if self.position >= self.source.len() {
            return Ok(None);
        }

        let _start_pos = self.position;
        let start_line = self.line;
        let start_col = self.column;

        // Get character at current byte position
        // Since we're using byte positions, we need to get the char that starts at this byte
        let ch = if self.position < self.source.len() {
            // Get the character starting at this byte position
            let remaining = &self.source[self.position..];
            remaining.chars().next()
        } else {
            None
        };

        let ch = match ch {
            Some(c) => c,
            None => return Ok(None),
        };

        let token = match ch {
            // Whitespace
            ' ' | '\t' => {
                self.skip_whitespace();
                return self.next_token();
            }
            '\n' | '\r' => {
                self.advance();
                Token::new(
                    TokenKind::Newline,
                    String::from(ch),
                    (start_line, start_col),
                )
            }
            // Comments
            '#' => {
                self.skip_comment();
                return self.next_token();
            }
            // Operators
            '+' => {
                // Check for increment operator (++) or compound assign (+=)
                if self.peek() == Some('+') {
                    self.advance(); // Advance past first '+'
                    self.advance(); // Advance past second '+'
                    Token::new(
                        TokenKind::Increment,
                        "++".to_string(),
                        (start_line, start_col),
                    )
                } else if self.peek() == Some('=') {
                    self.advance(); // Advance past '+'
                    self.advance(); // Advance past '='
                    Token::new(
                        TokenKind::PlusAssign,
                        "+=".to_string(),
                        (start_line, start_col),
                    )
                } else {
                    self.advance(); // Advance past '+'
                    Token::new(TokenKind::Plus, String::from(ch), (start_line, start_col))
                }
            }
            '-' => {
                // Check for arrow operator (->) first
                if self.peek() == Some('>') {
                    self.advance(); // Advance past '-'
                    self.advance(); // Advance past '>'
                    Token::new(TokenKind::Arrow, "->".to_string(), (start_line, start_col))
                } else if self.peek() == Some('-') {
                    // Check for decrement operator (--)
                    self.advance(); // Advance past first '-'
                    self.advance(); // Advance past second '-'
                    Token::new(
                        TokenKind::Decrement,
                        "--".to_string(),
                        (start_line, start_col),
                    )
                } else if self.peek() == Some('=') {
                    self.advance(); // Advance past '-'
                    self.advance(); // Advance past '='
                    Token::new(
                        TokenKind::MinusAssign,
                        "-=".to_string(),
                        (start_line, start_col),
                    )
                } else {
                    self.advance(); // Advance past '-'
                    Token::new(TokenKind::Minus, String::from(ch), (start_line, start_col))
                }
            }
            '→' => {
                // Unicode arrow operator (→) - equivalent to ->
                // advance() already handles Unicode correctly
                self.advance();
                Token::new(TokenKind::Arrow, "→".to_string(), (start_line, start_col))
            }
            '*' => {
                // Check next character before advancing: **, **=, *=, *
                if self.peek() == Some('*') {
                    self.advance(); // Advance past first '*'
                    self.advance(); // Advance past second '*'
                                    // Check for **=
                    if self.position < self.source.len() {
                        let rem = &self.source[self.position..];
                        if rem.starts_with('=') {
                            self.advance(); // Advance past '='
                            Token::new(
                                TokenKind::PowerAssign,
                                "**=".to_string(),
                                (start_line, start_col),
                            )
                        } else {
                            Token::new(TokenKind::Power, "**".to_string(), (start_line, start_col))
                        }
                    } else {
                        Token::new(TokenKind::Power, "**".to_string(), (start_line, start_col))
                    }
                } else if self.peek() == Some('=') {
                    self.advance(); // Advance past '*'
                    self.advance(); // Advance past '='
                    Token::new(
                        TokenKind::StarAssign,
                        "*=".to_string(),
                        (start_line, start_col),
                    )
                } else {
                    self.advance(); // Advance past '*'
                    Token::new(TokenKind::Star, String::from(ch), (start_line, start_col))
                }
            }
            '/' => {
                if self.peek() == Some('=') {
                    self.advance(); // Advance past '/'
                    self.advance(); // Advance past '='
                    Token::new(
                        TokenKind::SlashAssign,
                        "/=".to_string(),
                        (start_line, start_col),
                    )
                } else {
                    self.advance();
                    Token::new(TokenKind::Slash, String::from(ch), (start_line, start_col))
                }
            }
            '%' => {
                if self.peek() == Some('=') {
                    self.advance(); // Advance past '%'
                    self.advance(); // Advance past '='
                    Token::new(
                        TokenKind::PercentAssign,
                        "%=".to_string(),
                        (start_line, start_col),
                    )
                } else {
                    self.advance();
                    Token::new(
                        TokenKind::Percent,
                        String::from(ch),
                        (start_line, start_col),
                    )
                }
            }
            '<' => {
                // Check next character before advancing
                if self.peek() == Some('<') {
                    self.advance(); // Advance past '<'
                    self.advance(); // Advance past second '<'
                    Token::new(
                        TokenKind::LeftShift,
                        "<<".to_string(),
                        (start_line, start_col),
                    )
                } else if self.peek() == Some('=') {
                    self.advance(); // Advance past '<'
                    self.advance(); // Advance past '='
                    Token::new(
                        TokenKind::LessEqual,
                        "<=".to_string(),
                        (start_line, start_col),
                    )
                } else {
                    self.advance(); // Advance past '<'
                    Token::new(TokenKind::Less, String::from(ch), (start_line, start_col))
                }
            }
            '>' => {
                // Check next character before advancing
                if self.peek() == Some('>') {
                    self.advance(); // Advance past '>'
                    self.advance(); // Advance past second '>'
                    Token::new(
                        TokenKind::RightShift,
                        ">>".to_string(),
                        (start_line, start_col),
                    )
                } else if self.peek() == Some('=') {
                    self.advance(); // Advance past '>'
                    self.advance(); // Advance past '='
                    Token::new(
                        TokenKind::GreaterEqual,
                        ">=".to_string(),
                        (start_line, start_col),
                    )
                } else {
                    self.advance(); // Advance past '>'
                    Token::new(
                        TokenKind::Greater,
                        String::from(ch),
                        (start_line, start_col),
                    )
                }
            }
            '=' => {
                // Check next character before advancing
                if self.peek() == Some('=') {
                    self.advance(); // Advance past '='
                    self.advance(); // Advance past second '='
                    Token::new(TokenKind::Equal, "==".to_string(), (start_line, start_col))
                } else {
                    self.advance(); // Advance past '='
                    Token::new(
                        TokenKind::Assignment,
                        String::from(ch),
                        (start_line, start_col),
                    )
                }
            }
            '!' => {
                // Check next character before advancing
                if self.peek() == Some('=') {
                    self.advance(); // Advance past '!'
                    self.advance(); // Advance past '='
                    Token::new(
                        TokenKind::NotEqual,
                        "!=".to_string(),
                        (start_line, start_col),
                    )
                } else {
                    return Err(format!("Unexpected character: {}", ch));
                }
            }
            '?' => {
                // Check for null coalesce (??) or optional chain (?.) or standalone ?
                if self.peek() == Some('?') {
                    self.advance(); // Advance past first '?'
                    self.advance(); // Advance past second '?'
                    Token::new(
                        TokenKind::NullCoalesce,
                        "??".to_string(),
                        (start_line, start_col),
                    )
                } else if self.peek() == Some('.') {
                    self.advance(); // Advance past '?'
                    self.advance(); // Advance past '.'
                    Token::new(
                        TokenKind::OptionalChain,
                        "?.".to_string(),
                        (start_line, start_col),
                    )
                } else {
                    self.advance(); // Advance past '?'
                    Token::new(
                        TokenKind::QuestionMark,
                        "?".to_string(),
                        (start_line, start_col),
                    )
                }
            }
            '&' => {
                self.advance();
                let next = self.source[self.position..].chars().next();
                if next == Some('&') {
                    self.advance();
                    Token::new(TokenKind::And, "&&".to_string(), (start_line, start_col))
                } else if next == Some('=') {
                    self.advance();
                    Token::new(
                        TokenKind::BitAndAssign,
                        "&=".to_string(),
                        (start_line, start_col),
                    )
                } else {
                    Token::new(TokenKind::BitAnd, String::from(ch), (start_line, start_col))
                }
            }
            '|' => {
                self.advance();
                let next = self.source[self.position..].chars().next();
                if next == Some('>') {
                    self.advance();
                    Token::new(TokenKind::Pipe, "|>".to_string(), (start_line, start_col))
                } else if next == Some('|') {
                    self.advance();
                    Token::new(TokenKind::Or, "||".to_string(), (start_line, start_col))
                } else if next == Some('=') {
                    self.advance();
                    Token::new(
                        TokenKind::BitOrAssign,
                        "|=".to_string(),
                        (start_line, start_col),
                    )
                } else {
                    Token::new(TokenKind::BitOr, String::from(ch), (start_line, start_col))
                }
            }
            '^' => {
                self.advance();
                let next = self.source[self.position..].chars().next();
                if next == Some('=') {
                    self.advance();
                    Token::new(
                        TokenKind::BitXorAssign,
                        "^=".to_string(),
                        (start_line, start_col),
                    )
                } else {
                    Token::new(TokenKind::BitXor, String::from(ch), (start_line, start_col))
                }
            }
            '~' => {
                self.advance();
                Token::new(TokenKind::BitNot, String::from(ch), (start_line, start_col))
            }
            // Delimiters
            '(' => {
                self.advance();
                Token::new(
                    TokenKind::LeftParen,
                    String::from(ch),
                    (start_line, start_col),
                )
            }
            ')' => {
                self.advance();
                Token::new(
                    TokenKind::RightParen,
                    String::from(ch),
                    (start_line, start_col),
                )
            }
            '{' => {
                self.advance();
                Token::new(
                    TokenKind::LeftBrace,
                    String::from(ch),
                    (start_line, start_col),
                )
            }
            '}' => {
                self.advance();
                Token::new(
                    TokenKind::RightBrace,
                    String::from(ch),
                    (start_line, start_col),
                )
            }
            '[' => {
                self.advance();
                Token::new(
                    TokenKind::LeftBracket,
                    String::from(ch),
                    (start_line, start_col),
                )
            }
            ']' => {
                self.advance();
                Token::new(
                    TokenKind::RightBracket,
                    String::from(ch),
                    (start_line, start_col),
                )
            }
            ',' => {
                self.advance();
                Token::new(TokenKind::Comma, String::from(ch), (start_line, start_col))
            }
            ';' => {
                self.advance();
                Token::new(
                    TokenKind::Semicolon,
                    String::from(ch),
                    (start_line, start_col),
                )
            }
            ':' => {
                self.advance();
                Token::new(TokenKind::Colon, String::from(ch), (start_line, start_col))
            }
            '.' => {
                self.advance();
                // Check for spread: ...
                if self.source[self.position..].starts_with("..") {
                    self.advance(); // consume second .
                    self.advance(); // consume third .
                    Token::new(
                        TokenKind::Spread,
                        "...".to_string(),
                        (start_line, start_col),
                    )
                } else {
                    Token::new(TokenKind::Dot, String::from(ch), (start_line, start_col))
                }
            }
            // String and char literals
            '"' => {
                // Check for multiline string: """..."""
                if self.source[self.position..].starts_with("\"\"\"") {
                    self.read_multiline_string()?
                } else {
                    self.read_string(ch)?
                }
            }
            '\'' => {
                // Check if it's a char literal (single character) or string literal
                self.read_char_or_string(ch)?
            }
            // Numbers
            '0'..='9' => self.read_number()?,
            // Identifiers and keywords (also raw strings: r"...", f-strings: f"...")
            'a'..='z' | 'A'..='Z' | '_' => {
                // Check for raw string: r"..."
                if ch == 'r' && self.peek() == Some('"') {
                    self.advance(); // consume 'r'
                    self.read_raw_string()?
                } else if ch == 'f' && (self.peek() == Some('"') || self.peek() == Some('\'')) {
                    // f-string prefix: f"..." or f'...' — consume 'f' and read as normal string
                    // The string reader already marks strings with { } as InterpolatedString
                    self.advance(); // consume 'f'
                    let quote = self.source[self.position..].chars().next().unwrap_or('"');
                    self.read_string(quote)?
                } else {
                    self.read_identifier()?
                }
            }
            _ => {
                return Err(format!(
                    "Unexpected character: {} at line {}:{}",
                    ch, self.line, self.column
                ));
            }
        };

        Ok(Some(token))
    }

    fn advance(&mut self) {
        if self.position < self.source.len() {
            // Get the character at current byte position to check for newline
            let remaining = &self.source[self.position..];
            if let Some(ch) = remaining.chars().next() {
                if ch == '\n' {
                    self.line += 1;
                    self.column = 1;
                } else {
                    self.column += 1;
                }
                // Advance by the byte length of this character (handles Unicode)
                self.position += ch.len_utf8();
            } else {
                // Fallback: advance by 1 byte (shouldn't happen)
                self.position += 1;
            }
        }
    }

    fn peek(&self) -> Option<char> {
        if self.position < self.source.len() {
            // Get the character at the next position after current
            // First, get current char to know its byte length
            let remaining = &self.source[self.position..];
            if let Some(current_ch) = remaining.chars().next() {
                // Advance past current character
                let next_pos = self.position + current_ch.len_utf8();
                if next_pos < self.source.len() {
                    let next_remaining = &self.source[next_pos..];
                    next_remaining.chars().next()
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    fn skip_whitespace(&mut self) {
        while self.position < self.source.len() {
            let remaining = &self.source[self.position..];
            if let Some(ch) = remaining.chars().next() {
                if ch == ' ' || ch == '\t' {
                    self.advance();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        while self.position < self.source.len() {
            let remaining = &self.source[self.position..];
            if let Some(ch) = remaining.chars().next() {
                if ch == '\n' || ch == '\r' {
                    break;
                }
                self.advance();
            } else {
                break;
            }
        }
    }

    fn read_char_or_string(&mut self, quote: char) -> Result<Token, String> {
        let start_line = self.line;
        let start_col = self.column;
        self.advance(); // Skip opening quote

        if self.position >= self.source.len() {
            return Err("Unterminated character literal".to_string());
        }

        let remaining = &self.source[self.position..];
        let ch = match remaining.chars().next() {
            Some(c) => c,
            None => return Err("Unterminated character literal".to_string()),
        };

        // Check if it's an escape sequence
        if ch == '\\' {
            self.advance();
            if self.position >= self.source.len() {
                return Err("Unterminated escape sequence".to_string());
            }
            let escaped_remaining = &self.source[self.position..];
            let escaped = match escaped_remaining.chars().next() {
                Some(c) => c,
                None => return Err("Unterminated escape sequence".to_string()),
            };
            let char_value = match escaped {
                'n' => '\n',
                't' => '\t',
                'r' => '\r',
                '\\' => '\\',
                '\'' => '\'',
                '"' => '"',
                _ => escaped,
            };
            self.advance();

            // Check for closing quote
            let closing_remaining = &self.source[self.position..];
            if self.position >= self.source.len()
                || closing_remaining
                    .chars()
                    .next()
                    .map(|c| c != quote)
                    .unwrap_or(true)
            {
                return Err("Character literal must be exactly one character".to_string());
            }
            self.advance(); // Skip closing quote

            Ok(Token::new(
                TokenKind::Char,
                char_value.to_string(),
                (start_line, start_col),
            ))
        } else {
            // Regular character
            let char_value = ch;
            self.advance();

            // Check for closing quote
            let closing_remaining = &self.source[self.position..];
            if self.position >= self.source.len()
                || closing_remaining
                    .chars()
                    .next()
                    .map(|c| c != quote)
                    .unwrap_or(true)
            {
                // Not a char literal, treat as string
                return self.read_string(quote);
            }
            self.advance(); // Skip closing quote

            Ok(Token::new(
                TokenKind::Char,
                char_value.to_string(),
                (start_line, start_col),
            ))
        }
    }

    fn read_string(&mut self, quote: char) -> Result<Token, String> {
        let start_line = self.line;
        let start_col = self.column;
        self.advance(); // Skip opening quote
        let mut value = String::new();
        let mut has_interpolation = false;
        let mut found_closing_quote = false;

        while self.position < self.source.len() {
            let remaining = &self.source[self.position..];
            let ch = match remaining.chars().next() {
                Some(c) => c,
                None => break,
            };
            if ch == quote {
                self.advance(); // Skip closing quote
                found_closing_quote = true;
                break;
            } else if ch == '{' {
                // Check if it's an interpolation (not escaped)
                if self.position > 0 {
                    let prev_remaining = &self.source[self.position - 1..];
                    if let Some(prev_ch) = prev_remaining.chars().next() {
                        if prev_ch != '\\' {
                            has_interpolation = true;
                        }
                    }
                } else {
                    has_interpolation = true;
                }
                value.push(ch);
                self.advance();
            } else if ch == '\\' {
                self.advance();
                if self.position < self.source.len() {
                    let escaped_remaining = &self.source[self.position..];
                    let escaped = match escaped_remaining.chars().next() {
                        Some(c) => c,
                        None => break,
                    };
                    value.push(match escaped {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        '\\' => '\\',
                        '"' => '"',
                        '\'' => '\'',
                        '{' => '{', // Allow escaping {
                        '}' => '}', // Allow escaping }
                        _ => escaped,
                    });
                    self.advance();
                }
            } else {
                value.push(ch);
                self.advance();
            }
        }

        // Check if string was properly terminated
        if !found_closing_quote {
            return Err(format!(
                "Unterminated string literal starting at line {}, column {}",
                start_line, start_col
            ));
        }

        // If string contains unescaped {, mark it as interpolated
        if has_interpolation {
            Ok(Token::new(
                TokenKind::InterpolatedString,
                value,
                (start_line, start_col),
            ))
        } else {
            Ok(Token::new(
                TokenKind::String,
                value,
                (start_line, start_col),
            ))
        }
    }

    fn read_number(&mut self) -> Result<Token, String> {
        let start_line = self.line;
        let start_col = self.column;
        let mut value = String::new();
        let mut has_dot = false;
        let mut has_exponent = false;

        // Check for hex (0x) or binary (0b) prefixes
        if self.position < self.source.len() {
            let remaining = &self.source[self.position..];
            if let Some(ch) = remaining.chars().next() {
                if ch == '0' {
                    let next_remaining = &self.source[self.position + 1..];
                    if let Some(next_ch) = next_remaining.chars().next() {
                        if next_ch == 'x' || next_ch == 'X' {
                            // Hexadecimal literal: 0xFF, 0xABCD
                            self.advance(); // consume '0'
                            self.advance(); // consume 'x' or 'X'
                            value.push('0');
                            value.push('x');

                            // Read hex digits
                            let mut has_digits = false;
                            while self.position < self.source.len() {
                                let hex_remaining = &self.source[self.position..];
                                if let Some(hex_ch) = hex_remaining.chars().next() {
                                    if hex_ch.is_ascii_hexdigit() {
                                        value.push(hex_ch);
                                        has_digits = true;
                                        self.advance();
                                    } else {
                                        break;
                                    }
                                } else {
                                    break;
                                }
                            }

                            if !has_digits {
                                return Err(format!("Hexadecimal literal must have at least one digit at line {}:{}", self.line, self.column));
                            }

                            // Parse hex value
                            let hex_str = &value[2..]; // Skip "0x"
                            let int_val = i64::from_str_radix(hex_str, 16).map_err(|_| {
                                format!(
                                    "Invalid hexadecimal number at line {}:{}",
                                    self.line, self.column
                                )
                            })?;

                            return Ok(Token::new(
                                TokenKind::Integer,
                                int_val.to_string(),
                                (start_line, start_col),
                            ));
                        } else if next_ch == 'b' || next_ch == 'B' {
                            // Binary literal: 0b1010, 0b1111
                            self.advance(); // consume '0'
                            self.advance(); // consume 'b' or 'B'
                            value.push('0');
                            value.push('b');

                            // Read binary digits
                            let mut has_digits = false;
                            while self.position < self.source.len() {
                                let bin_remaining = &self.source[self.position..];
                                if let Some(bin_ch) = bin_remaining.chars().next() {
                                    if bin_ch == '0' || bin_ch == '1' {
                                        value.push(bin_ch);
                                        has_digits = true;
                                        self.advance();
                                    } else {
                                        break;
                                    }
                                } else {
                                    break;
                                }
                            }

                            if !has_digits {
                                return Err(format!(
                                    "Binary literal must have at least one digit at line {}:{}",
                                    self.line, self.column
                                ));
                            }

                            // Parse binary value
                            let bin_str = &value[2..]; // Skip "0b"
                            let int_val = i64::from_str_radix(bin_str, 2).map_err(|_| {
                                format!(
                                    "Invalid binary number at line {}:{}",
                                    self.line, self.column
                                )
                            })?;

                            return Ok(Token::new(
                                TokenKind::Integer,
                                int_val.to_string(),
                                (start_line, start_col),
                            ));
                        }
                    }
                }
            }
        }

        // Regular decimal number parsing
        while self.position < self.source.len() {
            let remaining = &self.source[self.position..];
            let ch = match remaining.chars().next() {
                Some(c) => c,
                None => break,
            };
            if ch == '_' {
                // Number separator: skip underscore (e.g., 1_000_000)
                self.advance();
                continue;
            }
            if ch.is_ascii_digit() {
                value.push(ch);
                self.advance();
            } else if ch == '.' && !has_dot && !has_exponent {
                value.push(ch);
                has_dot = true;
                self.advance();
            } else if (ch == 'e' || ch == 'E') && !has_exponent {
                // Scientific notation: 1e10, 2.5e-3, 1E+5
                value.push(ch);
                has_exponent = true;
                has_dot = true; // Once we have exponent, treat as float
                self.advance();

                // Optional + or - after e/E
                if self.position < self.source.len() {
                    let next_remaining = &self.source[self.position..];
                    if let Some(next_ch) = next_remaining.chars().next() {
                        if next_ch == '+' || next_ch == '-' {
                            value.push(next_ch);
                            self.advance();
                        }
                    }
                }

                // Exponent must be followed by digits
                if self.position >= self.source.len() {
                    return Err(format!(
                        "Exponent must be followed by digits at line {}:{}",
                        self.line, self.column
                    ));
                }
                let digit_remaining = &self.source[self.position..];
                if let Some(digit_ch) = digit_remaining.chars().next() {
                    if !digit_ch.is_ascii_digit() {
                        return Err(format!(
                            "Exponent must be followed by digits at line {}:{}",
                            self.line, self.column
                        ));
                    }
                } else {
                    return Err(format!(
                        "Exponent must be followed by digits at line {}:{}",
                        self.line, self.column
                    ));
                }
            } else {
                break;
            }
        }

        let kind = if has_dot || has_exponent {
            TokenKind::Float
        } else {
            TokenKind::Integer
        };

        Ok(Token::new(kind, value, (start_line, start_col)))
    }

    /// Read a raw string literal: r"..." — no escape processing
    fn read_raw_string(&mut self) -> Result<Token, String> {
        let start_line = self.line;
        let start_col = self.column;
        self.advance(); // Skip opening quote
        let mut value = String::new();
        let mut found_closing = false;

        while self.position < self.source.len() {
            let remaining = &self.source[self.position..];
            let ch = match remaining.chars().next() {
                Some(c) => c,
                None => break,
            };
            if ch == '"' {
                self.advance();
                found_closing = true;
                break;
            } else {
                value.push(ch);
                self.advance();
            }
        }

        if !found_closing {
            return Err(format!(
                "Unterminated raw string literal at line {}, column {}",
                start_line, start_col
            ));
        }

        Ok(Token::new(
            TokenKind::String,
            value,
            (start_line, start_col),
        ))
    }

    /// Read a multiline string literal: """..."""
    fn read_multiline_string(&mut self) -> Result<Token, String> {
        let start_line = self.line;
        let start_col = self.column;
        // Consume opening """
        self.advance();
        self.advance();
        self.advance();
        let mut value = String::new();
        let mut found_closing = false;

        while self.position < self.source.len() {
            // Check for closing """
            if self.source[self.position..].starts_with("\"\"\"") {
                self.advance();
                self.advance();
                self.advance();
                found_closing = true;
                break;
            }
            let remaining = &self.source[self.position..];
            let ch = match remaining.chars().next() {
                Some(c) => c,
                None => break,
            };
            value.push(ch);
            self.advance();
        }

        if !found_closing {
            return Err(format!(
                "Unterminated multiline string literal at line {}, column {}",
                start_line, start_col
            ));
        }

        // Multiline strings may contain unescaped { for interpolation
        let has_interpolation = value.contains('{');
        if has_interpolation {
            Ok(Token::new(
                TokenKind::InterpolatedString,
                value,
                (start_line, start_col),
            ))
        } else {
            Ok(Token::new(
                TokenKind::String,
                value,
                (start_line, start_col),
            ))
        }
    }

    fn read_identifier(&mut self) -> Result<Token, String> {
        let start_line = self.line;
        let start_col = self.column;
        let mut value = String::new();

        while self.position < self.source.len() {
            let remaining = &self.source[self.position..];
            if let Some(ch) = remaining.chars().next() {
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    value.push(ch);
                    self.advance();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        let kind = if is_keyword(&value) {
            TokenKind::Keyword
        } else {
            TokenKind::Identifier
        };

        Ok(Token::new(kind, value, (start_line, start_col)))
    }
}
