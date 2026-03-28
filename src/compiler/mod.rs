pub mod bytecode;
pub mod optimizer;
/// Task 12.3 — WASM compilation target
pub mod wasm;
/// Task 29.2 — WASM binary output (requires wasm feature)
pub mod wasm_binary;

#[allow(unused_imports)]
pub use bytecode::*;
#[allow(unused_imports)]
pub use optimizer::*;
pub use wasm::WasmCompiler;
