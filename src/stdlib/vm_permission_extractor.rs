use crate::stdlib::permission_checker::PermissionChecker;
use crate::runtime::permissions::PermissionResource;
use crate::runtime::errors::RuntimeError;

/// Helper to extract permission checking capability from VM
/// This allows stdlib functions to check permissions without borrow checker issues
pub trait VMPermissionExtractor {
    fn check_permission_safe(&self, resource: &PermissionResource, scope: Option<&str>) -> Result<(), RuntimeError>;
}

