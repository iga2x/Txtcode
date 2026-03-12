use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Code integrity and version management system
pub struct IntegritySystem {
    version: String,
    checksums: HashMap<String, Vec<u8>>,
    signatures: HashMap<String, Vec<u8>>,
}

impl IntegritySystem {
    pub fn new(version: String) -> Self {
        Self {
            version,
            checksums: HashMap::new(),
            signatures: HashMap::new(),
        }
    }

    /// Calculate checksum for code/data
    pub fn calculate_checksum(&self, data: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.update(self.version.as_bytes());
        hasher.finalize().to_vec()
    }

    /// Verify checksum
    pub fn verify_checksum(&self, data: &[u8], expected_checksum: &[u8]) -> bool {
        let calculated = self.calculate_checksum(data);
        calculated == expected_checksum
    }

    /// Sign code with a key
    pub fn sign(&mut self, identifier: String, data: &[u8], key: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.update(key);
        hasher.update(self.version.as_bytes());
        let signature = hasher.finalize().to_vec();

        self.signatures
            .insert(identifier.clone(), signature.clone());
        self.checksums
            .insert(identifier, self.calculate_checksum(data));

        signature
    }

    /// Verify signature
    pub fn verify_signature(&self, identifier: &str, data: &[u8], key: &[u8]) -> bool {
        if let Some(expected_signature) = self.signatures.get(identifier) {
            let mut hasher = Sha256::new();
            hasher.update(data);
            hasher.update(key);
            hasher.update(self.version.as_bytes());
            let calculated = hasher.finalize().to_vec();

            calculated == *expected_signature
        } else {
            false
        }
    }

    /// Check version compatibility
    pub fn check_version_compatibility(&self, other_version: &str) -> CompatibilityResult {
        let current_parts: Vec<&str> = self.version.split('.').collect();
        let other_parts: Vec<&str> = other_version.split('.').collect();

        if current_parts.len() != 3 || other_parts.len() != 3 {
            return CompatibilityResult::Incompatible {
                reason: "Invalid version format".to_string(),
            };
        }

        let current_major: u32 = current_parts[0].parse().unwrap_or(0);
        let current_minor: u32 = current_parts[1].parse().unwrap_or(0);
        let current_patch: u32 = current_parts[2].parse().unwrap_or(0);

        let other_major: u32 = other_parts[0].parse().unwrap_or(0);
        let other_minor: u32 = other_parts[1].parse().unwrap_or(0);
        let other_patch: u32 = other_parts[2].parse().unwrap_or(0);

        if current_major != other_major {
            CompatibilityResult::Incompatible {
                reason: format!(
                    "Major version mismatch: {} vs {}",
                    current_major, other_major
                ),
            }
        } else if current_minor != other_minor {
            if current_minor > other_minor {
                CompatibilityResult::BackwardCompatible
            } else {
                CompatibilityResult::Incompatible {
                    reason: format!("Minor version too old: {} < {}", other_minor, current_minor),
                }
            }
        } else if current_patch != other_patch {
            CompatibilityResult::BackwardCompatible
        } else {
            CompatibilityResult::FullyCompatible
        }
    }

    /// Generate migration script for version upgrade
    pub fn generate_migration(&self, from_version: &str, to_version: &str) -> MigrationScript {
        MigrationScript {
            from_version: from_version.to_string(),
            to_version: to_version.to_string(),
            steps: vec![
                "1. Backup current code".to_string(),
                "2. Update type definitions".to_string(),
                "3. Update function signatures".to_string(),
                "4. Run migration tests".to_string(),
            ],
        }
    }

    /// Get current version
    pub fn get_version(&self) -> &str {
        &self.version
    }

    /// Get all checksums
    pub fn get_checksums(&self) -> &HashMap<String, Vec<u8>> {
        &self.checksums
    }
}

/// Version compatibility result
#[derive(Debug, Clone, PartialEq)]
pub enum CompatibilityResult {
    FullyCompatible,
    BackwardCompatible,
    Incompatible { reason: String },
}

/// Migration script for version upgrades
#[derive(Debug, Clone)]
pub struct MigrationScript {
    pub from_version: String,
    pub to_version: String,
    pub steps: Vec<String>,
}

impl MigrationScript {
    pub fn execute(&self) -> Result<(), String> {
        // In a real implementation, this would perform the actual migration
        println!(
            "Migrating from {} to {}",
            self.from_version, self.to_version
        );
        for step in &self.steps {
            println!("  {}", step);
        }
        Ok(())
    }
}
