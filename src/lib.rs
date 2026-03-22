// RuntimeError is intentionally large to carry control-flow signals (return/break/continue)
// without additional heap allocation. Boxing would require widespread API changes.
#![allow(clippy::result_large_err)]

pub mod capability;
pub mod cli;
pub mod compiler;
pub mod embed;
pub mod config;
pub mod lexer;
pub mod parser;
pub mod policy;
pub mod runtime;
pub mod security;
pub mod stdlib;
pub mod tools;
pub mod typecheck;
pub mod validator;

#[allow(unused_imports)]
pub use compiler::*;
pub use config::*;
#[allow(unused_imports)]
pub use lexer::*;
#[allow(unused_imports)]
#[allow(ambiguous_glob_reexports)]
pub use parser::*;
#[allow(unused_imports)]
#[allow(ambiguous_glob_reexports)] // runtime::core and stdlib::core are different modules
pub use runtime::*;
#[allow(unused_imports)]
pub use security::*;
#[allow(unused_imports)]
#[allow(ambiguous_glob_reexports)] // runtime::core and stdlib::core are different modules
pub use stdlib::*;
#[allow(unused_imports)]
pub use tools::*;
#[allow(unused_imports)]
pub use typecheck::*;
