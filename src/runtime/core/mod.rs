pub mod scope;
pub mod stack;
pub mod value;

pub use scope::ScopeManager;
pub use stack::{CallFrame, CallStack};
pub use value::Value;
