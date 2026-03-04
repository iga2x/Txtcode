use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use sha2::{Sha256, Digest};
use crate::runtime::permissions::PermissionResource;

/// Result of an audited action
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditResult {
    Allowed,
    Denied,
    Error(String),
}

impl AuditResult {
    pub fn to_string(&self) -> String {
        match self {
            AuditResult::Allowed => "allowed".to_string(),
            AuditResult::Denied => "denied".to_string(),
            AuditResult::Error(msg) => format!("error: {}", msg),
        }
    }
}

/// AI agent metadata for tracking AI involvement in execution
#[derive(Debug, Clone, PartialEq)]
pub struct AIMetadata {
    pub model: Option<String>,           // e.g., "gpt-4", "claude-3"
    pub user: Option<String>,            // User who initiated the AI request
    pub session: Option<String>,         // Session identifier
    pub policy_version: Option<String>,  // Policy version used
    pub function_name: Option<String>,   // Function where AI action occurred
    pub file_name: Option<String>,       // Source file
    pub line_number: Option<u32>,        // Line number
}

impl AIMetadata {
    pub fn new() -> Self {
        Self {
            model: None,
            user: None,
            session: None,
            policy_version: None,
            function_name: None,
            file_name: None,
            line_number: None,
        }
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = Some(model);
        self
    }

    pub fn with_user(mut self, user: String) -> Self {
        self.user = Some(user);
        self
    }

    pub fn with_session(mut self, session: String) -> Self {
        self.session = Some(session);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.model.is_none() && 
        self.user.is_none() && 
        self.session.is_none() && 
        self.policy_version.is_none() &&
        self.function_name.is_none() &&
        self.file_name.is_none() &&
        self.line_number.is_none()
    }
}

impl Default for AIMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Individual audit entry for an action
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub timestamp: SystemTime,
    pub timestamp_nanos: u128,  // Nanoseconds since epoch for precise ordering
    pub action: String,          // e.g., "fs.read", "net.connect", "process.exec"
    pub resource: String,        // Resource accessed (e.g., file path, hostname, command)
    pub permission: Option<String>, // Permission that was checked (e.g., "fs.read", "net.connect")
    pub result: AuditResult,     // Whether action was allowed, denied, or errored
    pub context: HashMap<String, String>, // Additional context (AI model, user, function, etc.)
}

impl AuditEntry {
    pub fn new(
        action: String,
        resource: String,
        permission: Option<String>,
        result: AuditResult,
    ) -> Self {
        let now = SystemTime::now();
        let nanos = now.duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);

