use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::typecheck::TypeChecker;

#[derive(Debug, Clone)]
pub struct LintIssue {
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// Code linter
pub struct Linter {
    check_types: bool,
    check_security: bool,
    check_style: bool,
}

impl Linter {
    pub fn new() -> Self {
        Self {
            check_types: true,
            check_security: true,
            check_style: true,
        }
    }

    pub fn lint_source(source: &str) -> Result<Vec<LintIssue>, Box<dyn std::error::Error>> {
        let linter = Self::new();
        let mut issues = Vec::new();

        // Lexical analysis
        let mut lexer = Lexer::new(source.to_string());
        let tokens = lexer.tokenize()?;

        // Parse
        let mut parser = Parser::new(tokens);
        let program = parser.parse()?;

        // Type checking
        if linter.check_types {
            let mut type_checker = TypeChecker::new();
            if let Err(type_errors) = type_checker.check_program(&program) {
                for error in type_errors {
                    issues.push(LintIssue {
                        line: error.span.line,
                        column: error.span.column,
                        message: error.message,
                        severity: Severity::Error,
                    });
                }
            }
        }

        // Security checks
        if linter.check_security {
            issues.extend(linter.check_security_issues(&program));
        }

        // Style checks
        if linter.check_style {
            issues.extend(linter.check_style_issues(source));
        }

        Ok(issues)
    }

    fn check_security_issues(&self, _program: &crate::parser::ast::Program) -> Vec<LintIssue> {
        let issues = Vec::new();
        
        // Check for potential security issues
        // - Use of eval (if implemented)
        // - Unsafe file operations
        // - Hardcoded secrets
        // etc.

        issues
    }

    fn check_style_issues(&self, source: &str) -> Vec<LintIssue> {
        let mut issues = Vec::new();
        let mut line_num = 1;

        for line in source.lines() {
            // Check line length
            if line.len() > 120 {
                issues.push(LintIssue {
                    line: line_num,
                    column: 0,
                    message: "Line exceeds 120 characters".to_string(),
                    severity: Severity::Warning,
                });
            }

            // Check for trailing whitespace
            if line.ends_with(' ') || line.ends_with('\t') {
                issues.push(LintIssue {
                    line: line_num,
                    column: line.len(),
                    message: "Trailing whitespace".to_string(),
                    severity: Severity::Info,
                });
            }

            line_num += 1;
        }

        issues
    }
}

impl Default for Linter {
    fn default() -> Self {
        Self::new()
    }
}

