// Type-checker module — static type inference and checking.
// This module is NOT invoked on the primary `txtcode run` path.
// It is used by: `txtcode check` (static analysis command) and the REPL `:type <expr>` command.
// Type annotations in .tc programs are advisory and do not affect runtime behaviour.
pub mod checker;
pub mod inference;
pub mod types;

#[allow(unused_imports)]
pub use checker::*;
#[allow(unused_imports)]
pub use inference::*;
#[allow(unused_imports)]
pub use types::*;