        Self {
            timestamp: now,
            timestamp_nanos: nanos,
            action,
            resource,
            permission,
            result,
            context: HashMap::new(),
        }
    }

    /// Add AI metadata to context
    pub fn with_ai_metadata(mut self, ai_meta: &AIMetadata) -> Self {
        if let Some(model) = &ai_meta.model {
            self.context.insert("ai_model".to_string(), model.clone());
        }
        if let Some(user) = &ai_meta.user {
            self.context.insert("ai_user".to_string(), user.clone());
        }
        if let Some(session) = &ai_meta.session {
            self.context.insert("ai_session".to_string(), session.clone());
        }
        if let Some(policy) = &ai_meta.policy_version {
            self.context.insert("ai_policy_version".to_string(), policy.clone());
        }
        if let Some(func) = &ai_meta.function_name {
            self.context.insert("function".to_string(), func.clone());
        }
        if let Some(file) = &ai_meta.file_name {
            self.context.insert("file".to_string(), file.clone());
        }
        if let Some(line) = ai_meta.line_number {
            self.context.insert("line".to_string(), line.to_string());
        }
        self
    }

    /// Add custom context key-value pair
    pub fn with_context(mut self, key: String, value: String) -> Self {
        self.context.insert(key, value);
        self
    }

    /// Convert to JSON string for export
    pub fn to_json(&self) -> String {
        use std::fmt::Write;
        let mut json = String::new();
        
        write!(json, "{{\n").unwrap();
        write!(json, "  \"timestamp\": {},\n", 
               self.timestamp.duration_since(UNIX_EPOCH)
                   .map(|d| d.as_secs())
                   .unwrap_or(0)).unwrap();
        write!(json, "  \"timestamp_nanos\": {},\n", self.timestamp_nanos).unwrap();
        write!(json, "  \"action\": \"{}\",\n", escape_json(&self.action)).unwrap();
        write!(json, "  \"resource\": \"{}\",\n", escape_json(&self.resource)).unwrap();
        if let Some(perm) = &self.permission {
            write!(json, "  \"permission\": \"{}\",\n", escape_json(perm)).unwrap();
        } else {
            write!(json, "  \"permission\": null,\n").unwrap();
        }
        write!(json, "  \"result\": \"{}\",\n", self.result.to_string()).unwrap();
        
        // Write context as JSON object
        write!(json, "  \"context\": {{\n").unwrap();
        let context_iter: Vec<_> = self.context.iter().collect();
        for (i, (key, value)) in context_iter.iter().enumerate() {
            write!(json, "    \"{}\": \"{}\"", escape_json(key), escape_json(value)).unwrap();
            if i < context_iter.len() - 1 {
                write!(json, ",").unwrap();
            }
            write!(json, "\n").unwrap();
        }
        write!(json, "  }}\n").unwrap();
        write!(json, "}}").unwrap();
        
        json
    }

    /// Create entry from permission check
    pub fn from_permission_check(
        resource: &PermissionResource,
        scope: Option<&str>,
        result: Result<(), crate::runtime::permissions::PermissionError>,
    ) -> Self {
        let action = format!("permission.check.{}", resource.to_string());
        let resource_str = scope.unwrap_or("").to_string();
        let (permission, audit_result) = match result {
            Ok(()) => (Some(resource.to_string()), AuditResult::Allowed),
            Err(_e) => (Some(resource.to_string()), AuditResult::Denied),
        };
        
        Self::new(action, resource_str, permission, audit_result)
    }
}

fn escape_json(s: &str) -> String {
    s.replace("\\", "\\\\")
        .replace("\"", "\\\"")
        .replace("\n", "\\n")
        .replace("\r", "\\r")
        .replace("\t", "\\t")
}

/// Execution provenance information for calculating provenance hash
#[derive(Debug, Clone)]
pub struct ExecutionProvenance {
    pub source_hash: Option<Vec<u8>>,      // Hash of source code
    pub policies_hash: Option<Vec<u8>>,    // Hash of all policies
    pub permissions_hash: Option<Vec<u8>>, // Hash of all permissions
    pub intent_hash: Option<Vec<u8>>,      // Hash of all declared intents
    pub ai_metadata_hash: Option<Vec<u8>>, // Hash of AI metadata
    pub deterministic_mode: bool,          // Whether deterministic mode was enabled
}

impl ExecutionProvenance {
    pub fn new() -> Self {
        Self {
            source_hash: None,
            policies_hash: None,
            permissions_hash: None,
            intent_hash: None,
            ai_metadata_hash: None,
            deterministic_mode: false,
        }
    }

    /// Calculate provenance hash from all components
    pub fn calculate_hash(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        
        if let Some(ref hash) = self.source_hash {
            hasher.update(b"source:");
            hasher.update(hash);
        }
        
        if let Some(ref hash) = self.policies_hash {
            hasher.update(b"policies:");
            hasher.update(hash);
        }
        
        if let Some(ref hash) = self.permissions_hash {
            hasher.update(b"permissions:");
            hasher.update(hash);
        }
        
        if let Some(ref hash) = self.intent_hash {
            hasher.update(b"intent:");
            hasher.update(hash);
        }
        
        if let Some(ref hash) = self.ai_metadata_hash {
            hasher.update(b"ai_metadata:");
            hasher.update(hash);
        }
        
        hasher.update(b"deterministic:");
        if self.deterministic_mode {
            hasher.update(b"true");
        } else {
            hasher.update(b"false");
        }
        
        hasher.finalize().to_vec()
    }

