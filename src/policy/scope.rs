// Scope policy - controls resource access scope

use std::collections::HashSet;

/// Scope policy configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopePolicy {
    pub allowed_scopes: HashSet<String>,  // Allowed resource scopes (e.g., "/tmp/*", "example.com")
    pub denied_scopes: HashSet<String>,   // Denied resource scopes
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

    /// Simple glob pattern matching
    fn matches(pattern: &str, value: &str) -> bool {
        if pattern.contains('*') {
            if pattern.ends_with('*') {
                value.starts_with(&pattern[..pattern.len() - 1])
            } else if pattern.starts_with('*') {
                value.ends_with(&pattern[1..])
            } else {
                // Contains * in middle
                let parts: Vec<&str> = pattern.split('*').collect();
                if parts.len() == 2 {
                    value.starts_with(parts[0]) && value.ends_with(parts[1])
                } else {
                    pattern == value
                }
            }
        } else {
            pattern == value
        }
    }
}

impl Default for ScopePolicy {
    fn default() -> Self {
        Self::new()
    }
}

