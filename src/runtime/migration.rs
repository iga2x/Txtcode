use crate::parser::ast::Program;
use crate::runtime::compatibility::{CompatibilityLayer, FeatureFlags, MigrationReport, Version};
use crate::runtime::errors::RuntimeError;
use crate::tools::ast_printer::{detect_version_from_source, AstPrinter};
use std::path::PathBuf;

/// Migration framework for automated code migration
pub struct MigrationFramework {
    compatibility_layer: CompatibilityLayer,
    dry_run: bool,
}

impl Default for MigrationFramework {
    fn default() -> Self {
        Self::new()
    }
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
        let source_code = std::fs::read_to_string(file_path).map_err(|e| {
            RuntimeError::new(format!(
                "Failed to read file {}: {}",
                file_path.display(),
                e
            ))
        })?;

        // Apply source-level migration passes first (before parsing)
        let src_ver_for_source = source_version.clone();
        let (source_code, _source_passes_applied) = {
            let reg = default_source_registry();
            let sv = src_ver_for_source.unwrap_or_else(|| {
                crate::tools::ast_printer::detect_version_from_source(&source_code)
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
            reg.apply(&source_code, &sv)
        };

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
        let target_version = target_version.unwrap_or_else(Version::current);

        // Parse AST
        let mut lexer = crate::lexer::Lexer::new(source_code);
        let tokens = lexer.tokenize().map_err(|e| {
            RuntimeError::new(format!("Failed to tokenize {}: {}", file_path.display(), e))
        })?;
        let mut parser = crate::parser::Parser::new(tokens);
        let program = parser.parse().map_err(|e| {
            RuntimeError::new(format!("Failed to parse {}: {}", file_path.display(), e))
        })?;

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
        let deprecation_warnings = self
            .compatibility_layer
            .check_deprecations(&program, &flags);
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
        let _migrated_program = self
            .compatibility_layer
            .migrate_ast(program, Some(source_version.clone()))?;

        // Record transformations
        // Check if any transformations were actually applied
        // (In production, compare AST before/after to detect changes)
        report.add_transformation(format!(
            "Migrated from {} to {}",
            source_version_str, target_version_str
        ));

        // If not dry run, regenerate source from migrated AST and write back
        if !self.dry_run {
            let mut printer = AstPrinter::new();
            let new_source = format!(
                "# version: {}\n{}",
                target_version,
                printer.print_program(&_migrated_program)
            );
            report.generated_source = Some(new_source.clone());
            if !_file_path.as_os_str().is_empty() {
                std::fs::write(_file_path, &new_source).map_err(|e| {
                    RuntimeError::new(format!(
                        "Failed to write migrated file {}: {}",
                        _file_path.display(),
                        e
                    ))
                })?;
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

        let entries = std::fs::read_dir(dir_path).map_err(|e| {
            RuntimeError::new(format!(
                "Failed to read directory {}: {}",
                dir_path.display(),
                e
            ))
        })?;

        for entry in entries {
            let entry = entry
                .map_err(|e| RuntimeError::new(format!("Failed to read directory entry: {}", e)))?;
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
            summary.push_str(&format!(
                "    - Transformations: {}\n",
                report.transformations_applied.len()
            ));
            summary.push_str(&format!("    - Warnings: {}\n", report.warnings.len()));
            if !report.errors.is_empty() {
                summary.push_str(&format!("    - Errors: {}\n", report.errors.len()));
            }
        }

        summary.push_str(&format!(
            "\nTotal: {} transformations, {} warnings, {} errors\n",
            total_transformations, total_warnings, total_errors
        ));

        summary
    }
}

// ---------------------------------------------------------------------------
// MigrationRegistry — extensible, version-ordered migration pass architecture
// ---------------------------------------------------------------------------

/// Error type for migration passes.
#[derive(Debug)]
pub struct MigrationError(pub String);

impl std::fmt::Display for MigrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<MigrationError> for RuntimeError {
    fn from(e: MigrationError) -> Self {
        RuntimeError::new(e.0)
    }
}

/// A single migration transformation applied to a Program AST.
///
/// Implement this trait for each version-to-version migration step.
/// Passes are applied in version order by `MigrationRegistry::apply`.
pub trait MigrationPass: Send + Sync {
    /// A human-readable name for this pass (used in reports).
    fn name(&self) -> &str;

    /// Apply the migration transformation to `program`.
    /// Return the (potentially modified) program on success.
    fn apply(&self, program: Program) -> Result<Program, MigrationError>;
}

/// Registry of migration passes, each associated with a version range `(from, to)`.
///
/// Passes are stored in insertion order and applied in the order that their
/// version ranges are traversed (ascending). A pass is applied when
/// `source_version <= from` and `to <= target_version`.
///
/// # Example
/// ```text
/// let mut registry = MigrationRegistry::new();
/// registry.register(Version::new(0,2,0), Version::new(0,3,0), MyPass02To03);
/// registry.register(Version::new(0,3,0), Version::new(0,4,0), MyPass03To04);
/// let (migrated, applied) = registry.apply(program, &from_ver, &to_ver)?;
/// ```
pub struct MigrationRegistry {
    passes: Vec<(Version, Version, Box<dyn MigrationPass>)>,
}

impl Default for MigrationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MigrationRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self { passes: Vec::new() }
    }

    /// Register a migration pass for the range `[from_ver, to_ver)`.
    ///
    /// Passes are applied when `source_version <= from_ver` and
    /// `to_ver <= target_version` — i.e., the pass bridges a version gap
    /// that falls within the requested migration window.
    pub fn register<P: MigrationPass + 'static>(
        &mut self,
        from_ver: Version,
        to_ver: Version,
        pass: P,
    ) {
        self.passes.push((from_ver, to_ver, Box::new(pass)));
    }

