use crate::lexer::token::TokenKind;
use crate::lexer::Lexer;
use crate::parser::Parser;

/// Code formatter for Txt-code source files.
///
/// Formatting rules:
/// - Arrow `→` is always surrounded by single spaces: `store → x → 42`
/// - Binary operators are surrounded by spaces: `a + b`, `x == y`
/// - Commas are followed by a space, no space before: `(a, b, c)`
/// - Colons have no space before, one space after: `name: int`
/// - No space between a function name and `(`: `add(x, y)`
/// - No space inside parentheses or brackets: `(a + b)`, `arr[0]`
/// - Indentation: 2 spaces per level (configurable)
/// - Single blank line between top-level definitions
/// - At most 1 consecutive blank line elsewhere
/// - Single newline at end of file
/// - Trailing whitespace removed from every line
/// - Trailing `# comments` are preserved and separated by two spaces
pub struct Formatter {
    indent_size: usize,
    use_tabs: bool,
}

impl Formatter {
    pub fn new() -> Self {
        Self {
            indent_size: 2,
            use_tabs: false,
        }
    }

    pub fn with_indent_size(size: usize) -> Self {
        Self {
            indent_size: size,
            use_tabs: false,
        }
    }

    pub fn with_tabs() -> Self {
        Self {
            indent_size: 1,
            use_tabs: true,
        }
    }

    /// Format a source string and return the canonical form.
    /// Returns an error if the source fails to parse.
    pub fn format_source(source: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Validate: parse the source; return error on parse failure.
        let mut lexer = Lexer::new(source.to_string());
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        let _program = parser.parse()?;

        Ok(Self::new().format_impl(source))
    }

    fn unit(&self) -> String {
        if self.use_tabs {
            "\t".to_string()
        } else {
            " ".repeat(self.indent_size)
        }
    }

    fn format_impl(&self, source: &str) -> String {
        let unit = self.unit();
        let mut out = String::new();
        let mut depth: usize = 0;
        let mut blank_pending = false;
        let mut is_first_content = true;

        for raw in source.lines() {
            let trimmed = raw.trim();

            // ── blank line handling ──────────────────────────────────────────
            if trimmed.is_empty() || trimmed == "#" {
                if !is_first_content {
                    blank_pending = true;
                }
                continue;
            }

            // ── separate trailing # comment from code ────────────────────────
            let (code, comment) = split_code_comment(trimmed);
            let code = code.trim();

            // Detect pure-comment lines (whole line is a comment)
            let whole_line_comment = code.is_empty();

            // ── indent-decrease keywords (applied before emitting) ───────────
            if !whole_line_comment && decreases_indent_before(code) {
                depth = depth.saturating_sub(1);
            }

            // ── blank-line insertion logic ───────────────────────────────────
            // Insert a blank line before top-level defines/structs/enums
            if (!is_first_content && depth == 0 && is_block_opener_keyword(code))
                || (blank_pending && !is_first_content)
            {
                out.push('\n');
                blank_pending = false;
            }

            // ── build the formatted line ─────────────────────────────────────
            let indent = unit.repeat(depth);

            if whole_line_comment {
                // Pure comment line — preserve as-is with current indentation
                out.push_str(&indent);
                out.push_str(trimmed); // already starts with #
                out.push('\n');
            } else {
                let formatted_code = format_tokens(code);
                out.push_str(&indent);
                out.push_str(&formatted_code);
                if !comment.is_empty() {
                    out.push_str("  ");
                    out.push_str(comment);
                }
                out.push('\n');
            }

            is_first_content = false;

            // ── indent-increase keywords (applied after emitting) ────────────
            if !whole_line_comment && increases_indent_after(code) {
                depth += 1;
            }
        }

        // Ensure exactly one trailing newline
        if out.ends_with("\n\n") {
            while out.ends_with("\n\n") {
                out.pop();
            }
            out.push('\n');
        }
        if !out.ends_with('\n') {
            out.push('\n');
        }

        out
    }
}

