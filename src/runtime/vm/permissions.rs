use super::VirtualMachine;
use crate::runtime::audit::AuditResult;
use crate::runtime::errors::RuntimeError;
use crate::runtime::permissions::{Permission, PermissionManager, PermissionResource};
use crate::runtime::security_pipeline::{
    self, PipelineAuditResult, SecurityPipelineContext,
};

/// Permission management methods for VirtualMachine
impl VirtualMachine {
    /// Grant a permission
    pub fn grant_permission(&mut self, resource: PermissionResource, scope: Option<String>) {
        self.permission_manager
            .grant(Permission::new(resource, scope));
    }

    /// Deny a permission
    pub fn deny_permission(&mut self, resource: PermissionResource, scope: Option<String>) {
        self.permission_manager
            .deny(Permission::new(resource, scope));
    }

    /// Check if a permission is granted (immutable for trait compatibility)
    pub fn check_permission(
        &self,
        resource: &PermissionResource,
        scope: Option<&str>,
    ) -> Result<(), RuntimeError> {
        self.permission_manager
            .check(resource, scope)
            .map_err(|e| self.create_error(format!("Permission error: {}", e)))
    }

    /// Check permission and log to audit trail (mutable version).
    /// Delegates to the shared 6-layer `run_pipeline()` in `security_pipeline.rs`.
    pub fn check_permission_with_audit(
        &mut self,
        resource: &PermissionResource,
        scope: Option<&str>,
    ) -> Result<(), RuntimeError> {
        security_pipeline::run_pipeline(self, resource, scope)
            .into_result()
            .map_err(|e| self.create_error(e))
    }

    /// Get permission manager reference
    pub fn get_permission_manager(&self) -> &PermissionManager {
        &self.permission_manager
    }
}

// ── SecurityPipelineContext impl ─────────────────────────────────────────────

impl SecurityPipelineContext for VirtualMachine {
    fn check_max_execution_time(&mut self) -> Result<(), String> {
        self.policy_engine
            .check_max_execution_time()
            .map_err(|e| format!("Policy error: {}", e))
    }

    fn check_ai_allowed(&mut self) -> Result<(), String> {
        self.policy_engine
            .check_ai_allowed()
            .map_err(|e| format!("Policy error: {}", e))
    }

    fn check_intent(&self, function_name: &str, action: &str, resource: &str) -> Result<(), String> {
        self.intent_checker
            .check_action(function_name, action, resource)
            .map_err(|e| e.to_string())
    }

    /// Handles deny-wins, rate-limit (Phase 2.4), and audit logging for capability checks.
    fn check_capability(
        &mut self,
        resource: &PermissionResource,
        action: &str,
        scope: Option<&str>,
    ) -> Option<Result<(), String>> {
        let token_id = self.active_capability.clone()?;

        match self.capability_manager.check(&token_id, resource, action, scope) {
            Ok(()) => {
                // Explicit denies always win, even over a valid capability token.
                if let Err(deny_err) = self.permission_manager.check_denied(resource, scope) {
                    let _ = self.audit_trail.log_action(
                        format!("capability.denied.{}", action),
                        scope.unwrap_or("").to_string(),
                        Some(format!("capability:{}/deny-override", token_id)),
                        AuditResult::Denied,
                        if self.ai_metadata.is_empty() { None } else { Some(&self.ai_metadata) },
                    );
                    return Some(Err(format!("Permission error: {}", deny_err)));
                }
                // Rate limit still applies even when capability grants access (Phase 2.4).
                if let Err(e) = self.policy_engine
                    .check_rate_limit(&format!("capability.check.{}", action))
                {
                    return Some(Err(format!("Policy error: {}", e)));
                }
                let _ = self.audit_trail.log_action(
                    format!("capability.used.{}", action),
                    scope.unwrap_or("").to_string(),
                    Some(format!("capability:{}", token_id)),
                    AuditResult::Allowed,
                    if self.ai_metadata.is_empty() { None } else { Some(&self.ai_metadata) },
                );
                Some(Ok(()))
            }
            Err(cap_err) => {
                let _ = self.audit_trail.log_action(
                    format!("capability.check.{}", action),
                    scope.unwrap_or("").to_string(),
                    Some(format!("capability:{}", token_id)),
                    AuditResult::Denied,
                    if self.ai_metadata.is_empty() { None } else { Some(&self.ai_metadata) },
                );
                Some(Err(format!("Capability error: {}", cap_err)))
            }
        }
    }

    fn check_rate_limit(&mut self, action: &str) -> Result<(), String> {
        self.policy_engine
            .check_rate_limit(action)
            .map_err(|e| format!("Policy error: {}", e))
    }

    fn check_permission_manager(
        &mut self,
        resource: &PermissionResource,
        scope: Option<&str>,
    ) -> Result<(), String> {
        let result = self.permission_manager.check(resource, scope);
        let _ = self.audit_trail.log_permission_check(
            resource,
            scope,
            result.clone(),
            if self.ai_metadata.is_empty() { None } else { Some(&self.ai_metadata) },
        );
        result.map_err(|e| format!("Permission error: {}", e))
    }

    fn current_function_name(&self) -> Option<&str> {
        self.call_stack.current_frame().map(|f| f.function_name.as_str())
    }

    fn has_ai_metadata(&self) -> bool {
        !self.ai_metadata.is_empty()
    }

    fn log_audit(
        &mut self,
        action: &str,
        resource: &str,
        token: Option<&str>,
        result: PipelineAuditResult,
    ) {
        let audit_result = match result {
            PipelineAuditResult::Allowed => AuditResult::Allowed,
            PipelineAuditResult::Denied => AuditResult::Denied,
        };
        let _ = self.audit_trail.log_action(
            action.to_string(),
            resource.to_string(),
            token.map(|s| s.to_string()),
            audit_result,
            if self.ai_metadata.is_empty() { None } else { Some(&self.ai_metadata) },
        );
    }
}
