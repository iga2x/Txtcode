use crate::parser::ast::{Expression, Program, Span, Statement};
use crate::runtime::errors::RuntimeError;
use crate::tools::logger::log_warn;
use std::collections::HashMap;

/// Version information for compatibility tracking
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub fn from_string(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(format!("Invalid version format: {}", s));
        }
        Ok(Self {
            major: parts[0].parse().map_err(|_| "Invalid major version")?,
            minor: parts[1].parse().map_err(|_| "Invalid minor version")?,
            patch: parts[2].parse().map_err(|_| "Invalid patch version")?,
        })
    }

    /// Current runtime version — always matches the Cargo package version (6.1 fix).
    pub fn current() -> Self {
        // Parse CARGO_PKG_VERSION at compile time so this is always accurate.
        let ver = env!("CARGO_PKG_VERSION");
        Self::from_string(ver).unwrap_or(Self::new(0, 4, 0))
    }

    /// Check if this version (runtime) is compatible with `other` (source).
    ///
    /// - Runtime newer major → BackwardCompatible (migration needed)
    /// - Runtime older major → Incompatible (source too new for this runtime)
    /// - Same major, runtime newer minor → BackwardCompatible
    /// - Same major, runtime older minor → Incompatible
    /// - Same major.minor → FullyCompatible
    pub fn is_compatible_with(&self, other: &Self) -> CompatibilityResult {
        if self.major > other.major {
            // Runtime is newer: migration needed
            CompatibilityResult::BackwardCompatible
        } else if self.major < other.major {
            // Source is newer than runtime: cannot run
            CompatibilityResult::Incompatible {
                reason: format!("Major version mismatch: {} vs {}", self.major, other.major),
            }
        } else if self.minor > other.minor {
            CompatibilityResult::BackwardCompatible
        } else if self.minor < other.minor {
            CompatibilityResult::Incompatible {
                reason: format!("Minor version too old: {} < {}", other.minor, self.minor),
            }
        } else {
            CompatibilityResult::FullyCompatible
        }
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Compatibility check result
#[derive(Debug, Clone, PartialEq)]
pub enum CompatibilityResult {
    FullyCompatible,
    BackwardCompatible, // Can run with migration
    Incompatible { reason: String },
}

/// Module-scoped feature flags
#[derive(Debug, Clone, PartialEq)]
pub struct FeatureFlags {
    pub enable_new_permission_system: bool,
    pub enable_audit_trail: bool,
    pub strict_path_validation: bool,
    pub deprecated_exec_allowed: bool, // Keep old behavior available
    pub legacy_return_syntax: bool,    // Allow "return ->"
    pub module_version: Option<Version>,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            enable_new_permission_system: true,
            enable_audit_trail: true,
            strict_path_validation: true,
            deprecated_exec_allowed: true, // Backward compatibility
            legacy_return_syntax: false,   // Disable by default
            module_version: None,
        }
    }
}

impl FeatureFlags {
    /// Create feature flags from module metadata
    pub fn from_metadata(metadata: &HashMap<String, String>) -> Self {
        let mut flags = Self::default();

        if let Some(v) = metadata.get("version") {
            flags.module_version = Version::from_string(v).ok();
        }

        if let Some(v) = metadata.get("enable_new_permission_system") {
            flags.enable_new_permission_system = v == "true";
        }

        if let Some(v) = metadata.get("legacy_return_syntax") {
            flags.legacy_return_syntax = v == "true";
        }

        flags
    }
}

/// Migration metadata attached to AST nodes
#[derive(Debug, Clone)]
pub struct MigrationMetadata {
    pub original_syntax: String,
    pub migration_version: Version,
    pub migration_notes: Vec<String>,
}

/// AST-level compatibility layer
pub struct CompatibilityLayer {
    current_version: Version,
    pub(crate) strict_mode: bool, // If true, errors on deprecated features instead of warning
}

