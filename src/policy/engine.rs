// Policy engine - enforces policies at runtime
// Pre-execution policy checks (rate limiting, timeouts, AI control)

use super::rate_limit::{RateLimit, RateLimiter};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// Policy configuration
#[derive(Debug, Clone)]
pub struct Policy {
    pub rate_limits: HashMap<String, RateLimit>, // Action/resource -> rate limit
    pub ai_allowed: bool,                        // Whether AI execution is allowed
    pub deterministic_mode: bool,                // Whether deterministic execution is enabled
    pub max_execution_time: Option<Duration>,    // Maximum execution time
}

impl Policy {
    pub fn new() -> Self {
        Self {
            rate_limits: HashMap::new(),
            ai_allowed: true,          // Default: allow AI execution
            deterministic_mode: false, // Default: non-deterministic
            max_execution_time: None,  // Default: no limit
        }
    }

    /// Set rate limit for action/resource
    pub fn set_rate_limit(&mut self, action: String, limit: RateLimit) {
        self.rate_limits.insert(action, limit);
    }

    /// Get rate limit for action/resource
    pub fn get_rate_limit(&self, action: &str) -> Option<&RateLimit> {
        self.rate_limits.get(action)
    }

    /// Set AI allowance
    pub fn set_ai_allowed(&mut self, allowed: bool) {
        self.ai_allowed = allowed;
    }

    /// Set deterministic mode
    pub fn set_deterministic_mode(&mut self, enabled: bool) {
        self.deterministic_mode = enabled;
    }

    /// Set max execution time
    pub fn set_max_execution_time(&mut self, duration: Option<Duration>) {
        self.max_execution_time = duration;
    }
}

impl Default for Policy {
    fn default() -> Self {
        Self::new()
    }
}

/// Policy engine enforces policies at runtime
pub struct PolicyEngine {
    policy: Policy,
    rate_limiters: HashMap<String, RateLimiter>, // Action/resource -> rate limiter
    start_time: Option<SystemTime>,              // Execution start time (for max execution time)
    deterministic_overrides: DeterministicOverrides, // Overrides for deterministic mode
}

/// Overrides for deterministic execution mode
#[derive(Debug, Clone)]
pub struct DeterministicOverrides {
    pub random_seed: Option<u64>, // Fixed seed for random number generation
    pub time_override: Option<SystemTime>, // Fixed time for timestamp operations
    pub network_mock: bool,       // Mock network calls
}

impl DeterministicOverrides {
    pub fn new() -> Self {
        Self {
            random_seed: None,
            time_override: None,
            network_mock: false,
        }
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.random_seed = Some(seed);
        self
    }

    pub fn with_time(mut self, time: SystemTime) -> Self {
        self.time_override = Some(time);
        self
    }

    pub fn with_network_mock(mut self, mock: bool) -> Self {
        self.network_mock = mock;
        self
    }
}

impl Default for DeterministicOverrides {
    fn default() -> Self {
        Self::new()
    }
}

impl PolicyEngine {
    pub fn new() -> Self {
        Self {
            policy: Policy::new(),
            rate_limiters: HashMap::new(),
            start_time: None,
            deterministic_overrides: DeterministicOverrides::new(),
        }
    }

    /// Create with policy
    pub fn with_policy(policy: Policy) -> Self {
        let mut engine = Self {
            policy,
            rate_limiters: HashMap::new(),
            start_time: None,
            deterministic_overrides: DeterministicOverrides::new(),
        };

        // Initialize rate limiters for all rate limits
        for (action, limit) in &engine.policy.rate_limits {
            engine
                .rate_limiters
                .insert(action.clone(), RateLimiter::new(limit.clone()));
        }

        engine
    }

    /// Set policy
    pub fn set_policy(&mut self, policy: Policy) {
        self.policy = policy;
        // Reinitialize rate limiters
        self.rate_limiters.clear();
        for (action, limit) in &self.policy.rate_limits {
            self.rate_limiters
                .insert(action.clone(), RateLimiter::new(limit.clone()));
        }
    }

    /// Get policy
    pub fn get_policy(&self) -> &Policy {
        &self.policy
    }

    /// Check rate limit for action/resource.
    ///
    /// Uses `self.get_time()` so deterministic-mode time overrides are respected.
    pub fn check_rate_limit(&mut self, action: &str) -> Result<(), PolicyError> {
        // Capture the policy clock before the mutable borrow on rate_limiters.
        let now = self.get_time();

        // Get or create rate limiter for this action
        let limiter = self
            .rate_limiters
            .entry(action.to_string())
            .or_insert_with(|| {
                let limit = self
                    .policy
                    .get_rate_limit(action)
                    .cloned()
                    .unwrap_or_else(|| RateLimit::new(1000, 3600)); // Default: 1000/hour
                RateLimiter::new(limit)
            });

        match limiter.check(now) {
            Ok(()) => Ok(()),
            Err(_msg) => Err(PolicyError::RateLimitExceeded {
                action: action.to_string(),
                limit: self
                    .policy
                    .get_rate_limit(action)
                    .cloned()
                    .unwrap_or_else(|| RateLimit::new(1000, 3600)),
                wait_seconds: 0, // Returns 0 — precise wait time extraction deferred to v0.5
            }),
        }
    }

