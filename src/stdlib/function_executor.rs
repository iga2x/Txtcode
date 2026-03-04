use crate::runtime::{Value, RuntimeError};

/// Trait for executing function values (used by higher-order functions)
pub trait FunctionExecutor {
    fn call_function_value(&mut self, func: &Value, args: &[Value]) -> Result<Value, RuntimeError>;
}

