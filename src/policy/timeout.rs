// Timeout policy - controls maximum execution time

use std::time::{SystemTime, Duration};

/// Timeout policy configuration
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

