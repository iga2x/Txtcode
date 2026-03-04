use crate::runtime::compatibility::{Version, FeatureFlags};
use std::collections::HashMap;
use std::path::PathBuf;

/// Module metadata extracted from source code
#[derive(Debug, Clone)]
pub struct ModuleMetadata {
    pub version: Option<Version>,
    pub feature_flags: FeatureFlags,
    pub file_path: Option<PathBuf>,
    pub raw_metadata: HashMap<String, String>,
}

impl ModuleMetadata {
    pub fn new() -> Self {
        Self {
            version: None,
            feature_flags: FeatureFlags::default(),
            file_path: None,
            raw_metadata: HashMap::new(),
        }
    }

    /// Parse metadata from source code comments
    /// Looks for patterns like:
    /// # version: 0.1.0
    /// # feature_flags:
    /// #   legacy_return_syntax: true
    /// #   deprecated_exec_allowed: true
    pub fn from_source(source: &str) -> Self {
        let mut metadata = Self::new();
        let mut current_section: Option<String> = None;

        for line in source.lines() {
            let trimmed = line.trim();
            
            // Skip non-comment lines
            if !trimmed.starts_with('#') {
                continue;
            }

            let content = trimmed.trim_start_matches('#').trim();

            // Parse version
            if let Some(version_str) = content.strip_prefix("version:") {
                if let Ok(version) = Version::from_string(version_str.trim()) {
                    metadata.version = Some(version);
                }
            }

            // Parse feature flags section
            if content == "feature_flags:" {
                current_section = Some("feature_flags".to_string());
                continue;
            }

            if let Some(ref section) = current_section {
                if section == "feature_flags" {
                    if let Some((key, value)) = content.split_once(':') {
                        let key = key.trim();
                        let value = value.trim();
                        metadata.raw_metadata.insert(key.to_string(), value.to_string());
                        
                        // Map to feature flags
                        match key {
                            "legacy_return_syntax" => {
                                metadata.feature_flags.legacy_return_syntax = value == "true";
                            }
                            "deprecated_exec_allowed" => {
                                metadata.feature_flags.deprecated_exec_allowed = value == "true";
                            }
                            "enable_new_permission_system" => {
                                metadata.feature_flags.enable_new_permission_system = value == "true";
                            }
                            "strict_path_validation" => {
                                metadata.feature_flags.strict_path_validation = value == "true";
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Reset section if we hit a non-feature-flag line
            if current_section.is_some() && !content.starts_with("  ") && !content.contains(':') {
                current_section = None;
            }
        }

        metadata
    }

    /// Get feature flags for this module
    pub fn get_feature_flags(&self) -> &FeatureFlags {
        &self.feature_flags
    }

    /// Get version for this module
    pub fn get_version(&self) -> Option<&Version> {
        self.version.as_ref()
    }
}

impl Default for ModuleMetadata {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version() {
        let source = r#"
# version: 0.1.0
# feature_flags:
#   legacy_return_syntax: true
#   deprecated_exec_allowed: true
"#;
        let metadata = ModuleMetadata::from_source(source);
        assert_eq!(metadata.version, Some(Version::new(0, 1, 0)));
        assert!(metadata.feature_flags.legacy_return_syntax);
        assert!(metadata.feature_flags.deprecated_exec_allowed);
    }
}

