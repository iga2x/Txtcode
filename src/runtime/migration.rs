use crate::parser::ast::Program;
use crate::runtime::compatibility::{CompatibilityLayer, Version, MigrationReport, FeatureFlags};
use crate::runtime::errors::RuntimeError;
use crate::tools::ast_printer::{AstPrinter, detect_version_from_source};
use std::path::PathBuf;

/// Migration framework for automated code migration
pub struct MigrationFramework {
    compatibility_layer: CompatibilityLayer,
    dry_run: bool,
}

impl MigrationFramework {
    pub fn new() -> Self {
        Self {
            compatibility_layer: CompatibilityLayer::new(),
            dry_run: false,
        }
    }

    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.compatibility_layer = CompatibilityLayer::new().with_strict_mode(strict);
        self
    }

    /// Migrate a file from old version to new version
    pub fn migrate_file(
        &self,
        file_path: &PathBuf,
        source_version: Option<Version>,
        target_version: Option<Version>,
    ) -> Result<MigrationReport, RuntimeError> {
        // Read file
        let source_code = std::fs::read_to_string(file_path)
            .map_err(|e| RuntimeError::new(format!("Failed to read file {}: {}", file_path.display(), e)))?;

        // Auto-detect source version from header comment if not provided
        let source_version = source_version.unwrap_or_else(|| {
            detect_version_from_source(&source_code)
                .and_then(|v| {
                    let parts: Vec<&str> = v.split('.').collect();
                    if parts.len() == 3 {
                        let major = parts[0].parse().ok()?;
                        let minor = parts[1].parse().ok()?;
                        let patch = parts[2].parse().ok()?;
                        Some(Version::new(major, minor, patch))
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| Version::new(0, 1, 0))
        });
        let target_version = target_version.unwrap_or_else(|| Version::current());

        // Parse AST
        let mut lexer = crate::lexer::Lexer::new(source_code);
        let tokens = lexer.tokenize()
            .map_err(|e| RuntimeError::new(format!("Failed to tokenize {}: {}", file_path.display(), e)))?;
        let mut parser = crate::parser::Parser::new(tokens);
        let program = parser.parse()
            .map_err(|e| RuntimeError::new(format!("Failed to parse {}: {}", file_path.display(), e)))?;

        // Perform migration
        self.migrate_program(program, source_version, target_version, file_path)
    }

    /// Migrate a Program AST
    pub fn migrate_program(
        &self,
        program: Program,
        source_version: Version,
        target_version: Version,
        _file_path: &PathBuf,
    ) -> Result<MigrationReport, RuntimeError> {
        let target_version_str = target_version.to_string();
        let source_version_str = source_version.to_string();
        let mut report = MigrationReport::new(source_version.clone(), target_version.clone());

        // Extract feature flags from module metadata (if present)
        let flags = self.extract_feature_flags(&program);

        // Check deprecations
        let deprecation_warnings = self.compatibility_layer.check_deprecations(&program, &flags);
        let strict_mode = self.compatibility_layer.is_strict_mode();
        for warning in deprecation_warnings {
            if strict_mode {
                report.add_error(warning.message.clone());
            } else {
                report.add_warning(warning);
            }
        }

        if report.has_errors() && strict_mode {
            return Err(RuntimeError::new(format!(
                "Migration failed: {} errors found",
                report.errors.len()
            )));
        }

        // Perform AST migration
        let _migrated_program = self.compatibility_layer.migrate_ast(program, Some(source_version.clone()))?;

        // Record transformations
        // Check if any transformations were actually applied
        // (In production, compare AST before/after to detect changes)
        report.add_transformation(format!("Migrated from {} to {}", 
            source_version_str, target_version_str));

        // If not dry run, regenerate source from migrated AST and write back
        if !self.dry_run {
            let mut printer = AstPrinter::new();
            let new_source = format!(
                "# version: {}\n{}",
                target_version.to_string(),
                printer.print_program(&_migrated_program)
            );
            report.generated_source = Some(new_source.clone());
            if !_file_path.as_os_str().is_empty() {
                std::fs::write(_file_path, &new_source)
                    .map_err(|e| RuntimeError::new(format!(
                        "Failed to write migrated file {}: {}",
                        _file_path.display(), e
                    )))?;
                report.add_transformation(format!(
                    "Wrote migrated source to {}",
                    _file_path.display()
                ));
            }
        } else {
            report.add_transformation(
                "DRY RUN: Files were validated but not modified. \
                 Review the warnings above and apply changes manually."
                .to_string(),
            );
        }

        Ok(report)
    }

    /// Extract feature flags from program metadata
    /// In future: parse from module header comments like:
    /// # version: 0.1.0
    /// # feature_flags:
    /// #   legacy_return_syntax: true
    fn extract_feature_flags(&self, _program: &Program) -> FeatureFlags {
        // Placeholder - in production, parse from module metadata/headers
        FeatureFlags::default()
    }

    /// Migrate entire directory of files
    pub fn migrate_directory(
        &self,
        dir_path: &PathBuf,
        source_version: Option<Version>,
        target_version: Option<Version>,
        _pattern: Option<&str>, // File pattern like "*.tc" (currently unused)
    ) -> Result<Vec<(PathBuf, MigrationReport)>, RuntimeError> {
        let mut results = Vec::new();

        let entries = std::fs::read_dir(dir_path)
            .map_err(|e| RuntimeError::new(format!("Failed to read directory {}: {}", dir_path.display(), e)))?;

        for entry in entries {
            let entry = entry.map_err(|e| RuntimeError::new(format!("Failed to read directory entry: {}", e)))?;
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("tc") {
                let src_ver = source_version.clone();
                let tgt_ver = target_version.clone();
                match self.migrate_file(&path, src_ver, tgt_ver) {
                    Ok(report) => {
                        results.push((path, report));
                    }
                    Err(e) => {
                        return Err(RuntimeError::new(format!(
                            "Failed to migrate {}: {}",
                            path.display(),
                            e
                        )));
                    }
                }
            }
        }

        Ok(results)
    }

    /// Generate migration report summary
    pub fn generate_summary(reports: &[(PathBuf, MigrationReport)]) -> String {
        let mut summary = format!("Migration Summary: {} files\n\n", reports.len());

        let mut total_transformations = 0;
        let mut total_warnings = 0;
        let mut total_errors = 0;

        for (path, report) in reports {
            total_transformations += report.transformations_applied.len();
            total_warnings += report.warnings.len();
            total_errors += report.errors.len();

            summary.push_str(&format!("  {}:\n", path.display()));
            summary.push_str(&format!("    - Transformations: {}\n", report.transformations_applied.len()));
            summary.push_str(&format!("    - Warnings: {}\n", report.warnings.len()));
            if !report.errors.is_empty() {
                summary.push_str(&format!("    - Errors: {}\n", report.errors.len()));
            }
        }

        summary.push_str(&format!("\nTotal: {} transformations, {} warnings, {} errors\n",
            total_transformations, total_warnings, total_errors));

        summary
    }
}

/// Runtime version detector - detects code version from source/file metadata
pub struct VersionDetector;

impl VersionDetector {
    /// Detect version from program AST (cannot read comments from AST; use detect_version_from_source)
    pub fn detect_version(_program: &Program) -> Option<Version> {
        None
    }

    /// Detect version from a source file by reading its header comment `# version: X.Y.Z`
    pub fn detect_version_from_file(file_path: &PathBuf) -> Option<Version> {
        let source = std::fs::read_to_string(file_path).ok()?;
        let ver_str = detect_version_from_source(&source)?;
        let parts: Vec<&str> = ver_str.split('.').collect();
        if parts.len() == 3 {
            let major = parts[0].parse().ok()?;
            let minor = parts[1].parse().ok()?;
            let patch = parts[2].parse().ok()?;
            Some(Version::new(major, minor, patch))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_framework() {
        let framework = MigrationFramework::new().with_dry_run(true);
        assert!(framework.dry_run);
    }
}