impl Default for CompatibilityLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl CompatibilityLayer {
    pub fn new() -> Self {
        Self {
            current_version: Version::current(),
            strict_mode: false,
        }
    }

    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.strict_mode = strict;
        self
    }

    /// Check if strict mode is enabled
    pub fn is_strict_mode(&self) -> bool {
        self.strict_mode
    }

    /// Get current version
    pub fn current_version(&self) -> &Version {
        &self.current_version
    }

    /// Migrate AST from older version to current version
    pub fn migrate_ast(
        &self,
        mut program: Program,
        source_version: Option<Version>,
    ) -> Result<Program, RuntimeError> {
        let source_version = source_version.unwrap_or_else(|| Version::new(0, 1, 0));

        // Check compatibility
        match self.current_version.is_compatible_with(&source_version) {
            CompatibilityResult::FullyCompatible => {
                // No migration needed
                return Ok(program);
            }
            CompatibilityResult::Incompatible { reason } => {
                return Err(RuntimeError::new(format!(
                    "Version incompatibility: {}. Source version: {}, Runtime version: {}",
                    reason, source_version, self.current_version
                )));
            }
            CompatibilityResult::BackwardCompatible => {
                // Migration needed - continue
            }
        }

        // Apply migrations based on version
        if source_version.major < self.current_version.major {
            program = self.migrate_major_version(program, &source_version)?;
        } else if source_version.minor < self.current_version.minor {
            program = self.migrate_minor_version(program, &source_version)?;
        }

        Ok(program)
    }

    /// Migrate AST for major version changes
    fn migrate_major_version(
        &self,
        program: Program,
        from_version: &Version,
    ) -> Result<Program, RuntimeError> {
        // For major version migrations, apply all breaking changes
        // Handles 0.1.0 → 0.2.0 only; later minor/patch migrations handled in migrate_minor_version

        if from_version.major == 0 && from_version.minor == 1 {
            // Migrate from 0.1.0 to 0.2.0
            self.migrate_0_1_to_0_2(program)
        } else {
            Ok(program)
        }
    }

    /// Migrate AST for minor version changes
    fn migrate_minor_version(
        &self,
        program: Program,
        _from_version: &Version,
    ) -> Result<Program, RuntimeError> {
        // Minor version changes are backward compatible but may need syntax updates
        Ok(program) // Placeholder - add minor version migrations as needed
    }

    /// Migrate from version 0.1.0 to 0.2.0
    /// Main changes:
    /// - exec_allowed flag → permission system
    /// - return -> syntax → return syntax
    /// - Path validation changes
    fn migrate_0_1_to_0_2(&self, mut program: Program) -> Result<Program, RuntimeError> {
        let mut migration_notes = Vec::new();

        // Transform statements
        let transformed_statements: Vec<Statement> = program
            .statements
            .into_iter()
            .map(|stmt| self.transform_statement_0_1_to_0_2(stmt, &mut migration_notes))
            .collect();

        program.statements = transformed_statements;

        if !migration_notes.is_empty() {
            log_warn(&format!("Migration notes: {}", migration_notes.join(", ")));
        }

        Ok(program)
    }

    /// Transform a statement for 0.1 -> 0.2 migration
    #[allow(clippy::only_used_in_recursion)]
    fn transform_statement_0_1_to_0_2(
        &self,
        stmt: Statement,
        notes: &mut Vec<String>,
    ) -> Statement {
        match stmt {
            Statement::Return { value, span } => {
                // Check if this was using legacy "return ->" syntax
                // If so, the value might need transformation
                // For now, return statement is already correct in AST
                Statement::Return { value, span } // span used in struct construction
            }
            Statement::FunctionDef {
                name,
                type_params,
                params,
                return_type,
                body,
                is_async,
                intent: _,
                ai_hint: _,
                allowed_actions: _,
                forbidden_actions: _,
                span, // used in struct construction
            } => {
                // Transform function body
                let transformed_body: Vec<Statement> = body
                    .into_iter()
                    .map(|s| self.transform_statement_0_1_to_0_2(s, notes))
                    .collect();

                Statement::FunctionDef {
                    name,
                    type_params,
                    params,
                    return_type,
                    body: transformed_body,
                    is_async,
                    intent: None,
                    ai_hint: None,
                    allowed_actions: Vec::new(),
                    forbidden_actions: Vec::new(),
                    span, // used in struct construction
                }
            }
            Statement::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
                span, // used in struct construction
            } => {
                let transformed_then: Vec<Statement> = then_branch
                    .into_iter()
                    .map(|s| self.transform_statement_0_1_to_0_2(s, notes))
                    .collect();

                let transformed_else_ifs: Vec<(Expression, Vec<Statement>)> = else_if_branches
                    .into_iter()
                    .map(|(cond, branch)| {
                        let transformed_branch: Vec<Statement> = branch
                            .into_iter()
                            .map(|s| self.transform_statement_0_1_to_0_2(s, notes))
                            .collect();
                        (cond, transformed_branch)
                    })
                    .collect();

                let transformed_else = else_branch.map(|branch| {
                    branch
                        .into_iter()
                        .map(|s| self.transform_statement_0_1_to_0_2(s, notes))
                        .collect()
                });

                Statement::If {
                    condition,
                    then_branch: transformed_then,
                    else_if_branches: transformed_else_ifs,
                    else_branch: transformed_else,
                    span, // used in struct construction
                }
            }
            Statement::While {
                condition,
                body,
                span,
            } => {
                let transformed_body: Vec<Statement> = body
                    .into_iter()
                    .map(|s| self.transform_statement_0_1_to_0_2(s, notes))
                    .collect();
                Statement::While {
                    condition,
                    body: transformed_body,
                    span,
                }
            }
            Statement::For {
                variable,
                iterable,
                body,
                span,
            } => {
                let transformed_body: Vec<Statement> = body
                    .into_iter()
                    .map(|s| self.transform_statement_0_1_to_0_2(s, notes))
                    .collect();
                Statement::For {
                    variable,
                    iterable,
                    body: transformed_body,
                    span,
                }
            }
            Statement::Repeat { count, body, span } => {
                let transformed_body: Vec<Statement> = body
                    .into_iter()
                    .map(|s| self.transform_statement_0_1_to_0_2(s, notes))
                    .collect();
                Statement::Repeat {
                    count,
                    body: transformed_body,
                    span,
                }
            }
            Statement::DoWhile {
                body,
                condition,
                span,
            } => {
                // span used in struct construction
                let transformed_body: Vec<Statement> = body
                    .into_iter()
                    .map(|s| self.transform_statement_0_1_to_0_2(s, notes))
                    .collect();
                Statement::DoWhile {
                    body: transformed_body,
                    condition,
                    span,
                }
            }
            Statement::Try {
                body,
                catch,
                finally,
                span,
            } => {
                let transformed_body: Vec<Statement> = body
                    .into_iter()
                    .map(|s| self.transform_statement_0_1_to_0_2(s, notes))
                    .collect();

                let transformed_catch = catch.map(|(var, catch_body)| {
                    let transformed_catch_body: Vec<Statement> = catch_body
                        .into_iter()
                        .map(|s| self.transform_statement_0_1_to_0_2(s, notes))
                        .collect();
                    (var, transformed_catch_body)
                });

                let transformed_finally = finally.map(|finally_body| {
                    finally_body
                        .into_iter()
                        .map(|s| self.transform_statement_0_1_to_0_2(s, notes))
                        .collect()
                });

                Statement::Try {
                    body: transformed_body,
                    catch: transformed_catch,
                    finally: transformed_finally,
                    span,
                }
            }
            Statement::Match {
                value,
                cases,
                default,
                span,
            } => {
                let transformed_cases: Vec<(
                    crate::parser::ast::Pattern,
                    Option<Expression>,
                    Vec<Statement>,
                )> = cases
                    .into_iter()
                    .map(|(pattern, guard, case_body)| {
                        let transformed_case_body: Vec<Statement> = case_body
                            .into_iter()
                            .map(|s| self.transform_statement_0_1_to_0_2(s, notes))
                            .collect();
                        (pattern, guard, transformed_case_body)
                    })
                    .collect();

                let transformed_default = default.map(|default_body| {
                    default_body
                        .into_iter()
                        .map(|s| self.transform_statement_0_1_to_0_2(s, notes))
                        .collect()
                });

                Statement::Match {
                    value,
                    cases: transformed_cases,
                    default: transformed_default,
                    span,
                }
            }
            // Other statements don't need transformation or are leaf nodes
            _ => stmt,
        }
    }

    /// Detect deprecated features in AST and emit warnings/errors
    pub fn check_deprecations(
        &self,
        program: &Program,
        flags: &FeatureFlags,
    ) -> Vec<DeprecationWarning> {
        let mut warnings = Vec::new();

        self.check_statement_deprecations(&program.statements, flags, &mut warnings);

        warnings
    }

    #[allow(clippy::only_used_in_recursion)]
    fn check_statement_deprecations(
        &self,
        statements: &[Statement],
        flags: &FeatureFlags,
        warnings: &mut Vec<DeprecationWarning>,
    ) {
        for stmt in statements {
            match stmt {
                Statement::FunctionDef {
                    body, span: _span, ..
                } => {
                    // Check function body for deprecated constructs
                    self.check_statement_deprecations(body, flags, warnings);
                }
                _ => {
                    // Check for other deprecated patterns
                }
            }
        }
    }

    /// Transform legacy exec_allowed flag to permission system
    /// This is a runtime transformation, not just AST
    pub fn migrate_exec_allowed_to_permissions(
        &self,
        has_exec_allowed: bool,
        exec_allowed_value: bool,
    ) -> Vec<Statement> {
        if !has_exec_allowed {
            return Vec::new();
        }

        let mut permissions = Vec::new();

        if exec_allowed_value {
            // Old code had exec_allowed: true
            // In new system, this means: permission → sys.exec (but with restrictions)
            // We add a permission statement for backward compatibility
            // Note: This is a simplified version - real implementation would need proper AST construction
            permissions.push(Statement::Permission {
                resource: "sys".to_string(),
                action: "exec".to_string(),
                scope: None, // No scope = allow all (less secure, but backward compatible)
                span: Span::default(),
            });
        }

        permissions
    }
}