    /// Get provenance hash as hex string
    pub fn hash_hex(&self) -> String {
        hex::encode(self.calculate_hash())
    }
}

impl Default for ExecutionProvenance {
    fn default() -> Self {
        Self::new()
    }
}

/// Immutable audit trail of all actions
pub struct AuditTrail {
    entries: Vec<AuditEntry>,
    immutable: bool,  // Once set to true, entries cannot be modified
    provenance: ExecutionProvenance,
}

impl AuditTrail {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            immutable: false,
            provenance: ExecutionProvenance::new(),
        }
    }

    /// Add an audit entry (fails if trail is immutable)
    pub fn add_entry(&mut self, entry: AuditEntry) -> Result<(), String> {
        if self.immutable {
            return Err("Audit trail is immutable and cannot be modified".to_string());
        }
        self.entries.push(entry);
        Ok(())
    }

    /// Add entry from permission check
    pub fn log_permission_check(
        &mut self,
        resource: &PermissionResource,
        scope: Option<&str>,
        result: Result<(), crate::runtime::permissions::PermissionError>,
        ai_metadata: Option<&AIMetadata>,
    ) -> Result<(), String> {
        if self.immutable {
            return Err("Audit trail is immutable and cannot be modified".to_string());
        }

        let mut entry = AuditEntry::from_permission_check(resource, scope, result);
        if let Some(ai_meta) = ai_metadata {
            entry = entry.with_ai_metadata(ai_meta);
        }
        
        self.entries.push(entry);
        Ok(())
    }
    
    #[allow(dead_code)] // Will be used when audit logging is fully integrated
    fn log_action_internal(
        &mut self,
        action: String,
        resource: String,
        permission: Option<String>,
        result: AuditResult,
        ai_metadata: Option<&AIMetadata>,
    ) -> Result<(), String> {
        if self.immutable {
            return Err("Audit trail is immutable and cannot be modified".to_string());
        }

        let mut entry = AuditEntry::new(action, resource, permission, result);
        if let Some(ai_meta) = ai_metadata {
            entry = entry.with_ai_metadata(ai_meta);
        }
        
        self.entries.push(entry);
        Ok(())
    }

    /// Log a general action
    pub fn log_action(
        &mut self,
        action: String,
        resource: String,
        permission: Option<String>,
        result: AuditResult,
        ai_metadata: Option<&AIMetadata>,
    ) -> Result<(), String> {
        if self.immutable {
            return Err("Audit trail is immutable and cannot be modified".to_string());
        }

        let mut entry = AuditEntry::new(action, resource, permission, result);
        if let Some(ai_meta) = ai_metadata {
            entry = entry.with_ai_metadata(ai_meta);
        }
        
        self.entries.push(entry);
        Ok(())
    }

    /// Make the audit trail immutable (entries cannot be modified after this)
    pub fn make_immutable(&mut self) {
        self.immutable = true;
    }

    /// Check if audit trail is immutable
    pub fn is_immutable(&self) -> bool {
        self.immutable
    }

    /// Get all entries (read-only)
    pub fn entries(&self) -> &[AuditEntry] {
        &self.entries
    }

    /// Query entries by action
    pub fn query_by_action(&self, action: &str) -> Vec<&AuditEntry> {
        self.entries.iter()
            .filter(|e| e.action.contains(action))
            .collect()
    }

    /// Query entries by resource
    pub fn query_by_resource(&self, resource: &str) -> Vec<&AuditEntry> {
        self.entries.iter()
            .filter(|e| e.resource.contains(resource))
            .collect()
    }

    /// Query entries by result
    pub fn query_by_result(&self, result: &AuditResult) -> Vec<&AuditEntry> {
        self.entries.iter()
            .filter(|e| &e.result == result)
            .collect()
    }

    /// Query entries by context key-value
    pub fn query_by_context(&self, key: &str, value: &str) -> Vec<&AuditEntry> {
        self.entries.iter()
            .filter(|e| e.context.get(key).map(|v| v == value).unwrap_or(false))
            .collect()
    }

    /// Query entries in time range (timestamp in seconds since epoch)
    pub fn query_by_time_range(&self, start: u64, end: u64) -> Vec<&AuditEntry> {
        self.entries.iter()
            .filter(|e| {
                let entry_secs = e.timestamp.duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                entry_secs >= start && entry_secs <= end
            })
            .collect()
    }

    /// Export all entries as JSON array
    pub fn export_json(&self) -> String {
        use std::fmt::Write;
        let mut json = String::new();
        write!(json, "[\n").unwrap();
        
        for (i, entry) in self.entries.iter().enumerate() {
            let entry_json = entry.to_json();
            // Indent each line of the entry JSON
            let indented: String = entry_json.lines()
                .map(|line| format!("  {}\n", line))
                .collect();
            write!(json, "{}", indented).unwrap();
            
            if i < self.entries.len() - 1 {
                write!(json, ",").unwrap();
            }
            write!(json, "\n").unwrap();
        }
        
        write!(json, "]").unwrap();
        json
    }

    /// Get execution provenance
    pub fn provenance(&self) -> &ExecutionProvenance {
        &self.provenance
    }

    /// Set execution provenance
    pub fn set_provenance(&mut self, provenance: ExecutionProvenance) {
        self.provenance = provenance;
    }

    /// Get provenance hash as hex string
    pub fn provenance_hash_hex(&self) -> String {
        self.provenance.hash_hex()
    }

    /// Get number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if audit trail is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for AuditTrail {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::permissions::{PermissionResource, PermissionError};

    #[test]
    fn test_audit_entry_creation() {
        let entry = AuditEntry::new(
            "fs.read".to_string(),
            "/tmp/test.txt".to_string(),
            Some("fs.read".to_string()),
            AuditResult::Allowed,
        );
        
        assert_eq!(entry.action, "fs.read");
        assert_eq!(entry.resource, "/tmp/test.txt");
        assert_eq!(entry.result, AuditResult::Allowed);
    }

    #[test]
    fn test_audit_trail_add_entry() {
        let mut trail = AuditTrail::new();
        let entry = AuditEntry::new(
            "fs.read".to_string(),
            "/tmp/test.txt".to_string(),
            Some("fs.read".to_string()),
            AuditResult::Allowed,
        );
        
        assert!(trail.add_entry(entry).is_ok());
        assert_eq!(trail.len(), 1);
    }

    #[test]
    fn test_audit_trail_immutable() {
        let mut trail = AuditTrail::new();
        trail.make_immutable();
        
        let entry = AuditEntry::new(
            "fs.read".to_string(),
            "/tmp/test.txt".to_string(),
            Some("fs.read".to_string()),
            AuditResult::Allowed,
        );
        
        assert!(trail.add_entry(entry).is_err());
    }

    #[test]
    fn test_permission_check_logging() {
        let mut trail = AuditTrail::new();
        let resource = PermissionResource::FileSystem("read".to_string());
        
        // Test allowed
        trail.log_permission_check(&resource, Some("/tmp/test.txt"), Ok(()), None).unwrap();
        
        // Test denied
        trail.log_permission_check(
            &resource, 
            Some("/etc/passwd"), 
            Err(PermissionError::NotGranted("test".to_string())),
            None
        ).unwrap();
        
        assert_eq!(trail.len(), 2);
        assert_eq!(trail.query_by_result(&AuditResult::Allowed).len(), 1);
        assert_eq!(trail.query_by_result(&AuditResult::Denied).len(), 1);
    }
}

