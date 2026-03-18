use super::VirtualMachine;
use crate::capability::CapabilityManager;
use crate::runtime::audit::{AIMetadata, AuditResult};
use crate::runtime::errors::RuntimeError;
use crate::runtime::permissions::PermissionResource;

/// Capability management methods for VirtualMachine
impl VirtualMachine {
    /// Grant a capability token
    pub fn grant_capability(
        &mut self,
        resource: PermissionResource,
        action: String,
        scope: Option<String>,
        expires_in: Option<std::time::Duration>,
        granted_by: Option<String>,
        ai_metadata: Option<AIMetadata>,
    ) -> String {
        let ai_meta_for_log = ai_metadata.as_ref().filter(|meta| !meta.is_empty());

        let token_id = self.capability_manager.grant(
            resource,
            action.clone(),
            scope.clone(),
            expires_in,
            granted_by,
            ai_metadata.clone(),
        );

        // Log capability grant to audit trail
        let _ = self.audit_trail.log_action(
            format!("capability.granted.{}", action),
            scope.clone().unwrap_or("".to_string()),
            Some(format!("capability:{}", token_id)),
            AuditResult::Allowed,
            if let Some(meta) = ai_meta_for_log {
                Some(meta)
            } else if !self.ai_metadata.is_empty() {
                Some(&self.ai_metadata)
            } else {
                None
            },
        );

        token_id
    }

    /// Set active capability for current scope
    pub fn use_capability(&mut self, token_id: String) -> Result<(), RuntimeError> {
        // Validate capability exists and is valid; use detailed result for useful errors.
        let result = self.capability_manager.is_valid_detailed(&token_id);
        if result.is_granted() {
            self.active_capability = Some(token_id);
            Ok(())
        } else {
            let reason = result.denial_reason().unwrap_or_else(|| "invalid token".to_string());
            Err(self.create_error(format!("Capability denied: {}", reason)))
        }
    }

    /// Clear active capability
    pub fn clear_capability(&mut self) {
        self.active_capability = None;
    }

    /// Get active capability
    pub fn get_active_capability(&self) -> Option<&String> {
        self.active_capability.as_ref()
    }

    /// Revoke a capability token
    pub fn revoke_capability(
        &mut self,
        token_id: &str,
        reason: Option<String>,
    ) -> Result<(), RuntimeError> {
        self.capability_manager
            .revoke(token_id, reason.clone())
            .map_err(|e| self.create_error(format!("Capability revocation error: {}", e)))?;

        // If revoked capability is active, clear it
        if self.active_capability.as_ref() == Some(&token_id.to_string()) {
            self.active_capability = None;
        }

        // Log revocation to audit trail
        let _ = self.audit_trail.log_action(
            "capability.revoked".to_string(),
            token_id.to_string(),
            Some("capability".to_string()),
            AuditResult::Denied,
            if self.ai_metadata.is_empty() {
                None
            } else {
                Some(&self.ai_metadata)
            },
        );

        Ok(())
    }

    /// Get capability manager reference
    pub fn get_capability_manager(&self) -> &CapabilityManager {
        &self.capability_manager
    }

    /// Check if a capability token is valid
    pub fn capability_valid(&self, token_id: &str) -> bool {
        self.capability_manager.is_valid(token_id)
    }
}
