// Abstract Syntax Tree module
// Modular structure for better maintainability

pub mod capabilities; // Phase 4: AST-based capabilities
pub mod common;
pub mod expressions;
pub mod statements;
pub mod time; // Prepared for Phase 4 (not exported yet - conflicts with stdlib::time)

// Re-export everything for backward compatibility
pub use capabilities::{CapabilityExpr, DurationExpr, RateLimitExpr};
pub use common::*;
pub use expressions::*;
pub use statements::*;

// Time module not exported yet to avoid conflicts with stdlib::time
