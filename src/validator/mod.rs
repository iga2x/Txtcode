// Validator layer - separates parsing from validation
// Validates syntax, semantics, and pentest-specific restrictions

pub mod syntax;
pub mod semantics;
pub mod restrictions;

pub use syntax::SyntaxValidator;
pub use semantics::SemanticsValidator;
pub use restrictions::RestrictionChecker;

use crate::parser::ast::Program;

/// Main validator entry point
pub struct Validator;

impl Validator {
    /// Validate a complete program
    pub fn validate_program(program: &Program) -> Result<(), ValidationError> {
        // 1. Syntax validation
        SyntaxValidator::check_program(program)?;
        
        // 2. Semantic validation (type checking)
        SemanticsValidator::check_program(program)?;
        
        // 3. Restriction checking (pentest-specific rules)
        RestrictionChecker::check_program(program)?;
        
        Ok(())
    }
}

/// Validation error
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    Syntax(String),
    Semantic(String),
    Restriction(String),
}

impl ValidationError {
    pub fn message(&self) -> &str {
        match self {
            ValidationError::Syntax(msg) => msg,
            ValidationError::Semantic(msg) => msg,
            ValidationError::Restriction(msg) => msg,
        }
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for ValidationError {}

