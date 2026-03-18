// async_executor: removed in Phase 4.1 — zero callers, pulls in tokio::full for nothing.
// trace: removed in Phase 4.2 — never connected to any VM; unsound raw-pointer graph.
// Both will be redesigned as proper subsystems when actually needed.
pub mod audit;
#[cfg(feature = "bytecode")]
pub mod bytecode_vm;
pub mod compatibility;
pub mod security_pipeline;
pub mod core;
pub mod errors;
pub mod execution;
pub mod gc;
pub mod intent;
pub mod migration;
pub mod module;
pub mod module_metadata;
pub mod operators;
pub mod permission_map;
pub mod permissions;
pub mod security;
pub mod tools;
pub mod vm;

// CANONICAL IMPLEMENTATION: src/capability/manager.rs (re-exported via src/capability/mod.rs)
// This inline shim keeps old `crate::runtime::capabilities::*` import paths working.
// DO NOT add a `capabilities.rs` file here — it would be shadowed by this inline block
// and become dead code, which was the root cause of a previous duplicate divergence bug.
pub mod capabilities {
    pub use crate::capability::*;
}

#[cfg(feature = "bytecode")]
#[allow(unused_imports)]
pub use bytecode_vm::*;
#[allow(unused_imports)]
pub use gc::*;
#[allow(unused_imports)]
pub use security::*;
#[allow(unused_imports)]
pub use vm::*;

// Re-export core types for backward compatibility
pub use core::{CallFrame, CallStack, Value};
pub use errors::{ErrorCode, RuntimeError};
