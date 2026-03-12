/// Permission resource type (e.g., "fs.read", "net.connect")
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PermissionResource {
    FileSystem(String),   // "read", "write", "delete"
    Network(String),      // "connect", "listen"
    Process(Vec<String>), // Whitelist of allowed commands
    System(String),       // "exec", "env"
}

impl PermissionResource {
    /// Create from string like "fs.read", "net.connect", "sys.exec", "process.tar".
    /// Accepts both short (`fs`, `net`, `sys`, `process`) and long-form aliases
    /// (`filesystem`, `network`, `system`, `proc`) so that env.toml, CLI flags,
    /// and library callers all share one canonical parser.
    pub fn from_string(s: &str) -> Result<Self, String> {
        let (prefix, action) = s.split_once('.').ok_or_else(|| {
            format!(
                "Invalid permission format: '{}'. Expected 'resource.action'",
                s
            )
        })?;

        match prefix {
            "fs" | "filesystem" => Ok(PermissionResource::FileSystem(action.to_string())),
            "net" | "network" => Ok(PermissionResource::Network(action.to_string())),
            "sys" | "system" => Ok(PermissionResource::System(action.to_string())),
            "process" | "proc" => Ok(PermissionResource::Process(vec![action.to_string()])),
            _ => Err(format!("Unknown resource type: '{}'", prefix)),
        }
    }
}

impl std::fmt::Display for PermissionResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PermissionResource::FileSystem(action) => write!(f, "fs.{}", action),
            PermissionResource::Network(action) => write!(f, "net.{}", action),
            PermissionResource::System(action) => write!(f, "sys.{}", action),
            PermissionResource::Process(cmds) => write!(f, "process:[{}]", cmds.join(",")),
        }
    }
}

/// Permission with optional scope restriction
#[derive(Debug, Clone, PartialEq)]
pub struct Permission {
    pub resource: PermissionResource,
    pub scope: Option<String>, // e.g., "/tmp/*", "*.example.com"
}

impl Permission {
    pub fn new(resource: PermissionResource, scope: Option<String>) -> Self {
        Self { resource, scope }
    }
}

/// Permission manager - grants, denies, and checks permissions
pub struct PermissionManager {
    granted: Vec<Permission>,
    denied: Vec<Permission>, // Explicit denials override grants
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
    pub fn check(
        &self,
        resource: &PermissionResource,
        scope: Option<&str>,
    ) -> Result<(), PermissionError> {
        // 1. Check explicit denials first (denials override grants)
        for denied in &self.denied {
            if self.matches(&denied.resource, resource) && self.scope_matches(&denied.scope, scope)
            {
                return Err(PermissionError::Denied(format!(
                    "Permission explicitly denied: {}",
                    resource
                )));
            }
        }

        // 2. Check grants
        for granted in &self.granted {
            if self.matches(&granted.resource, resource)
                && self.scope_matches(&granted.scope, scope)
            {
                return Ok(());
            }
        }

        Err(PermissionError::NotGranted(resource.to_string()))
    }

    /// Check if two resources match
    fn matches(
        &self,
        perm_resource: &PermissionResource,
        check_resource: &PermissionResource,
    ) -> bool {
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

    /// Check if scope matches (supports glob patterns).
    fn scope_matches(&self, perm_scope: &Option<String>, check_scope: Option<&str>) -> bool {
        match (perm_scope, check_scope) {
            (None, _) => true,
            (Some(ps), Some(cs)) => glob_scope_matches(ps, cs),
            (Some(_), None) => false,
        }
    }

    /// Get all granted permissions (for debugging/audit)
    pub fn get_granted(&self) -> &[Permission] {
        &self.granted
    }

    /// Check only the explicit-deny list, ignoring grants.
    /// Used by capability-token paths to enforce that `deny_permission()` always wins,
    /// even when a valid token would otherwise bypass the full permission check.
    pub fn check_denied(
        &self,
        resource: &PermissionResource,
        scope: Option<&str>,
    ) -> Result<(), PermissionError> {
        for denied in &self.denied {
            if self.matches(&denied.resource, resource) && self.scope_matches(&denied.scope, scope)
            {
                return Err(PermissionError::Denied(format!(
                    "Permission explicitly denied: {}",
                    resource
                )));
            }
        }
        Ok(())
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

/// Glob scope matching shared by both `PermissionManager` and `CapabilityManager`.
///
/// Supports any number of `*` wildcards. Each `*` matches zero or more characters.
/// Examples: `/tmp/*`, `*.example.com`, `/var/*/log`, `/a/*/b/*`.
///
/// Uses byte-level recursion with memoisation via early termination; safe for the
/// short path/hostname patterns used in permission scopes.
pub(crate) fn glob_scope_matches(pattern: &str, value: &str) -> bool {
    glob_bytes(pattern.as_bytes(), value.as_bytes())
}

fn glob_bytes(pat: &[u8], val: &[u8]) -> bool {
    match (pat.split_first(), val.split_first()) {
        // Both exhausted — match
        (None, None) => true,
        // Pattern exhausted but value remains — no match
        (None, Some(_)) => false,
        // Wildcard: try consuming zero chars (advance pattern only)
        // or one char from value (advance value only)
        (Some((&b'*', pat_rest)), _) => {
            glob_bytes(pat_rest, val)
                || (!val.is_empty() && glob_bytes(pat, &val[1..]))
        }
        // Value exhausted but pattern has non-wildcard chars remaining — no match
        (Some(_), None) => false,
        // Literal match: both heads equal — advance both
        (Some((p, pat_rest)), Some((v, val_rest))) if p == v => glob_bytes(pat_rest, val_rest),
        // Mismatch
        _ => false,
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
