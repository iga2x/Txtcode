pub mod vm;
pub mod bytecode_vm;
pub mod bytecode_runner;
pub mod async_executor;
pub mod memory;
pub mod gc;
pub mod security;
pub mod module;
pub mod core;
pub mod errors;
pub mod operators;
pub mod execution;
pub mod permissions;
pub mod audit; // NEW: Audit trail system
// Policy moved to top-level: src/policy/
// This is a compatibility shim - use crate::policy instead
pub mod policy {
    pub use crate::policy::*;
}
pub mod intent; // NEW: Intent enforcement system
// Capabilities moved to top-level: src/capability/
// This is a compatibility shim - use crate::capability instead
pub mod capabilities {
    pub use crate::capability::*;
}
pub mod tools; // NEW: Tool orchestration interface for pentest tools
pub mod trace; // NEW: Execution trace system for replayable execution graphs
pub mod compatibility; // NEW: AST-level compatibility and migration system
pub mod migration; // NEW: Migration framework with dry-run and reporting
pub mod module_metadata; // NEW: Module-scoped feature flags and metadata

#[allow(unused_imports)]
pub use vm::*;
#[allow(unused_imports)]
pub use bytecode_vm::*;
#[allow(unused_imports)]
pub use bytecode_runner::*;
#[allow(unused_imports)]
pub use memory::*;
#[allow(unused_imports)]
pub use gc::*;
#[allow(unused_imports)]
pub use security::*;

// Re-export core types for backward compatibility
pub use core::{Value, CallFrame, CallStack};
pub use errors::RuntimeError;

