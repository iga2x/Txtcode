use super::VirtualMachine;
use crate::runtime::audit::{AIMetadata, AuditTrail};
use sha2::{Digest, Sha256};

/// Audit trail and provenance management methods for VirtualMachine
impl VirtualMachine {
    /// Get audit trail reference (read-only)
    pub fn get_audit_trail(&self) -> &AuditTrail {
        &self.audit_trail
    }

    /// Get audit trail mutable reference (for logging)
    pub fn get_audit_trail_mut(&mut self) -> &mut AuditTrail {
        &mut self.audit_trail
    }

    /// Get AI metadata reference
    pub fn get_ai_metadata(&self) -> &AIMetadata {
        &self.ai_metadata
    }

    /// Get AI metadata mutable reference
    pub fn get_ai_metadata_mut(&mut self) -> &mut AIMetadata {
        &mut self.ai_metadata
    }

    /// Set AI metadata (convenience method)
    pub fn set_ai_metadata(
        &mut self,
        model: Option<String>,
        user: Option<String>,
        session: Option<String>,
        policy_version: Option<String>,
    ) {
        self.ai_metadata.model = model;
        self.ai_metadata.user = user;
        self.ai_metadata.session = session;
        self.ai_metadata.policy_version = policy_version;
    }

    /// Calculate execution provenance hash
    pub fn calculate_provenance_hash(&self, source_code: Option<&str>) -> String {
        use crate::runtime::audit::ExecutionProvenance;

        let mut provenance = ExecutionProvenance::new();

        // Hash source code if provided
        if let Some(source) = source_code {
            let mut hasher = Sha256::new();
            hasher.update(source.as_bytes());
            provenance.source_hash = Some(hasher.finalize().to_vec());
        }

        // Hash permissions
        {
            let mut hasher = Sha256::new();
            for perm in self.permission_manager.get_granted() {
                hasher.update(format!("{}:{:?}", perm.resource, perm.scope).as_bytes());
            }
            for perm in self.permission_manager.get_denied() {
                hasher.update(format!("deny:{}:{:?}", perm.resource, perm.scope).as_bytes());
            }
            provenance.permissions_hash = Some(hasher.finalize().to_vec());
        }

        // Hash AI metadata
        if !self.ai_metadata.is_empty() {
            let mut hasher = Sha256::new();
            if let Some(model) = &self.ai_metadata.model {
                hasher.update(b"model:");
                hasher.update(model.as_bytes());
            }
            if let Some(user) = &self.ai_metadata.user {
                hasher.update(b"user:");
                hasher.update(user.as_bytes());
            }
            if let Some(session) = &self.ai_metadata.session {
                hasher.update(b"session:");
                hasher.update(session.as_bytes());
            }
            if let Some(policy) = &self.ai_metadata.policy_version {
                hasher.update(b"policy:");
                hasher.update(policy.as_bytes());
            }
            provenance.ai_metadata_hash = Some(hasher.finalize().to_vec());
        }

        provenance.hash_hex()
    }

    /// Export audit trail as JSON
    pub fn export_audit_trail_json(&self) -> String {
        self.audit_trail.export_json()
    }
}
