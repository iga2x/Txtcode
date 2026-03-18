#[cfg(feature = "bytecode")]
pub mod bytecode;
#[cfg(feature = "bytecode")]
pub mod optimizer;

#[cfg(feature = "bytecode")]
#[allow(unused_imports)]
pub use bytecode::*;
#[cfg(feature = "bytecode")]
#[allow(unused_imports)]
pub use optimizer::*;
