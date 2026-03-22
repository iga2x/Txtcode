use super::VirtualMachine;
use crate::runtime::audit::AuditTrail;
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

        // B.1: ai_metadata removed — hash omitted (was always empty)
        provenance.hash_hex()
    }

    /// Export audit trail as JSON
    pub fn export_audit_trail_json(&self) -> String {
        self.audit_trail.export_json()
    }
}
