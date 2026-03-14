// Shared 6-layer security pipeline — used by both the AST VM and Bytecode VM.
//
// LAYER ORDER (never reorder — order matters for security):
//   1. Max execution time check (policy engine)
//   2. AI metadata allowance check (if AI metadata is present)
//   3. Intent check (declared intent vs. requested action)
//   4. Capability token check (deny-wins, rate-limit still applies on grant)
//   5. Rate limit check (applies even when capability grants access — Phase 2.4 fix)
//   6. Permission manager check (final allow/deny decision)
//
// Each VM passes its own typed components into the pipeline. The pipeline returns
// a `PipelineResult` that the VM converts to its own error type.
//
// FUTURE (Phase 5 consolidation): when the bytecode VM becomes the sole production VM,
// this module will be the only place the 6 layers are implemented.

use crate::runtime::permissions::PermissionResource;

/// Result of a single pipeline evaluation.
#[derive(Debug)]
pub enum PipelineResult {
    /// Access granted.
    Allowed,
    /// Access denied with a human-readable explanation.
    Denied(String),
}

impl PipelineResult {
    pub fn is_allowed(&self) -> bool {
        matches!(self, PipelineResult::Allowed)
    }

    /// Convert to a `Result<(), String>` for ergonomic use in callers.
    pub fn into_result(self) -> Result<(), String> {
        match self {
            PipelineResult::Allowed => Ok(()),
            PipelineResult::Denied(msg) => Err(msg),
        }
    }
}

/// Audit result type for pipeline logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineAuditResult {
    Allowed,
    Denied,
}

/// Trait that pipeline participants must implement to be checkable.
///
/// Both VMs implement this trait so the pipeline can call them uniformly.
/// Each method corresponds to one layer of the 6-layer pipeline.
pub trait SecurityPipelineContext {
    /// Layer 1: Check maximum execution time.
    fn check_max_execution_time(&mut self) -> Result<(), String>;

    /// Layer 2: Check whether AI metadata is present and allowed.
    fn check_ai_allowed(&mut self) -> Result<(), String>;

    /// Layer 3: Check declared intent for the given action on the resource.
    fn check_intent(
        &self,
        function_name: &str,
        action: &str,
        resource: &str,
    ) -> Result<(), String>;

    /// Layer 4: Check the active capability token.
    ///
    /// Returns `Some(Ok(()))` if a token exists and grants access,
    /// `Some(Err(msg))` if a token exists and denies access,
    /// `None` if no capability token is active.
    fn check_capability(
        &mut self,
        resource: &PermissionResource,
        action: &str,
        scope: Option<&str>,
    ) -> Option<Result<(), String>>;

    /// Layer 5: Check rate limit for the given action string.
    fn check_rate_limit(&mut self, action: &str) -> Result<(), String>;

    /// Layer 6: Check the permission manager.
    fn check_permission_manager(
        &self,
        resource: &PermissionResource,
        scope: Option<&str>,
    ) -> Result<(), String>;

    /// Name of the currently executing function (for intent check context).
    fn current_function_name(&self) -> Option<&str>;

    /// Whether AI metadata is present in the current context.
    fn has_ai_metadata(&self) -> bool;

    /// Log an audit event (best-effort; errors are silently ignored).
    fn log_audit(
        &mut self,
        action: &str,
        resource: &str,
        token: Option<&str>,
        result: PipelineAuditResult,
    );
}

/// Run the full 6-layer pipeline using the provided context.
///
/// Returns `PipelineResult::Allowed` or `PipelineResult::Denied(msg)`.
pub fn run_pipeline(
    ctx: &mut dyn SecurityPipelineContext,
    resource: &PermissionResource,
    scope: Option<&str>,
) -> PipelineResult {
    let action = action_from_resource(resource);
    let resource_str = scope.unwrap_or("");

    // Layer 1: Max execution time
    if let Err(e) = ctx.check_max_execution_time() {
        return PipelineResult::Denied(e);
    }

    // Layer 2: AI allowance
    if ctx.has_ai_metadata() {
        if let Err(e) = ctx.check_ai_allowed() {
            return PipelineResult::Denied(e);
        }
    }

    // Layer 3: Intent check
    if let Some(fn_name) = ctx.current_function_name() {
        let fn_name = fn_name.to_string(); // clone to avoid borrow conflict
        if let Err(e) = ctx.check_intent(&fn_name, &action, resource_str) {
            ctx.log_audit(
                &format!("intent.violation.{}", action),
                resource_str,
                Some(&format!("intent:{}", fn_name)),
                PipelineAuditResult::Denied,
            );
            return PipelineResult::Denied(format!("Intent violation: {}", e));
        }
    }

    // Layer 4: Capability token
    if let Some(cap_result) = ctx.check_capability(resource, &action, scope) {
        match cap_result {
            Ok(()) => {
                // Layer 5 (rate limit) still applies even when capability grants access (2.4 fix)
                if let Err(e) = ctx.check_rate_limit(&format!("capability.check.{}", action)) {
                    return PipelineResult::Denied(format!("Rate limit exceeded: {}", e));
                }
                ctx.log_audit(
                    &format!("capability.used.{}", action),
                    resource_str,
                    None,
                    PipelineAuditResult::Allowed,
                );
                return PipelineResult::Allowed;
            }
            Err(e) => {
                ctx.log_audit(
                    &format!("capability.check.{}", action),
                    resource_str,
                    None,
                    PipelineAuditResult::Denied,
                );
                return PipelineResult::Denied(format!("Capability denied: {}", e));
            }
        }
    }

    // Layer 5: Rate limit (standard path, no capability token)
    let rate_key = format!("permission.check.{}", resource);
    if let Err(e) = ctx.check_rate_limit(&rate_key) {
        return PipelineResult::Denied(format!("Rate limit exceeded: {}", e));
    }

    // Layer 6: Permission manager
    match ctx.check_permission_manager(resource, scope) {
        Ok(()) => {
            ctx.log_audit(
                &format!("permission.{}", action),
                resource_str,
                None,
                PipelineAuditResult::Allowed,
            );
            PipelineResult::Allowed
        }
        Err(e) => {
            ctx.log_audit(
                &format!("permission.{}", action),
                resource_str,
                None,
                PipelineAuditResult::Denied,
            );
            PipelineResult::Denied(format!("Permission denied: {}", e))
        }
    }
}

/// Derive a namespaced action string from a permission resource.
/// Matches the format used by capability tokens and intent declarations.
pub fn action_from_resource(resource: &PermissionResource) -> String {
    match resource {
        PermissionResource::FileSystem(action) => format!("fs.{}", action),
        PermissionResource::Network(action) => format!("net.{}", action),
        PermissionResource::System(action) => format!("sys.{}", action),
        PermissionResource::Process(_) => "process.exec".to_string(),
        PermissionResource::WiFi(action) => format!("wifi.{}", action),
        PermissionResource::Bluetooth(action) => format!("ble.{}", action),
    }
}