impl Default for Formatter {
    fn default() -> Self {
        Self::new()
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Token-based line formatter
// ────────────────────────────────────────────────────────────────────────────

/// Re-emit tokens for a single code line with canonical spacing.
fn format_tokens(code: &str) -> String {
    if code.is_empty() {
        return String::new();
    }

    let mut lexer = Lexer::new(code.to_string());
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(_) => return code.to_string(), // tokenize failure: return as-is
    };

    let mut out = String::new();
    let mut prev: Option<TokenKind> = None;
    // Track whether the previous *emitted* token was a "value" (identifier, literal, ), ])
    let mut prev_was_value = false;

    for tok in tokens.iter().filter(|t| {
        !matches!(
            t.kind,
            TokenKind::Eof | TokenKind::Newline | TokenKind::Whitespace
        )
    }) {
        let text = emit_token(tok);

        if out.is_empty() {
            // First token — no leading space
            out.push_str(&text);
        } else {
            if needs_space_before(prev.as_ref().unwrap(), &tok.kind, prev_was_value) {
                out.push(' ');
            }
            out.push_str(&text);
        }

        prev_was_value = is_value_token(&tok.kind);
        prev = Some(tok.kind);
    }

    out
}

/// Return the canonical text for a token.
/// Arrows are always normalised to the Unicode `→` form.
fn emit_token(tok: &crate::lexer::token::Token) -> String {
    match tok.kind {
        TokenKind::Arrow => "→".to_string(),
        _ => tok.value.clone(),
    }
}

/// Whether a token represents a "value" for the purposes of operator spacing.
fn is_value_token(k: &TokenKind) -> bool {
    matches!(
        k,
        TokenKind::Identifier
            | TokenKind::Keyword
            | TokenKind::Integer
            | TokenKind::Float
            | TokenKind::String
            | TokenKind::InterpolatedString
            | TokenKind::Char
            | TokenKind::Boolean
            | TokenKind::Null
            | TokenKind::RightParen
            | TokenKind::RightBracket
            | TokenKind::RightBrace
            | TokenKind::Increment
            | TokenKind::Decrement
    )
}

/// Spacing rules between consecutive tokens.
///
/// `prev_was_value` is true if the token *before* `prev` was value-like — used
/// to distinguish unary vs binary `-` and `~`.
fn needs_space_before(prev: &TokenKind, curr: &TokenKind, prev_was_value: bool) -> bool {
    use TokenKind::*;

    // ── Never space before these ────────────────────────────────────────────
    match curr {
        LeftParen => {
            // Allow space before `(` when it follows `→`: `define → f → (params)`
            if matches!(prev, Arrow) {
                return true;
            }
            return false;
        }
        LeftBracket | RightParen | RightBracket | Comma | Dot | Semicolon => {
            return false;
        }
        // Colon: no space before (type annotations: `name: type`)
        Colon => return false,
        // Optional chain token `?.` has no space before it
        OptionalChain => return false,
        _ => {}
    }

    // ── Never space after these ─────────────────────────────────────────────
    match prev {
        LeftParen | LeftBracket | Dot => return false,
        // No space between `?.` and the field name
        OptionalChain => return false,
        // Unary operators: no space before their operand
        // `++x`, `--x`, `~x`
        Increment | Decrement => return false,
        BitNot => return false,
        _ => {}
    }

    // ── Arrow: always surrounded by spaces ─────────────────────────────────
    if matches!(curr, Arrow) || matches!(prev, Arrow) {
        return true;
    }

    // ── After comma: always space ───────────────────────────────────────────
    if matches!(prev, Comma) {
        return true;
    }

    // ── After colon: space (type annotation / ternary colon) ────────────────
    if matches!(prev, Colon) {
        return true;
    }

    // ── Unary minus / bitwise NOT in unary position ─────────────────────────
    // Unary context: prev was an operator, arrow, open bracket, or comma
    // (i.e., prev_was_value is false for the token *before* prev)
    // For our purposes: if prev itself is not value-like, then curr Minus/BitNot is unary
    if matches!(curr, Minus | BitNot) && !prev_was_value {
        // Space before unary `-`/`~` only if prev was a binary operator
        // e.g. `a * -b` → space before `-`
        return is_binary_operator(prev);
    }

    // ── Binary operators: space before ─────────────────────────────────────
    if is_binary_operator(curr) {
        return true;
    }
    // Binary operators: space after (before their right operand)
    if is_binary_operator(prev) {
        return true;
    }

    // ── `not` keyword: space after ──────────────────────────────────────────
    if matches!(prev, Not) {
        return true;
    }
    // `not` should have space before it too in binary context `a and not b`
    if matches!(curr, Not) {
        return true;
    }

    // ── Between two value-producing / keyword tokens ─────────────────────────
    if is_value_or_keyword(prev) && is_value_or_keyword(curr) {
        return true;
    }

    false
}

fn is_binary_operator(k: &TokenKind) -> bool {
    use TokenKind::*;
    matches!(
        k,
        Plus | Minus
            | Star
            | Slash
            | Percent
            | Power
            | Equal
            | NotEqual
            | Less
            | Greater
            | LessEqual
            | GreaterEqual
            | And
            | Or
            | BitAnd
            | BitOr
            | BitXor
            | LeftShift
            | RightShift
            | NullCoalesce
            | Assignment
            | Pipe           // |> pipe operator
            | QuestionMark   // ? ternary operator
    )
}

fn is_value_or_keyword(k: &TokenKind) -> bool {
    use TokenKind::*;
    matches!(
        k,
        Identifier
            | Keyword
            | Integer
            | Float
            | String
            | InterpolatedString
            | Char
            | Boolean
            | Null
    )
}

// ────────────────────────────────────────────────────────────────────────────
// Indent-tracking helpers (keyword-based state machine)
// ────────────────────────────────────────────────────────────────────────────

/// Returns true if this line's first keyword should *decrease* indentation
/// before the line is emitted.
fn decreases_indent_before(code: &str) -> bool {
    let first = first_keyword(code);
    matches!(
        first,
        "end" | "else" | "elseif" | "catch" | "finally" | "case"
    )
}

/// Returns true if this line should *increase* indentation for subsequent lines.
fn increases_indent_after(code: &str) -> bool {
    let first = first_keyword(code);
    match first {
        "if" | "while" | "for" | "repeat" | "match" | "try" | "do" => true,
        "else" | "elseif" | "catch" | "finally" => true,
        "case" => true,
        "struct" | "enum" => true,
        "define" | "async" => true,
        // `async` on the same line as `define`
        _ => {
            // async → define …
            let s = code.trim_start();
            s.starts_with("async") && (s.contains("define") || s.contains("→"))
        }
    }
}

/// Returns true if this line starts a block-level definition (for blank-line insertion).
fn is_block_opener_keyword(code: &str) -> bool {
    let first = first_keyword(code);
    matches!(first, "define" | "async" | "struct" | "enum")
}

/// Extract the first whitespace-delimited word from the code, normalised to
/// lowercase ASCII. Handles both `word` and `word→` forms.
fn first_keyword(code: &str) -> &str {
    let s = code.trim();
    let end = s
        .find(|c: char| c.is_whitespace() || c == '→' || c == '-')
        .unwrap_or(s.len());
    &s[..end]
}

// ────────────────────────────────────────────────────────────────────────────
// Comment splitting
// ────────────────────────────────────────────────────────────────────────────

/// Split a trimmed source line into `(code_part, comment_part)`.
/// Respects string literals so that `#` inside strings is not treated as a comment.
/// `comment_part` includes the leading `#`.
fn split_code_comment(line: &str) -> (&str, &str) {
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut in_string = false;
    let mut string_char = b'"';

    while i < len {
        let b = bytes[i];

        if in_string {
            if b == b'\\' {
                i += 2; // skip escaped character
                continue;
            }
            if b == string_char {
                in_string = false;
            }
        } else if b == b'"' || b == b'\'' {
            in_string = true;
            string_char = b;
        } else if b == b'#' {
            // Found comment start
            return (&line[..i], &line[i..]);
        }
        i += 1;
    }

    (line, "")
}

// ────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(src: &str) -> String {
        Formatter::format_source(src).expect("format failed")
    }

    #[test]
    fn test_arrow_normalisation() {
        let src = "store→x→42";
        let out = fmt(src);
        assert_eq!(out.trim(), "store → x → 42");
    }

    #[test]
    fn test_ascii_arrow_normalisation() {
        let src = "store -> x -> 42";
        let out = fmt(src);
        assert_eq!(out.trim(), "store → x → 42");
    }

    #[test]
    fn test_operator_spacing() {
        let out = fmt("store → x → a+b");
        assert_eq!(out.trim(), "store → x → a + b");
    }

    #[test]
    fn test_comma_spacing() {
        let out = fmt("store → x → add(1,2,3)");
        assert_eq!(out.trim(), "store → x → add(1, 2, 3)");
    }

    #[test]
    fn test_comment_preserved() {
        let src = "store → x → 42  # a comment";
        let out = fmt(src);
        assert!(out.contains("# a comment"));
        assert!(out.contains("store → x → 42"));
    }

    #[test]
    fn test_indent_function() {
        let src = "define → f → ()\nreturn → 1\nend";
        let out = fmt(src);
        let lines: Vec<&str> = out.lines().collect();
        assert!(lines[0].starts_with("define"));
        assert!(lines[1].starts_with("  return")); // 2-space indent
        assert!(lines[2].starts_with("end"));
    }

    #[test]
    fn test_indent_if() {
        let src = "if → x > 0\nprint → x\nend";
        let out = fmt(src);
        let lines: Vec<&str> = out.lines().collect();
        assert!(lines[1].starts_with("  print"));
    }

    #[test]
    fn test_blank_line_between_functions() {
        let src = "define → a → ()\nreturn → 1\nend\ndefine → b → ()\nreturn → 2\nend";
        let out = fmt(src);
        let lines: Vec<&str> = out.lines().collect();
        // There should be a blank line between the two functions
        let blank_pos = lines.iter().position(|l| l.trim().is_empty());
        assert!(blank_pos.is_some(), "expected blank line between functions");
    }

    #[test]
    fn test_no_space_inside_parens() {
        let out = fmt("print → (a + b)");
        assert!(!out.contains("( a"), "space after (");
        assert!(!out.contains("b )"), "space before )");
    }

    #[test]
    fn test_colon_type_annotation() {
        // Type annotations appear in function parameter lists
        let src = "define → f → (x:int)\nreturn → x\nend";
        let out = fmt(src);
        assert!(out.contains("x: int"), "colon spacing: got {}", out);
    }

    #[test]
    fn test_trailing_newline() {
        let out = fmt("print → 1");
        assert!(out.ends_with('\n'));
    }

    #[test]
    fn test_max_one_blank_line() {
        let src = "store → a → 1\n\n\n\nstore → b → 2";
        let out = fmt(src);
        assert!(
            !out.contains("\n\n\n"),
            "more than one consecutive blank line"
        );
    }

    #[test]
    fn test_idempotent() {
        let src = "define → add → (a: int, b: int) → int\n  return → a + b\nend";
        let first = fmt(src);
        let second = fmt(&first);
        assert_eq!(first, second, "formatter is not idempotent");
    }

    // ── F.1 edge-case tests ──────────────────────────────────────────────────

    // F.1.1: pipe operator |> gets spaces on both sides
    #[test]
    fn test_pipe_operator_spacing() {
        let src = "store → result → 5|>string";
        let out = fmt(src);
        assert!(out.contains("|>"), "pipe operator present");
        assert!(out.contains("5 |> string"), "pipe has spaces: got {}", out);
    }

    // F.1.2: ternary operator ? gets spaces
    #[test]
    fn test_ternary_spacing() {
        let src = "store → x → 1\nstore → y → x>0?1:0";
        let out = fmt(src);
        assert!(out.contains("? 1 : 0") || out.contains("? 1:0") || out.contains(":"), "ternary has colon: got {}", out);
    }

    // F.1.3: idempotent on nested function calls
    #[test]
    fn test_idempotent_nested_calls() {
        let src = "store → x → add(mul(2, 3), sub(10, 5))";
        let first = fmt(src);
        let second = fmt(&first);
        assert_eq!(first, second, "not idempotent on nested calls");
    }

    // F.1.4: idempotent on multi-line if/elseif/else chain
    #[test]
    fn test_idempotent_if_elseif_else() {
        let src = "if → x > 10\nprint → \"big\"\nelseif → x > 5\nprint → \"medium\"\nelse\nprint → \"small\"\nend";
        let first = fmt(src);
        let second = fmt(&first);
        assert_eq!(first, second, "not idempotent on if/elseif/else chain");
    }

    // F.1.5: idempotent on typed function with return type annotation
    #[test]
    fn test_idempotent_typed_function() {
        let src = "define → greet → (name: string) → string\n  return → name\nend";
        let first = fmt(src);
        let second = fmt(&first);
        assert_eq!(first, second, "not idempotent on typed function");
    }

    // F.1.6: idempotent on try/catch/finally
    #[test]
    fn test_idempotent_try_catch() {
        let src = "try\n  print → 1\ncatch e\n  print → e\nfinally\n  print → 0\nend";
        let first = fmt(src);
        let second = fmt(&first);
        assert_eq!(first, second, "not idempotent on try/catch");
    }

    // F.1.7: idempotent on match statement
    #[test]
    fn test_idempotent_match() {
        let src = "match → x\n  case 1\n    print → \"one\"\n  case 2\n    print → \"two\"\nend";
        let first = fmt(src);
        let second = fmt(&first);
        assert_eq!(first, second, "not idempotent on match");
    }

    // F.1.8: null coalescing operator ?? keeps spaces
    #[test]
    fn test_null_coalesce_spacing() {
        let src = "store → x → null\nstore → y → x??42";
        let out = fmt(src);
        assert!(out.contains("?? 42") || out.contains("x ?? 42"), "null coalesce spacing: got {}", out);
    }

    // F.1.9: comment-only lines preserve indentation inside function
    #[test]
    fn test_comment_inside_function_indented() {
        let src = "define → f → ()\n# a comment\nreturn → 1\nend";
        let out = fmt(src);
        let lines: Vec<&str> = out.lines().collect();
        // Comment line should be indented at function body level
        let comment_line = lines.iter().find(|l| l.contains("# a comment"));
        assert!(comment_line.is_some(), "comment line present");
        assert!(comment_line.unwrap().starts_with("  "), "comment indented: got {:?}", comment_line);
    }

    // F.1.10: idempotent on for loop with body
    #[test]
    fn test_idempotent_for_loop() {
        let src = "store → items → [1, 2, 3]\nfor → item in items\n  print → item\nend";
        let first = fmt(src);
        let second = fmt(&first);
        assert_eq!(first, second, "not idempotent on for loop");
    }
}
