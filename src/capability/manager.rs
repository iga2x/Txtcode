// Capability manager - grants, validates, and revokes capability tokens
// Runtime capability token management

use crate::runtime::audit::AIMetadata;
use crate::runtime::permissions::{glob_scope_matches, PermissionResource};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// Capability token - runtime, transferable, revocable permission
#[derive(Debug, Clone, PartialEq)]
pub struct Capability {
    pub id: String,                      // Unique token ID
    pub resource: PermissionResource,    // Resource type (fs, net, sys)
    pub action: String,                  // Action (read, write, connect)
    pub scope: Option<String>,           // Scope restriction (e.g., "/tmp/*")
    pub created_at: SystemTime,          // Creation timestamp
    pub expires_at: Option<SystemTime>,  // Expiration (None = no expiration)
    pub revoked: bool,                   // Revocation flag
    pub granted_by: Option<String>,      // Function/agent that granted this
    pub ai_metadata: Option<AIMetadata>, // AI context if granted by AI
}

/// Capability event for audit logging
#[derive(Debug, Clone)]
pub enum CapabilityEvent {
    Granted {
        token_id: String,
        resource: String,
        action: String,
        scope: Option<String>,
        expires_in: Option<Duration>,
    },
    Used {
        token_id: String,
        resource: String,
        action: String,
        success: bool,
    },
    Revoked {
        token_id: String,
        reason: Option<String>,
    },
    Expired {
        token_id: String,
    },
}

/// Typed result of a capability validity check.
/// Replaces raw `bool` returns so callers can emit actionable error messages.
#[derive(Debug, Clone, PartialEq)]
pub enum CapabilityResult {
    /// Token is present and has not been revoked or expired.
    Granted,
    /// Token ID was not found in the manager.
    NotFound,
    /// Token exists but was explicitly revoked.
    Revoked { token_id: String },
    /// Token exists but its TTL has elapsed.
    Expired { token_id: String },
}

impl CapabilityResult {
    /// Returns `true` only for `Granted`.
    pub fn is_granted(&self) -> bool {
        matches!(self, CapabilityResult::Granted)
    }

    /// Returns a human-readable denial reason, or `None` if granted.
    pub fn denial_reason(&self) -> Option<String> {
        match self {
            CapabilityResult::Granted => None,
            CapabilityResult::NotFound => Some("capability token not found".to_string()),
            CapabilityResult::Revoked { token_id } => {
                Some(format!("capability token '{}' has been revoked", token_id))
            }
            CapabilityResult::Expired { token_id } => {
                Some(format!("capability token '{}' has expired", token_id))
            }
        }
    }
}

/// Capability manager - grants, validates, and revokes capability tokens
#[derive(Clone)]
pub struct CapabilityManager {
    tokens: HashMap<String, Capability>,
}

impl CapabilityManager {
    pub fn new() -> Self {
        Self {
            tokens: HashMap::new(),
        }
    }

    /// Grant a new capability token
    pub fn grant(
        &mut self,
        resource: PermissionResource,
        action: String,
        scope: Option<String>,
        expires_in: Option<Duration>,
        granted_by: Option<String>,
        ai_metadata: Option<AIMetadata>,
    ) -> String {
        let resource_str = resource.to_string();
        let token_id = self.generate_token_id(&resource, &action);
        let expires_at = expires_in.map(|d| SystemTime::now() + d);

        let capability = Capability {
            id: token_id.clone(),
            resource,
            action: action.clone(),
            scope: scope.clone(),
            created_at: SystemTime::now(),
            expires_at,
            revoked: false,
            granted_by,
            ai_metadata,
        };

        self.tokens.insert(token_id.clone(), capability);

        use crate::tools::logger::log_debug;
        log_debug(&format!(
            "Granted capability token: {} for {}.{}",
            token_id, resource_str, action
        ));

        token_id
    }

    /// Check if capability token is valid and matches request
    pub fn check(
        &mut self,
        token_id: &str,
        resource: &PermissionResource,
        action: &str,
        scope: Option<&str>,
    ) -> Result<(), CapabilityError> {
        // 1. Find token and extract values (to avoid borrow checker issues)
        let (cap_resource, cap_action, cap_scope, is_revoked, expires_at) = {
            let capability = self
                .tokens
                .get(token_id)
                .ok_or_else(|| CapabilityError::NotFound(token_id.to_string()))?;

            (
                capability.resource.clone(),
                capability.action.clone(),
                capability.scope.clone(),
                capability.revoked,
                capability.expires_at,
            )
        };

        // 2. Check revocation
        if is_revoked {
            return Err(CapabilityError::Revoked(token_id.to_string()));
        }

        // 3. Check expiration
        if let Some(expires_at) = expires_at {
            if SystemTime::now() > expires_at {
                // Auto-revoke expired tokens
                if let Some(capability) = self.tokens.get_mut(token_id) {
                    capability.revoked = true;
                }
                return Err(CapabilityError::Expired(token_id.to_string()));
            }
        }

        // 4. Check resource/action match
        if !Self::matches_resource_static(&cap_resource, resource) {
            return Err(CapabilityError::Mismatch {
                token_id: token_id.to_string(),
                expected: format!("{}.{}", resource, action),
                got: format!("{}.{}", cap_resource, cap_action),
            });
        }

        if cap_action != action {
            return Err(CapabilityError::Mismatch {
                token_id: token_id.to_string(),
                expected: format!("{}.{}", resource, action),
                got: format!("{}.{}", cap_resource, cap_action),
            });
        }

        // 5. Check scope (if specified)
        if let Some(cap_scope) = cap_scope {
            if let Some(req_scope) = scope {
                if !Self::scope_matches_static(&cap_scope, req_scope) {
                    return Err(CapabilityError::ScopeMismatch {
                        token_id: token_id.to_string(),
                        expected: cap_scope.clone(),
                        got: req_scope.to_string(),
                    });
                }
            } else {
                return Err(CapabilityError::ScopeRequired {
                    token_id: token_id.to_string(),
                    required: cap_scope.clone(),
                });
            }
        }

        // Token is valid
        Ok(())
    }

