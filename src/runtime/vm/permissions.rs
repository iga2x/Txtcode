use super::VirtualMachine;
use crate::runtime::audit::AuditResult;
use crate::runtime::errors::RuntimeError;
use crate::runtime::permissions::{Permission, PermissionManager, PermissionResource};

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

    /// Check permission and log to audit trail (mutable version)
    /// Also checks rate limits and policies
    /// Execution order: Intent → Capability → Permission
    pub fn check_permission_with_audit(
        &mut self,
        resource: &PermissionResource,
        scope: Option<&str>,
    ) -> Result<(), RuntimeError> {
        // Check max execution time
        self.check_max_execution_time()?;

        // Check AI allowance if AI metadata is present
        if !self.ai_metadata.is_empty() {
            self.check_ai_allowed()?;
        }

        // 1. CHECK INTENT FIRST (if function has intent declared)
        // Get current function name from call stack
        if let Some(current_frame) = self.call_stack.current_frame() {
            let function_name = &current_frame.function_name;
            let action = match resource {
                PermissionResource::FileSystem(action) => format!("fs.{}", action),
                PermissionResource::Network(action) => format!("net.{}", action),
                PermissionResource::System(action) => format!("sys.{}", action),
                PermissionResource::Process(_) => "process.exec".to_string(),
            };
            let resource_str = scope.unwrap_or("");

            // Check intent - if violated, error immediately
            if let Err(intent_err) = self.check_intent(function_name, &action, resource_str) {
                // Log intent violation to audit trail
                let _ = self.audit_trail.log_action(
                    format!("intent.violation.{}", action),
                    resource_str.to_string(),
                    Some(format!("intent:{}", function_name)),
                    crate::runtime::audit::AuditResult::Denied,
                    if self.ai_metadata.is_empty() {
                        None
                    } else {
                        Some(&self.ai_metadata)
                    },
                );
                return Err(intent_err);
            }
        }

        // 2. CHECK CAPABILITY TOKEN (if active in current scope)
        if let Some(token_id) = &self.active_capability {
            let action = self.get_action_from_resource(resource);
            match self
                .capability_manager
                .check(token_id, resource, &action, scope)
            {
                Ok(()) => {
                    // Explicit denies always win, even over a valid capability token.
                    // A `deny_permission()` call must not be bypassable by holding a token.
                    if let Err(deny_err) = self.permission_manager.check_denied(resource, scope) {
                        let _ = self.audit_trail.log_action(
                            format!("capability.denied.{}", action),
                            scope.unwrap_or("").to_string(),
                            Some(format!("capability:{}/deny-override", token_id)),
                            AuditResult::Denied,
                            if self.ai_metadata.is_empty() {
                                None
                            } else {
                                Some(&self.ai_metadata)
                            },
                        );
                        return Err(
                            self.create_error(format!("Permission error: {}", deny_err))
                        );
                    }
                    // Capability valid and no explicit deny — log and allow
                    let _ = self.audit_trail.log_action(
                        format!("capability.used.{}", action),
                        scope.unwrap_or("").to_string(),
                        Some(format!("capability:{}", token_id)),
                        AuditResult::Allowed,
                        if self.ai_metadata.is_empty() {
                            None
                        } else {
                            Some(&self.ai_metadata)
                        },
                    );
                    return Ok(());
                }
                Err(cap_err) => {
                    // Capability check failed - log and return error
                    let _ = self.audit_trail.log_action(
                        format!("capability.check.{}", action),
                        scope.unwrap_or("").to_string(),
                        Some(format!("capability:{}", token_id)),
                        AuditResult::Denied,
                        if self.ai_metadata.is_empty() {
                            None
                        } else {
                            Some(&self.ai_metadata)
                        },
                    );
                    return Err(self.create_error(format!("Capability error: {}", cap_err)));
                }
            }
        }

        // 3. Check rate limit for this action/resource
        let action_str = format!("permission.check.{}", resource);
        self.check_rate_limit(&action_str)?;

        // 4. Check permission
        let result = self.permission_manager.check(resource, scope);

        // Log permission check to audit trail
        let _ = self.audit_trail.log_permission_check(
            resource,
            scope,
            result.clone(),
            if self.ai_metadata.is_empty() {
                None
            } else {
                Some(&self.ai_metadata)
            },
        );

        result.map_err(|e| self.create_error(format!("Permission error: {}", e)))
    }

    /// Get permission manager reference
    pub fn get_permission_manager(&self) -> &PermissionManager {
        &self.permission_manager
    }

    /// Get action string from PermissionResource (helper for capabilities)
    pub(super) fn get_action_from_resource(&self, resource: &PermissionResource) -> String {
        match resource {
            PermissionResource::FileSystem(action) => action.clone(),
            PermissionResource::Network(action) => action.clone(),
            PermissionResource::System(action) => action.clone(),
            PermissionResource::Process(_) => "exec".to_string(),
        }
    }
}
