use crate::runtime::{RuntimeError, Value};

/// Trait for executing function values (used by higher-order functions)
pub trait FunctionExecutor {
    fn call_function_value(&mut self, func: &Value, args: &[Value]) -> Result<Value, RuntimeError>;

    /// Returns the deterministic time override if deterministic mode is active.
    /// Default: always `None` (use real system time).
    fn deterministic_time(&self) -> Option<std::time::SystemTime> {
        None
    }

    /// Returns the deterministic random seed if deterministic mode is active.
    /// Default: always `None` (use OS entropy).
    fn deterministic_random_seed(&self) -> Option<u64> {
        None
    }
}