/// Deprecation warning
#[derive(Debug, Clone)]
pub struct DeprecationWarning {
    pub feature: String,
    pub replacement: String,
    pub version: Version,
    pub span: Span,
    pub message: String,
}

impl DeprecationWarning {
    pub fn emit(&self, strict_mode: bool) -> Result<(), RuntimeError> {
        let msg = format!(
            "DEPRECATED: '{}' is deprecated in v{}. Use '{}' instead. {}",
            self.feature, self.version, self.replacement, self.message
        );

        if strict_mode {
            Err(RuntimeError::new(format!(
                "{} (at line {}, column {})",
                msg, self.span.line, self.span.column
            )))
        } else {
            log_warn(&format!(
                "{} (at line {}, column {})",
                msg, self.span.line, self.span.column
            ));
            Ok(())
        }
    }
}

/// Legacy syntax support - runtime compatibility shims
pub struct LegacySyntaxSupport;

impl LegacySyntaxSupport {
    pub fn new() -> Self {
        Self
    }

    /// Convert legacy exec_allowed flag to permission system
    pub fn migrate_exec_allowed_to_permissions(&self, exec_allowed: bool) -> Vec<Statement> {
        if exec_allowed {
            // Add sys.exec permission with no scope (allows all commands)
            // This is less secure but maintains backward compatibility
            vec![Statement::Permission {
                resource: "sys".to_string(),
                action: "exec".to_string(),
                scope: None, // No scope = allow all (backward compatible)
                span: Span::default(),
            }]
        } else {
            // No permissions granted
            Vec::new()
        }
    }
}

