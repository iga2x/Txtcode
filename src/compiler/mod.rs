#[cfg(feature = "bytecode")]
pub mod bytecode;
#[cfg(feature = "bytecode")]
pub mod optimizer;
/// Task 12.3 — WASM compilation target (requires bytecode feature for IR)
#[cfg(feature = "bytecode")]
pub mod wasm;
/// Task 29.2 — WASM binary output (requires bytecode + wasm features)
#[cfg(feature = "bytecode")]
pub mod wasm_binary;

#[cfg(feature = "bytecode")]
#[allow(unused_imports)]
pub use bytecode::*;
#[cfg(feature = "bytecode")]
#[allow(unused_imports)]
pub use optimizer::*;
#[cfg(feature = "bytecode")]
pub use wasm::WasmCompiler;
