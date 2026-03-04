use super::VirtualMachine;
use crate::policy::{PolicyEngine, Policy};
use crate::runtime::errors::RuntimeError;

/// Policy engine management methods for VirtualMachine
impl VirtualMachine {
    /// Get policy engine reference
    pub fn get_policy_engine(&self) -> &PolicyEngine {
        &self.policy_engine
    }

    /// Get policy engine mutable reference
    pub fn get_policy_engine_mut(&mut self) -> &mut PolicyEngine {
        &mut self.policy_engine
    }

    /// Set policy
    pub fn set_policy(&mut self, policy: Policy) {
        self.policy_engine.set_policy(policy);
    }

    /// Check rate limit for action/resource
    pub fn check_rate_limit(&mut self, action: &str) -> Result<(), RuntimeError> {
        self.policy_engine.check_rate_limit(action)
            .map_err(|e| self.create_error(format!("Policy error: {}", e)))
    }

    /// Check AI allowance
    pub fn check_ai_allowed(&self) -> Result<(), RuntimeError> {
        self.policy_engine.check_ai_allowed()
            .map_err(|e| self.create_error(format!("Policy error: {}", e)))
    }

    /// Check max execution time
    pub fn check_max_execution_time(&self) -> Result<(), RuntimeError> {
        self.policy_engine.check_max_execution_time()
            .map_err(|e| self.create_error(format!("Policy error: {}", e)))
    }

    /// Check if deterministic mode is enabled
    pub fn is_deterministic_mode(&self) -> bool {
        self.policy_engine.is_deterministic_mode()
    }

    /// Get current time (with deterministic override if enabled)
    pub fn get_time(&self) -> std::time::SystemTime {
        self.policy_engine.get_time()
    }
}

