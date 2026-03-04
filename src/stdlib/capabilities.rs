use crate::runtime::{Value, RuntimeError};
use crate::runtime::permissions::PermissionResource;

/// Capability management library
pub struct CapabilityLib;

impl CapabilityLib {
    /// Call capability management function
    /// executor: VM instance that has capability management methods
    pub fn call_function<E: CapabilityExecutor>(name: &str, args: &[Value], executor: Option<&mut E>) -> Result<Value, RuntimeError> {
        let executor = executor.ok_or_else(|| {
            RuntimeError::new("Capability functions require VM executor".to_string())
        })?;

        match name {
            "grant_capability" => Self::grant_capability(executor, args),
            "use_capability" => Self::use_capability(executor, args),
            "revoke_capability" => Self::revoke_capability(executor, args),
            "capability_valid" => Self::capability_valid(executor, args),
            _ => Err(RuntimeError::new(format!("Unknown capability function: {}", name))),
        }
    }

    /// Grant a capability token
    /// grant_capability(resource, action, scope?, expires_in?)
    fn grant_capability<E: CapabilityExecutor>(executor: &mut E, args: &[Value]) -> Result<Value, RuntimeError> {
        if args.len() < 2 {
            return Err(RuntimeError::new("grant_capability requires at least 2 arguments: resource, action".to_string()));
        }

        let resource_str = match &args[0] {
            Value::String(s) => s.clone(),
            _ => return Err(RuntimeError::new("grant_capability: first argument must be a string (resource)".to_string())),
        };

        let action = match &args[1] {
            Value::String(s) => s.clone(),
            _ => return Err(RuntimeError::new("grant_capability: second argument must be a string (action)".to_string())),
        };

        let scope = if args.len() > 2 {
            match &args[2] {
                Value::String(s) => Some(s.clone()),
                Value::Null => None,
                _ => return Err(RuntimeError::new("grant_capability: third argument must be a string (scope) or null".to_string())),
            }
        } else {
            None
        };

        let expires_in = if args.len() > 3 {
            match &args[3] {
                Value::String(s) => {
                    // Parse duration string like "10m", "1h", "30s"
                    Self::parse_duration(s)?
                },
                Value::Integer(secs) => {
                    Some(std::time::Duration::from_secs(*secs as u64))
                },
                Value::Null => None,
                _ => return Err(RuntimeError::new("grant_capability: fourth argument must be a string (duration), integer (seconds), or null".to_string())),
            }
        } else {
            None
        };

        // Parse resource string to PermissionResource
        let resource = Self::parse_resource(&resource_str, &action)?;

        // Grant capability via executor
        let token_id = executor.grant_capability(
            resource,
            action,
            scope,
            expires_in,
            None, // granted_by
            None, // ai_metadata
        );

        Ok(Value::String(token_id))
    }

    /// Use a capability token in current scope
    /// use_capability(token_id)
    fn use_capability<E: CapabilityExecutor>(executor: &mut E, args: &[Value]) -> Result<Value, RuntimeError> {
        if args.len() < 1 {
            return Err(RuntimeError::new("use_capability requires 1 argument: token_id".to_string()));
        }

        let token_id = match &args[0] {
            Value::String(s) => s.clone(),
            _ => return Err(RuntimeError::new("use_capability: argument must be a string (token_id)".to_string())),
        };

        executor.use_capability(token_id)?;
        Ok(Value::Null)
    }

    /// Revoke a capability token
    /// revoke_capability(token_id, reason?)
    fn revoke_capability<E: CapabilityExecutor>(executor: &mut E, args: &[Value]) -> Result<Value, RuntimeError> {
        if args.len() < 1 {
            return Err(RuntimeError::new("revoke_capability requires at least 1 argument: token_id".to_string()));
        }

        let token_id = match &args[0] {
            Value::String(s) => s.clone(),
            _ => return Err(RuntimeError::new("revoke_capability: first argument must be a string (token_id)".to_string())),
        };

        let reason = if args.len() > 1 {
            match &args[1] {
                Value::String(s) => Some(s.clone()),
                Value::Null => None,
                _ => return Err(RuntimeError::new("revoke_capability: second argument must be a string (reason) or null".to_string())),
            }
        } else {
            None
        };

        executor.revoke_capability(&token_id, reason)?;
        Ok(Value::Null)
    }

