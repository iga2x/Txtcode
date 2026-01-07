use crate::lexer::Lexer;
use crate::parser::Parser;

/// Code formatter
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

    pub fn format_source(source: &str) -> Result<String, Box<dyn std::error::Error>> {
        let formatter = Self::new();
        
        // Parse the source to ensure it's valid
        let mut lexer = Lexer::new(source.to_string());
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        let _program = parser.parse()?;
        
        // Format the source
        // For now, return the source as-is (basic formatting)
        // In a full implementation, this would reformat according to style rules
        Ok(formatter.format_string(source))
    }

    fn format_string(&self, source: &str) -> String {
        let mut formatted = String::new();
        let mut indent: usize = 0;
        let indent_str = if self.use_tabs {
            "\t".to_string()
        } else {
            " ".repeat(self.indent_size)
        };

        for line in source.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                formatted.push('\n');
                continue;
            }

            // Decrease indent for 'end' statements
            if trimmed == "end" || trimmed.starts_with("else") {
                indent = indent.saturating_sub(1);
            }

            // Add indentation
            formatted.push_str(&indent_str.repeat(indent));
            formatted.push_str(trimmed);
            formatted.push('\n');

            // Increase indent for control structures
            if trimmed.starts_with("if") || trimmed.starts_with("while") || 
               trimmed.starts_with("for") || trimmed.starts_with("repeat") ||
               trimmed.starts_with("define") || trimmed.starts_with("match") ||
               trimmed.starts_with("try") {
                indent += 1;
            }
        }

        formatted
    }
}

impl Default for Formatter {
    fn default() -> Self {
        Self::new()
    }
}

