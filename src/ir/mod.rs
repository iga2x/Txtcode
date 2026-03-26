//! Backend-agnostic Intermediate Representation (IR).
//!
//! Enabled with `--features ir` (off by default; experimental).
//! All execution flows through `Builder`, which calls `IrBuilder::lower()`
//! after validation.
//!
//! ## Guarantees
//! - All constant folding has been applied before any backend sees the IR.
//! - Control flow is structured (`If`, `Loop`, `ForEach`) — no flat jump targets.
//! - Every stdlib call that touches a guarded resource is wrapped in an
//!   `IrNode::CapabilityCall` so backends can emit checks without re-parsing
//!   function names.
//!
//! ## Status (R.2)
//! The IR is currently informational only.  `Builder::run()` lowers to IR and
//! records fold/dead-branch counts but the AST-walking VM still executes the
//! original `Program`.  When a backend migrates to consume IR, it will call
//! `IrBuilder::lower()` and walk `ProgramIr::nodes` instead of the AST.

pub mod builder;
pub mod instruction;
pub mod program;

pub use builder::IrBuilder;
pub use instruction::{CapabilityCall, IrNode, IrParam};
pub use program::ProgramIr;
