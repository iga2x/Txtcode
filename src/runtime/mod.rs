pub mod async_executor;
pub mod audit;
pub mod bytecode_runner;
pub mod bytecode_vm;
pub mod core;
pub mod errors;
pub mod execution;
pub mod gc;
pub mod memory;
pub mod module;
pub mod operators;
pub mod permissions;
pub mod security;
pub mod vm; // NEW: Audit trail system
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
pub mod compatibility; // NEW: AST-level compatibility and migration system
pub mod migration; // NEW: Migration framework with dry-run and reporting
pub mod module_metadata;
pub mod tools; // NEW: Tool orchestration interface for pentest tools
pub mod trace; // NEW: Execution trace system for replayable execution graphs // NEW: Module-scoped feature flags and metadata

#[allow(unused_imports)]
pub use bytecode_runner::*;
#[allow(unused_imports)]
pub use bytecode_vm::*;
#[allow(unused_imports)]
pub use gc::*;
#[allow(unused_imports)]
pub use memory::*;
#[allow(unused_imports)]
pub use security::*;
#[allow(unused_imports)]
pub use vm::*;

// Re-export core types for backward compatibility
pub use core::{CallFrame, CallStack, Value};
pub use errors::RuntimeError;
