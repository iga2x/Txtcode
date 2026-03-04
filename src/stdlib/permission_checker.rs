use crate::runtime::permissions::PermissionResource;
use crate::runtime::errors::RuntimeError;

/// Trait for checking permissions (used by stdlib functions)
pub trait PermissionChecker {
    fn check_permission(&self, resource: &PermissionResource, scope: Option<&str>) -> Result<(), RuntimeError>;
}