    /// Check if a capability token is valid
    /// capability_valid(token_id) -> bool
    fn capability_valid<E: CapabilityExecutor>(executor: &mut E, args: &[Value]) -> Result<Value, RuntimeError> {
        if args.len() < 1 {
            return Err(RuntimeError::new("capability_valid requires 1 argument: token_id".to_string()));
        }

        let token_id = match &args[0] {
            Value::String(s) => s.clone(),
            _ => return Err(RuntimeError::new("capability_valid: argument must be a string (token_id)".to_string())),
        };

        let is_valid = executor.capability_valid(&token_id);
        Ok(Value::Boolean(is_valid))
    }

    /// Parse resource string to PermissionResource
    fn parse_resource(resource_str: &str, action: &str) -> Result<PermissionResource, RuntimeError> {
        match resource_str {
            "fs" | "filesystem" => Ok(PermissionResource::FileSystem(action.to_string())),
            "net" | "network" => Ok(PermissionResource::Network(action.to_string())),
            "sys" | "system" => Ok(PermissionResource::System(action.to_string())),
            "process" => Ok(PermissionResource::Process(vec![action.to_string()])),
            _ => Err(RuntimeError::new(format!("Unknown resource type: {}. Expected 'fs', 'net', 'sys', or 'process'", resource_str))),
        }
    }

    /// Parse duration string to Duration
    /// Supports: "10s", "5m", "1h", "30d"
    fn parse_duration(duration_str: &str) -> Result<Option<std::time::Duration>, RuntimeError> {
        if duration_str.is_empty() {
            return Ok(None);
        }

        let duration_str = duration_str.trim().to_lowercase();
        
        let (value_str, unit) = if duration_str.ends_with('s') {
            (&duration_str[..duration_str.len() - 1], "s")
        } else if duration_str.ends_with('m') {
            (&duration_str[..duration_str.len() - 1], "m")
        } else if duration_str.ends_with('h') {
            (&duration_str[..duration_str.len() - 1], "h")
        } else if duration_str.ends_with('d') {
            (&duration_str[..duration_str.len() - 1], "d")
        } else {
            // Try to parse as seconds if no unit
            return Ok(Some(std::time::Duration::from_secs(
                duration_str.parse().map_err(|_| {
                    RuntimeError::new(format!("Invalid duration format: {}. Expected format like '10s', '5m', '1h'", duration_str))
                })?
            )));
        };

        let value: u64 = value_str.parse().map_err(|_| {
            RuntimeError::new(format!("Invalid duration value: {}. Expected a number", value_str))
        })?;

        let duration = match unit {
            "s" => std::time::Duration::from_secs(value),
            "m" => std::time::Duration::from_secs(value * 60),
            "h" => std::time::Duration::from_secs(value * 3600),
            "d" => std::time::Duration::from_secs(value * 86400),
            _ => unreachable!(),
        };

        Ok(Some(duration))
    }
}

/// Trait for executors that can manage capabilities
pub trait CapabilityExecutor {
    fn grant_capability(
        &mut self,
        resource: PermissionResource,
        action: String,
        scope: Option<String>,
        expires_in: Option<std::time::Duration>,
        granted_by: Option<String>,
        ai_metadata: Option<crate::runtime::audit::AIMetadata>,
    ) -> String;

    fn use_capability(&mut self, token_id: String) -> Result<(), RuntimeError>;

    fn revoke_capability(&mut self, token_id: &str, reason: Option<String>) -> Result<(), RuntimeError>;

    fn capability_valid(&self, token_id: &str) -> bool;
}