    /// Apply all registered passes whose version range falls within
    /// `[source_version, target_version]`, in version-ascending order.
    ///
    /// Returns the transformed `Program` and a list of pass names that ran.
    pub fn apply(
        &self,
        mut program: Program,
        source_version: &Version,
        target_version: &Version,
    ) -> Result<(Program, Vec<String>), MigrationError> {
        // Collect applicable passes (those whose range is within the migration window).
        let mut applicable: Vec<&(Version, Version, Box<dyn MigrationPass>)> = self
            .passes
            .iter()
            .filter(|(from, to, _)| from >= source_version && to <= target_version)
            .collect();

        // Sort by the `from` version so passes run in ascending version order.
        applicable.sort_by(|(a_from, _, _), (b_from, _, _)| a_from.cmp(b_from));

        let mut applied = Vec::new();
        for (_, _, pass) in applicable {
            program = pass.apply(program)?;
            applied.push(pass.name().to_string());
        }

        Ok((program, applied))
    }

    /// Number of registered passes.
    pub fn len(&self) -> usize {
        self.passes.len()
    }

    /// True when no passes are registered.
    pub fn is_empty(&self) -> bool {
        self.passes.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Built-in no-op migration passes for v0.2→v0.3 and v0.3→v0.4.
// These document that no structural AST changes were needed for those hops.
// When a real transformation is required, replace the pass body.
// ---------------------------------------------------------------------------

struct NoOpPass(&'static str);

impl MigrationPass for NoOpPass {
    fn name(&self) -> &str {
        self.0
    }
    fn apply(&self, program: Program) -> Result<Program, MigrationError> {
        Ok(program)
    }
}

/// Build a `MigrationRegistry` pre-populated with all known passes.
pub fn default_registry() -> MigrationRegistry {
    // M.3: Removed v0.2→v0.3 and v0.3→v0.4 passes — no user data from those
    // versions exists in the wild. Only keep passes for versions that were
    // actually shipped with users.
    let mut r = MigrationRegistry::new();
    r.register(
        Version::new(0, 4, 0),
        Version::new(0, 5, 0),
        NoOpPass("v0.4→v0.5: no structural AST changes"),
    );
    r.register(
        Version::new(0, 9, 0),
        Version::new(1, 0, 0),
        NoOpPass("v0.9→v1.0: no structural AST changes"),
    );
    r
}

// ---------------------------------------------------------------------------
// Source-level migration passes (operate on raw source text before parsing).
// These handle cases where AST-level transformation is insufficient because
// the new parser would reject the old syntax before we could fix it.
// ---------------------------------------------------------------------------

/// A migration pass that operates on raw source text.
pub trait SourceMigrationPass: Send + Sync {
    fn name(&self) -> &str;
    /// Introduced in this version (source_version must be <= introduced).
    fn introduced_in(&self) -> Version;
    /// Apply the text transformation. Returns (new_source, changed: bool).
    fn apply_source(&self, source: &str) -> (String, bool);
}

/// Registry of source-level migration passes.
pub struct SourceMigrationRegistry {
    passes: Vec<Box<dyn SourceMigrationPass>>,
}

impl Default for SourceMigrationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceMigrationRegistry {
    pub fn new() -> Self {
        Self { passes: Vec::new() }
    }

    pub fn register<P: SourceMigrationPass + 'static>(&mut self, pass: P) {
        self.passes.push(Box::new(pass));
    }

    /// Apply all passes whose `introduced_in` version is > source_version
    /// (i.e., old code predates this pass and needs it applied).
    pub fn apply(
        &self,
        source: &str,
        source_version: &Version,
    ) -> (String, Vec<String>) {
        let mut text = source.to_string();
        let mut applied = Vec::new();
        for pass in &self.passes {
            if source_version < &pass.introduced_in() {
                let (new_text, changed) = pass.apply_source(&text);
                if changed {
                    applied.push(pass.name().to_string());
                    text = new_text;
                }
            }
        }
        (text, applied)
    }
}

// ── Concrete source migration passes ────────────────────────────────────────

/// v0.5→v0.6: Add `f` prefix to double-quoted strings that contain `{identifier}`.
///
/// Before v0.6, ALL strings with `{expr}` were treated as interpolated.
/// After v0.6 only `f"..."` strings interpolate. Old code that relied on the
/// old behaviour needs an `f` prefix added.
pub struct StringInterpolationMigration;

impl SourceMigrationPass for StringInterpolationMigration {
    fn name(&self) -> &str {
        "v0.5→v0.6: Add f-prefix to interpolated strings"
    }
    fn introduced_in(&self) -> Version {
        Version::new(0, 6, 0)
    }
    fn apply_source(&self, source: &str) -> (String, bool) {
        // Regex-free implementation: scan for `"...{word}..."` patterns that are
        // NOT already preceded by `f`, `r`, or `b`.
        let mut out = String::with_capacity(source.len() + 16);
        let chars: Vec<char> = source.chars().collect();
        let len = chars.len();
        let mut i = 0;
        let mut changed = false;

        while i < len {
            // Skip line comments (`# ...`)
            if chars[i] == '#' {
                while i < len && chars[i] != '\n' {
                    out.push(chars[i]);
                    i += 1;
                }
                continue;
            }

            // Check for a plain `"` not preceded by f/r/b
            if chars[i] == '"' {
                let prev = if i > 0 { chars[i - 1] } else { ' ' };
                let already_prefixed = matches!(prev, 'f' | 'r' | 'b');

                // Collect the string literal
                let mut s = String::new();
                i += 1; // skip opening `"`
                let mut has_interpolation = false;

                while i < len {
                    if chars[i] == '\\' && i + 1 < len {
                        s.push(chars[i]);
                        s.push(chars[i + 1]);
                        i += 2;
                        continue;
                    }
                    if chars[i] == '"' {
                        i += 1; // skip closing `"`
                        break;
                    }
                    // Check for `{identifier}`
                    if chars[i] == '{' {
                        let j = i + 1;
                        let mut k = j;
                        while k < len && (chars[k].is_alphanumeric() || chars[k] == '_') {
                            k += 1;
                        }
                        if k > j && k < len && chars[k] == '}' {
                            has_interpolation = true;
                        }
                    }
                    s.push(chars[i]);
                    i += 1;
                }

                if has_interpolation && !already_prefixed {
                    // Insert `f` before the opening quote
                    out.push('f');
                    changed = true;
                }
                out.push('"');
                out.push_str(&s);
                out.push('"');
                continue;
            }

            out.push(chars[i]);
            i += 1;
        }

        (out, changed)
    }
}

/// v0.7→v0.8: Rewrite `assert → expr → "msg"` to `assert(expr, "msg")`.
///
/// At some point `assert` used the arrow syntax; in v0.8+ it is a plain
/// function call.
pub struct AssertSyntaxMigration;

impl SourceMigrationPass for AssertSyntaxMigration {
    fn name(&self) -> &str {
        "v0.7→v0.8: assert arrow syntax → function call"
    }
    fn introduced_in(&self) -> Version {
        Version::new(0, 8, 0)
    }
    fn apply_source(&self, source: &str) -> (String, bool) {
        let mut out = String::new();
        let mut changed = false;
        for line in source.lines() {
            let trimmed = line.trim_start();
            // Match: `assert → <expr> → <msg>` (two arrows)
            if trimmed.starts_with("assert") {
                let rest = trimmed.trim_start_matches("assert").trim_start();
                if let Some(after_first) = rest.strip_prefix("→").map(str::trim_start) {
                    // Find the second `→`
                    if let Some(arrow2) = after_first.find("→") {
                        let expr = after_first[..arrow2].trim();
                        let msg = after_first[arrow2 + "→".len()..].trim();
                        let indent = &line[..line.len() - trimmed.len()];
                        out.push_str(&format!("{}assert({}, {})\n", indent, expr, msg));
                        changed = true;
                        continue;
                    }
                }
            }
            out.push_str(line);
            out.push('\n');
        }
        // Remove trailing newline added by line iteration if source didn't have one
        if !source.ends_with('\n') && out.ends_with('\n') {
            out.pop();
        }
        (out, changed)
    }
}

/// v0.8→v0.9: Rewrite bare `yield expr` to `yield → expr` inside generator bodies.
pub struct YieldArrowMigration;

impl SourceMigrationPass for YieldArrowMigration {
    fn name(&self) -> &str {
        "v0.8→v0.9: yield expr → yield → expr"
    }
    fn introduced_in(&self) -> Version {
        Version::new(0, 9, 0)
    }
    fn apply_source(&self, source: &str) -> (String, bool) {
        let mut out = String::new();
        let mut changed = false;
        for line in source.lines() {
            let trimmed = line.trim_start();
            // Match `yield <non-arrow>` — skip if already `yield →`
            if trimmed.starts_with("yield ") {
                let rest = trimmed.strip_prefix("yield ").unwrap_or("").trim_start();
                if !rest.starts_with('→') && !rest.is_empty() {
                    let indent = &line[..line.len() - trimmed.len()];
                    out.push_str(&format!("{}yield → {}\n", indent, rest));
                    changed = true;
                    continue;
                }
            }
            out.push_str(line);
            out.push('\n');
        }
        if !source.ends_with('\n') && out.ends_with('\n') {
            out.pop();
        }
        (out, changed)
    }
}

/// Build the default `SourceMigrationRegistry` with all known source-level passes.
pub fn default_source_registry() -> SourceMigrationRegistry {
    let mut r = SourceMigrationRegistry::new();
    r.register(StringInterpolationMigration);
    r.register(AssertSyntaxMigration);
    r.register(YieldArrowMigration);
    r
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

    #[test]
    fn test_string_interpolation_migration_adds_f_prefix() {
        let pass = StringInterpolationMigration;
        let src = r#"store → msg → "hello {name}""#;
        let (out, changed) = pass.apply_source(src);
        assert!(changed, "Should detect interpolation pattern");
        assert!(out.contains("f\"hello {name}\""), "Should add f prefix, got: {}", out);
    }

    #[test]
    fn test_string_interpolation_migration_skips_already_prefixed() {
        let pass = StringInterpolationMigration;
        let src = r#"store → msg → f"hello {name}""#;
        let (_, changed) = pass.apply_source(src);
        assert!(!changed, "Should not modify already-prefixed f-strings");
    }

    #[test]
    fn test_string_interpolation_migration_skips_plain_strings() {
        let pass = StringInterpolationMigration;
        let src = r#"store → msg → "hello world""#;
        let (_, changed) = pass.apply_source(src);
        assert!(!changed, "Should not modify strings without interpolation");
    }

    #[test]
    fn test_assert_syntax_migration() {
        let pass = AssertSyntaxMigration;
        let src = "assert → x > 0 → \"x must be positive\"";
        let (out, changed) = pass.apply_source(src);
        assert!(changed, "Should rewrite assert arrow syntax");
        assert!(
            out.contains("assert(x > 0,") && out.contains("\"x must be positive\""),
            "Should produce function call form, got: {}",
            out
        );
    }

    #[test]
    fn test_assert_syntax_migration_skips_function_call() {
        let pass = AssertSyntaxMigration;
        let src = "assert(x > 0, \"msg\")";
        let (_, changed) = pass.apply_source(src);
        assert!(!changed, "Should not modify already-function-call assert");
    }

    #[test]
    fn test_yield_arrow_migration() {
        let pass = YieldArrowMigration;
        let src = "yield 42";
        let (out, changed) = pass.apply_source(src);
        assert!(changed, "Should rewrite bare yield");
        assert!(out.trim() == "yield → 42", "Should produce yield → form, got: {}", out.trim());
    }

    #[test]
    fn test_yield_arrow_migration_skips_already_arrow() {
        let pass = YieldArrowMigration;
        let src = "yield → 42";
        let (_, changed) = pass.apply_source(src);
        assert!(!changed, "Should not modify already-arrow yield");
    }

    #[test]
    fn test_default_source_registry_applies_to_old_version() {
        let reg = default_source_registry();
        let src = "store → x → \"value {y}\"";
        let old_ver = Version::new(0, 5, 0);
        let (out, applied) = reg.apply(src, &old_ver);
        assert!(!applied.is_empty(), "Should apply at least one pass for v0.5 code");
        assert!(out.contains("f\""), "Should add f prefix for old version");
    }

    #[test]
    fn test_default_source_registry_skips_current_version() {
        let reg = default_source_registry();
        let src = "store → x → \"value {y}\"";
        let cur_ver = Version::new(1, 0, 0);
        let (_, applied) = reg.apply(src, &cur_ver);
        assert!(applied.is_empty(), "Should skip all passes for current version code");
    }

    #[test]
    fn test_migration_registry_default_passes() {
        let reg = default_registry();
        // M.3: v0.2→v0.3 and v0.3→v0.4 removed; 2 passes remain
        assert!(reg.len() >= 2, "Should have at least 2 registered passes");
    }
}
