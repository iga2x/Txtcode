/// Permission resource type (e.g., "fs.read", "net.connect")
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PermissionResource {
    FileSystem(String),      // "read", "write", "delete"
    Network(String),         // "connect", "listen"
    Process(Vec<String>),    // Whitelist of allowed commands
    System(String),          // "exec", "env"
}

impl PermissionResource {
    /// Create from string like "fs.read" or "net.connect"
    pub fn from_string(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 2 {
            return Err(format!("Invalid permission format: {}. Expected 'resource.action'", s));
        }
        
        match parts[0] {
            "fs" => Ok(PermissionResource::FileSystem(parts[1].to_string())),
            "net" => Ok(PermissionResource::Network(parts[1].to_string())),
            "sys" => Ok(PermissionResource::System(parts[1].to_string())),
            _ => Err(format!("Unknown resource type: {}", parts[0])),
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            PermissionResource::FileSystem(action) => format!("fs.{}", action),
            PermissionResource::Network(action) => format!("net.{}", action),
            PermissionResource::System(action) => format!("sys.{}", action),
            PermissionResource::Process(cmds) => format!("process:[{}]", cmds.join(",")),
        }
    }
}

/// Permission with optional scope restriction
#[derive(Debug, Clone, PartialEq)]
pub struct Permission {
    pub resource: PermissionResource,
    pub scope: Option<String>,  // e.g., "/tmp/*", "*.example.com"
}

impl Permission {
    pub fn new(resource: PermissionResource, scope: Option<String>) -> Self {
        Self { resource, scope }
    }
}

/// Permission manager - grants, denies, and checks permissions
pub struct PermissionManager {
    granted: Vec<Permission>,
    denied: Vec<Permission>,  // Explicit denials override grants
}

impl PermissionManager {
    pub fn new() -> Self {
        Self {
            granted: Vec::new(),
            denied: Vec::new(),
        }
    }

    /// Grant a permission
    pub fn grant(&mut self, permission: Permission) {
        self.granted.push(permission);
    }

    /// Explicitly deny a permission (overrides grants)
    pub fn deny(&mut self, permission: Permission) {
        self.denied.push(permission);
    }

    /// Check if a permission is granted
    pub fn check(&self, resource: &PermissionResource, scope: Option<&str>) -> Result<(), PermissionError> {
        // 1. Check explicit denials first (denials override grants)
        for denied in &self.denied {
            if self.matches(&denied.resource, resource) && self.scope_matches(&denied.scope, scope) {
                return Err(PermissionError::Denied(format!(
                    "Permission explicitly denied: {}",
                    resource.to_string()
                )));
            }
        }

        // 2. Check grants
        for granted in &self.granted {
            if self.matches(&granted.resource, resource) && self.scope_matches(&granted.scope, scope) {
                return Ok(());
            }
        }

        Err(PermissionError::NotGranted(resource.to_string()))
    }

    /// Check if two resources match
    fn matches(&self, perm_resource: &PermissionResource, check_resource: &PermissionResource) -> bool {
        match (perm_resource, check_resource) {
            (PermissionResource::FileSystem(a), PermissionResource::FileSystem(b)) => a == b,
            (PermissionResource::Network(a), PermissionResource::Network(b)) => a == b,
            (PermissionResource::System(a), PermissionResource::System(b)) => a == b,
            (PermissionResource::Process(allowed), PermissionResource::Process(cmd)) => {
                // Check if all commands in cmd are in allowed list
                cmd.iter().all(|c| allowed.contains(c))
            }
            _ => false,
        }
    }

    /// Check if scope matches (supports glob patterns)
    fn scope_matches(&self, perm_scope: &Option<String>, check_scope: Option<&str>) -> bool {
        match (perm_scope, check_scope) {
            (None, _) => true,  // No scope restriction means match all
            (Some(ps), Some(cs)) => {
                // Simple glob matching: "*.example.com", "/tmp/*"
                if ps.contains('*') {
                    // Simple matching (for production, use proper regex crate)
                    if ps.ends_with("*") {
                        cs.starts_with(&ps[..ps.len() - 1])
                    } else if ps.starts_with("*") {
                        cs.ends_with(&ps[1..])
                    } else {
                        // Contains * in middle - simple substring check
                        let parts: Vec<&str> = ps.split('*').collect();
                        if parts.len() == 2 {
                            cs.starts_with(parts[0]) && cs.ends_with(parts[1])
                        } else {
                            ps == cs  // Fallback to exact match
                        }
                    }
                } else {
                    ps == cs  // Exact match
                }
            }
            (Some(_), None) => false,  // Permission requires scope but none provided
        }
    }

    /// Get all granted permissions (for debugging/audit)
    pub fn get_granted(&self) -> &[Permission] {
        &self.granted
    }

    /// Get all denied permissions (for debugging/audit)
    pub fn get_denied(&self) -> &[Permission] {
        &self.denied
    }
}

impl Default for PermissionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Permission check error
#[derive(Debug, Clone)]
pub enum PermissionError {
    NotGranted(String),
    Denied(String),
}

impl std::fmt::Display for PermissionError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PermissionError::NotGranted(msg) => write!(f, "Permission not granted: {}", msg),
            PermissionError::Denied(msg) => write!(f, "Permission denied: {}", msg),
        }
    }
}

impl std::error::Error for PermissionError {}

