// Timeout policy - controls maximum execution time

use std::time::{Duration, SystemTime};

/// Timeout policy configuration.
///
/// NOTE: `TimeoutPolicy` is not wired to `PolicyEngine` or the VM. The VM
/// enforces execution time limits through `PolicyEngine::check_max_execution_time`.
/// This struct is available for direct use in library consumers but is redundant
/// with `Policy::set_max_execution_time` / `PolicyEngine`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeoutPolicy {
    pub max_execution_time: Option<Duration>,
}

impl TimeoutPolicy {
    pub fn new() -> Self {
        Self {
            max_execution_time: None,
        }
    }

    pub fn with_max_time(mut self, duration: Duration) -> Self {
        self.max_execution_time = Some(duration);
        self
    }

    /// Check if execution time has exceeded limit
    pub fn check(&self, start_time: SystemTime) -> Result<(), String> {
        if let Some(max_time) = self.max_execution_time {
            let elapsed = start_time.elapsed().unwrap_or(Duration::ZERO);
            if elapsed > max_time {
                return Err(format!(
                    "Maximum execution time exceeded: {} seconds (max: {} seconds)",
                    elapsed.as_secs(),
                    max_time.as_secs()
                ));
            }
        }
        Ok(())
    }
}

impl Default for TimeoutPolicy {
    fn default() -> Self {
        Self::new()
    }
}