impl Default for LegacySyntaxSupport {
    fn default() -> Self {
        Self::new()
    }
}

/// Migration report
#[derive(Debug, Clone)]
pub struct MigrationReport {
    pub from_version: Version,
    pub to_version: Version,
    pub transformations_applied: Vec<String>,
    pub warnings: Vec<DeprecationWarning>,
    pub errors: Vec<String>,
    /// Regenerated source code after migration (Some only when dry_run=false)
    pub generated_source: Option<String>,
}

impl MigrationReport {
    pub fn new(from_version: Version, to_version: Version) -> Self {
        Self {
            from_version,
            to_version,
            transformations_applied: Vec::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
            generated_source: None,
        }
    }

    pub fn add_transformation(&mut self, transformation: String) {
        self.transformations_applied.push(transformation);
    }

    pub fn add_warning(&mut self, warning: DeprecationWarning) {
        self.warnings.push(warning);
    }

    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

impl std::fmt::Display for MigrationReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Migration Report: {} → {}",
            self.from_version, self.to_version
        )?;

        if !self.transformations_applied.is_empty() {
            writeln!(f, "\nTransformations Applied:")?;
            for transformation in &self.transformations_applied {
                writeln!(f, "  - {}", transformation)?;
            }
        }

        if !self.warnings.is_empty() {
            writeln!(f, "\nWarnings:")?;
            for warning in &self.warnings {
                writeln!(
                    f,
                    "  - {} (line {}, col {})",
                    warning.message, warning.span.line, warning.span.column
                )?;
            }
        }

        if !self.errors.is_empty() {
            writeln!(f, "\nErrors:")?;
            for error in &self.errors {
                writeln!(f, "  - {}", error)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_compatibility() {
        let v1 = Version::new(0, 1, 0);
        let v2 = Version::new(0, 2, 0);

        assert!(matches!(
            v2.is_compatible_with(&v1),
            CompatibilityResult::BackwardCompatible
        ));
        assert!(matches!(
            v1.is_compatible_with(&v2),
            CompatibilityResult::Incompatible { .. }
        ));
    }

    #[test]
    fn test_version_parsing() {
        let v = Version::from_string("0.2.0").unwrap();
        assert_eq!(v.major, 0);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 0);
    }
}
