// Rate limiting policy - controls action frequency

use std::time::{Duration, SystemTime};

/// Rate limit configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimit {
    pub count: u64,          // Number of allowed actions
    pub window_seconds: u64, // Time window in seconds
}

impl RateLimit {
    pub fn new(count: u64, window_seconds: u64) -> Self {
        Self {
            count,
            window_seconds,
        }
    }

    /// Parse rate limit from string like "100/hour", "10/minute", "5/second"
    pub fn from_string(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 2 {
            return Err(format!(
                "Invalid rate limit format: {}. Expected 'count/period'",
                s
            ));
        }

        let count: u64 = parts[0]
            .parse()
            .map_err(|_| format!("Invalid count in rate limit: {}", parts[0]))?;

        let window_seconds = match parts[1].to_lowercase().as_str() {
            "second" | "sec" | "s" => 1,
            "minute" | "min" | "m" => 60,
            "hour" | "hr" | "h" => 3600,
            "day" | "d" => 86400,
            _ => {
                return Err(format!(
                "Invalid period in rate limit: {}. Expected 'second', 'minute', 'hour', or 'day'",
                parts[1]
            ))
            }
        };

        Ok(Self::new(count, window_seconds))
    }
}

/// Rate limiter tracks actions within a time window
#[derive(Debug, Clone)]
pub struct RateLimiter {
    limit: RateLimit,
    actions: Vec<SystemTime>, // Timestamps of actions
}

impl RateLimiter {
    pub fn new(limit: RateLimit) -> Self {
        Self {
            limit,
            actions: Vec::new(),
        }
    }

    /// Check if an action is allowed under the rate limit.
    ///
    /// `now` is passed in (rather than read from `SystemTime::now()`) so that
    /// callers in deterministic mode can supply a fixed clock via
    /// `PolicyEngine::get_time()`.
    pub fn check(&mut self, now: SystemTime) -> Result<(), String> {
        let window = Duration::from_secs(self.limit.window_seconds);

        // Remove actions outside the time window
        self.actions.retain(|&timestamp| {
            now.duration_since(timestamp)
                .map(|d| d < window)
                .unwrap_or(false)
        });

        // Check if limit exceeded
        if self.actions.len() >= self.limit.count as usize {
            let oldest = self
                .actions
                .first()
                .and_then(|&t| now.duration_since(t).ok())
                .unwrap_or(Duration::ZERO);
            let wait_seconds = self.limit.window_seconds.saturating_sub(oldest.as_secs());
            return Err(format!(
                "Rate limit exceeded: {} actions per {} seconds. Wait {} seconds before retrying.",
                self.limit.count, self.limit.window_seconds, wait_seconds
            ));
        }

        // Record action
        self.actions.push(now);
        Ok(())
    }

    /// Get remaining actions in the current window.
    ///
    /// `now` is passed in so callers in deterministic mode supply the fixed clock.
    pub fn remaining(&self, now: SystemTime) -> u64 {
        let window = Duration::from_secs(self.limit.window_seconds);

        let recent: usize = self
            .actions
            .iter()
            .filter(|&&timestamp| {
                now.duration_since(timestamp)
                    .map(|d| d < window)
                    .unwrap_or(false)
            })
            .count();

        self.limit.count.saturating_sub(recent as u64)
    }
}
