pub mod async_executor;
pub mod audit;
pub mod bytecode_runner;
pub mod bytecode_vm;
pub mod compatibility;
pub mod core;
pub mod errors;
pub mod execution;
pub mod gc;
pub mod intent;
pub mod memory;
pub mod migration;
pub mod module;
pub mod module_metadata;
pub mod operators;
pub mod permissions;
pub mod security;
pub mod tools;
pub mod trace;
pub mod vm;

// Policy lives at crate::policy (src/policy/); this shim keeps
// `crate::runtime::policy::*` import paths working.
pub mod policy {
    pub use crate::policy::*;
}

// CANONICAL IMPLEMENTATION: src/capability/manager.rs (re-exported via src/capability/mod.rs)
// This inline shim keeps old `crate::runtime::capabilities::*` import paths working.
// DO NOT add a `capabilities.rs` file here — it would be shadowed by this inline block
// and become dead code, which was the root cause of a previous duplicate divergence bug.
pub mod capabilities {
    pub use crate::capability::*;
}

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
