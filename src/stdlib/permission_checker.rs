use crate::runtime::errors::RuntimeError;
use crate::runtime::permissions::PermissionResource;

/// Trait for checking permissions (used by stdlib functions)
pub trait PermissionChecker {
    fn check_permission(
        &self,
        resource: &PermissionResource,
        scope: Option<&str>,
    ) -> Result<(), RuntimeError>;
}