    /// Check AI allowance
    pub fn check_ai_allowed(&self) -> Result<(), PolicyError> {
        if !self.policy.ai_allowed {
            Err(PolicyError::AIExecutionNotAllowed)
        } else {
            Ok(())
        }
    }

    /// Check max execution time
    pub fn check_max_execution_time(&self) -> Result<(), PolicyError> {
        if let Some(max_time) = self.policy.max_execution_time {
            if let Some(start) = self.start_time {
                let elapsed = start.elapsed().unwrap_or(Duration::ZERO);
                if elapsed > max_time {
                    return Err(PolicyError::MaxExecutionTimeExceeded {
                        max_time: max_time.as_secs(),
                        elapsed: elapsed.as_secs(),
                    });
                }
            }
        }
        Ok(())
    }

    /// Start execution timer (for max execution time checking)
    pub fn start_execution(&mut self) {
        self.start_time = Some(SystemTime::now());
    }

    /// Check if deterministic mode is enabled
    pub fn is_deterministic_mode(&self) -> bool {
        self.policy.deterministic_mode
    }

    /// Get deterministic overrides
    pub fn get_deterministic_overrides(&self) -> &DeterministicOverrides {
        &self.deterministic_overrides
    }

    /// Set deterministic overrides
    pub fn set_deterministic_overrides(&mut self, overrides: DeterministicOverrides) {
        self.deterministic_overrides = overrides;
    }

    /// Get current time (with deterministic override if enabled)
    pub fn get_time(&self) -> SystemTime {
        if self.policy.deterministic_mode {
            self.deterministic_overrides
                .time_override
                .unwrap_or_else(SystemTime::now)
        } else {
            SystemTime::now()
        }
    }

    /// Check if network should be mocked (deterministic mode)
    pub fn should_mock_network(&self) -> bool {
        self.policy.deterministic_mode && self.deterministic_overrides.network_mock
    }

    /// Get random seed (deterministic mode)
    pub fn get_random_seed(&self) -> Option<u64> {
        if self.policy.deterministic_mode {
            self.deterministic_overrides.random_seed
        } else {
            None
        }
    }

    /// Get remaining actions for rate limit.
    ///
    /// Uses `self.get_time()` so deterministic-mode time overrides are respected.
    pub fn get_rate_limit_remaining(&self, action: &str) -> Option<u64> {
        let now = self.get_time();
        self.rate_limiters
            .get(action)
            .map(|limiter| limiter.remaining(now))
    }

    /// Reset rate limiters (useful for testing)
    pub fn reset_rate_limiters(&mut self) {
        self.rate_limiters.clear();
        // Reinitialize rate limiters
        for (action, limit) in &self.policy.rate_limits {
            self.rate_limiters
                .insert(action.clone(), RateLimiter::new(limit.clone()));
        }
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Policy enforcement error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyError {
    RateLimitExceeded {
        action: String,
        limit: RateLimit,
        wait_seconds: u64,
    },
    AIExecutionNotAllowed,
    MaxExecutionTimeExceeded {
        max_time: u64,
        elapsed: u64,
    },
    DeterministicModeViolation {
        violation: String,
    },
}

impl std::fmt::Display for PolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PolicyError::RateLimitExceeded {
                action,
                limit,
                wait_seconds,
            } => {
                write!(f, "Rate limit exceeded for '{}': {} actions per {} seconds. Wait {} seconds before retrying.", 
                    action, limit.count, limit.window_seconds, wait_seconds)
            }
            PolicyError::AIExecutionNotAllowed => {
                write!(f, "AI execution is not allowed by policy")
            }
            PolicyError::MaxExecutionTimeExceeded { max_time, elapsed } => {
                write!(
                    f,
                    "Maximum execution time exceeded: {} seconds (max: {} seconds)",
                    elapsed, max_time
                )
            }
            PolicyError::DeterministicModeViolation { violation } => {
                write!(f, "Deterministic mode violation: {}", violation)
            }
        }
    }
}

impl std::error::Error for PolicyError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_engine_ai_allowed() {
        let mut engine = PolicyEngine::new();

        // Default: AI allowed
        assert!(engine.check_ai_allowed().is_ok());

        // Disable AI
        engine.policy.set_ai_allowed(false);
        assert!(engine.check_ai_allowed().is_err());
    }

    #[test]
    fn test_policy_engine_deterministic_mode() {
        let mut engine = PolicyEngine::new();

        // Default: non-deterministic
        assert!(!engine.is_deterministic_mode());

        // Enable deterministic mode
        engine.policy.set_deterministic_mode(true);
        assert!(engine.is_deterministic_mode());
    }

    #[test]
    fn test_policy_engine_rate_limit() {
        let mut policy = Policy::new();
        policy.set_rate_limit("fs.read".to_string(), RateLimit::new(2, 60));

        let mut engine = PolicyEngine::with_policy(policy);

        // First two should succeed
        assert!(engine.check_rate_limit("fs.read").is_ok());
        assert!(engine.check_rate_limit("fs.read").is_ok());

        // Third should fail
        assert!(engine.check_rate_limit("fs.read").is_err());
    }
}