    /// Revoke a capability token
    pub fn revoke(
        &mut self,
        token_id: &str,
        reason: Option<String>,
    ) -> Result<(), CapabilityError> {
        let capability = self
            .tokens
            .get_mut(token_id)
            .ok_or_else(|| CapabilityError::NotFound(token_id.to_string()))?;

        capability.revoked = true;

        use crate::tools::logger::log_debug;
        log_debug(&format!(
            "Revoked capability token: {} (reason: {:?})",
            token_id, reason
        ));

        Ok(())
    }

    /// Get capability by ID
    pub fn get(&self, token_id: &str) -> Option<&Capability> {
        self.tokens.get(token_id)
    }

    /// Check if capability is valid (not revoked, not expired).
    /// Returns a typed `CapabilityResult` with a denial reason when invalid.
    pub fn is_valid_detailed(&self, token_id: &str) -> CapabilityResult {
        match self.tokens.get(token_id) {
            None => CapabilityResult::NotFound,
            Some(cap) if cap.revoked => CapabilityResult::Revoked { token_id: token_id.to_string() },
            Some(cap) => {
                if let Some(expires_at) = cap.expires_at {
                    if SystemTime::now() > expires_at {
                        return CapabilityResult::Expired { token_id: token_id.to_string() };
                    }
                }
                CapabilityResult::Granted
            }
        }
    }

    /// Check if capability is valid (not revoked, not expired).
    /// Prefer `is_valid_detailed` for richer error reporting.
    pub fn is_valid(&self, token_id: &str) -> bool {
        self.is_valid_detailed(token_id).is_granted()
    }

    /// Generate unique token ID
    fn generate_token_id(&self, resource: &PermissionResource, action: &str) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        resource.to_string().hash(&mut hasher);
        action.hash(&mut hasher);
        SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .hash(&mut hasher);
        let hash = hasher.finish();

        format!("cap_{:016x}", hash)
    }

    /// Check if two resources match (static method to avoid borrow issues)
    fn matches_resource_static(a: &PermissionResource, b: &PermissionResource) -> bool {
        matches!(
            (a, b),
            (
                PermissionResource::FileSystem(_),
                PermissionResource::FileSystem(_)
            ) | (
                PermissionResource::Network(_),
                PermissionResource::Network(_)
            ) | (PermissionResource::System(_), PermissionResource::System(_))
                | (
                    PermissionResource::Process(_),
                    PermissionResource::Process(_)
                )
        )
    }

    fn scope_matches_static(pattern: &str, value: &str) -> bool {
        glob_scope_matches(pattern, value)
    }
}

/// Capability error
#[derive(Debug, Clone)]
pub enum CapabilityError {
    NotFound(String),
    Revoked(String),
    Expired(String),
    Mismatch {
        token_id: String,
        expected: String,
        got: String,
    },
    ScopeMismatch {
        token_id: String,
        expected: String,
        got: String,
    },
    ScopeRequired {
        token_id: String,
        required: String,
    },
}

impl std::fmt::Display for CapabilityError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CapabilityError::NotFound(id) => write!(f, "Capability token not found: {}", id),
            CapabilityError::Revoked(id) => write!(f, "Capability token revoked: {}", id),
            CapabilityError::Expired(id) => write!(f, "Capability token expired: {}", id),
            CapabilityError::Mismatch {
                token_id,
                expected,
                got,
            } => {
                write!(
                    f,
                    "Capability token mismatch: {} (expected: {}, got: {})",
                    token_id, expected, got
                )
            }
            CapabilityError::ScopeMismatch {
                token_id,
                expected,
                got,
            } => {
                write!(
                    f,
                    "Capability token scope mismatch: {} (expected: {}, got: {})",
                    token_id, expected, got
                )
            }
            CapabilityError::ScopeRequired { token_id, required } => {
                write!(
                    f,
                    "Capability token requires scope: {} (required: {})",
                    token_id, required
                )
            }
        }
    }
}

impl std::error::Error for CapabilityError {}

impl Default for CapabilityManager {
    fn default() -> Self {
        Self::new()
    }
}
