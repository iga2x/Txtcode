// Scope policy - controls resource access scope

use crate::runtime::permissions::glob_scope_matches;
use std::collections::HashSet;

/// Scope policy configuration.
///
/// NOTE: `ScopePolicy` is not wired to `PolicyEngine` or the VM's
/// `PermissionManager`. It is available for direct use in library consumers,
/// but the VM enforces scopes through `PermissionManager::scope_matches`.
/// Connecting the two is tracked as a future architectural consolidation task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopePolicy {
    pub allowed_scopes: HashSet<String>, // Allowed resource scopes (e.g., "/tmp/*", "example.com")
    pub denied_scopes: HashSet<String>,  // Denied resource scopes
}

impl ScopePolicy {
    pub fn new() -> Self {
        Self {
            allowed_scopes: HashSet::new(),
            denied_scopes: HashSet::new(),
        }
    }

    pub fn allow_scope(mut self, scope: String) -> Self {
        self.allowed_scopes.insert(scope);
        self
    }

    pub fn deny_scope(mut self, scope: String) -> Self {
        self.denied_scopes.insert(scope);
        self
    }

    /// Check if scope is allowed
    pub fn check(&self, scope: &str) -> Result<(), String> {
        // Check denied first
        for denied in &self.denied_scopes {
            if Self::matches(denied, scope) {
                return Err(format!("Scope denied: {}", scope));
            }
        }

        // If allowed_scopes is empty, all scopes are allowed (unless denied)
        if !self.allowed_scopes.is_empty() {
            let mut matched = false;
            for allowed in &self.allowed_scopes {
                if Self::matches(allowed, scope) {
                    matched = true;
                    break;
                }
            }
            if !matched {
                return Err(format!("Scope not allowed: {}", scope));
            }
        }

        Ok(())
    }

    fn matches(pattern: &str, value: &str) -> bool {
        glob_scope_matches(pattern, value)
    }
}

impl Default for ScopePolicy {
    fn default() -> Self {
        Self::new()
    }
}
