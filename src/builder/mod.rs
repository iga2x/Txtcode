//! Builder — single authoritative pipeline owner.
//!
//! All language execution flows through here:
//!
//! ```text
//! source → Lexer → Parser → AST → Validator → TypeChecker → [IR] → Backend → Runtime
//! ```
//!
//! CLI commands (`run`, `check`, `build`) must construct a [`BuildConfig`] and
//! delegate to [`Builder`]. They must not import Lexer, Parser, Validator,
//! TypeChecker, or any VM directly.
//!
//! ## Current status
//!
//! The builder is wired and functional. The CLI cutover (removing pipeline
//! imports from `src/cli/run.rs`, `src/cli/compile.rs`, `src/cli/check.rs`)
//! is tracked in `docs/dev-plan.md` Group W.

pub mod project;

use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::parser::ast::statements::Program;
use crate::runtime::core::value::Value;
use crate::typecheck::TypeChecker;
use crate::validator::Validator;
use project::ProjectConfig;
use std::path::PathBuf;

use crate::compiler::bytecode::{Bytecode, BytecodeCompiler};

// ── Public types ──────────────────────────────────────────────────────────────

/// Which execution / compilation backend to use.
#[derive(Debug, Clone, PartialEq)]
pub enum BuildTarget {
    /// AST-walking interpreter (current default).
    Ast,
    /// Bytecode compiler + VM.
    Bytecode,
    /// Bytecode → WAT text output.
    WasmText,
    /// Bytecode → binary `.wasm` output (requires `wasm` feature).
    #[cfg(feature = "wasm")]
    WasmBinary,
}

/// Configuration passed from CLI to the builder for every operation.
#[derive(Debug, Clone)]
pub struct BuildConfig {
    /// Path to a `.tc` source file or a project directory containing `txtcode.toml`.
    pub input: PathBuf,
    /// Output artifact path for `build` operations. `None` = derive from input.
    pub output: Option<PathBuf>,
    /// Target backend / execution engine.
    pub target: BuildTarget,
    /// Run the AST optimizer before compilation (bytecode targets only).
    pub optimize: bool,
    /// Run type checker as part of the pipeline.
    pub type_check: bool,
    /// Halt on the first type error instead of warning (requires `type_check`).
    pub strict_types: bool,
    /// Enable safe mode (seccomp/sandbox + deny exec).
    pub safe_mode: bool,
    /// Emit debug output.
    pub debug: bool,
    /// Emit verbose output.
    pub verbose: bool,
    /// Permit `exec`/`spawn` syscalls (overridden to false when `safe_mode`).
    pub allow_exec: bool,

    // ── CLI-facing execution options ─────────────────────────────────────────
    /// CLI `--allow-fs` path prefixes (grant fs.read + fs.write with scope).
    pub allow_fs: Vec<String>,
    /// CLI `--allow-net` hostnames (grant net.connect with scope).
    pub allow_net: Vec<String>,
    /// CLI `--allow-ffi` library paths (grant sys.ffi with scope).
    pub allow_ffi: Vec<String>,
    /// Cancellation flag set by the timeout runner; VM checks it each statement.
    pub cancel_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    /// Write the audit trail JSON to this path after execution.
    pub audit_log: Option<PathBuf>,
    /// Suppress auto-audit-log even when `safe_mode` is active (`--no-audit-log`).
    pub no_audit_log: bool,
    /// Apply AST obfuscation before validation/execution (from compiler config).
    pub obfuscate: bool,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            input: PathBuf::from("."),
            output: None,
            target: BuildTarget::Ast,
            optimize: false,
            type_check: true,
            strict_types: false,
            safe_mode: false,
            debug: false,
            verbose: false,
            allow_exec: false,
            allow_fs: Vec::new(),
            allow_net: Vec::new(),
            allow_ffi: Vec::new(),
            cancel_flag: None,
            audit_log: None,
            no_audit_log: false,
            obfuscate: false,
        }
    }
}

/// Diagnostics produced by `Builder::check`.
#[derive(Debug, Default)]
pub struct CheckReport {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl CheckReport {
    pub fn is_clean(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Artifact metadata produced by `Builder::build`.
#[derive(Debug)]
pub struct BuildOutput {
    pub entry_path: PathBuf,
    pub target: BuildTarget,
    /// Path to the written artifact, if any.
    pub artifact_path: Option<PathBuf>,
}

// ── Builder ───────────────────────────────────────────────────────────────────

pub struct Builder;

impl Builder {
    /// Parse + validate + type-check without executing or emitting artifacts.
    ///
    /// Returns a [`CheckReport`] with all collected errors and warnings.
    /// The report is always returned (never `Err`); callers decide whether to
    /// treat warnings as errors.
    pub fn check(config: &BuildConfig) -> Result<CheckReport, String> {
        let entry = ProjectConfig::resolve_entry(&config.input)?;
        let source = std::fs::read_to_string(&entry)
            .map_err(|e| format!("Cannot read '{}': {}", entry.display(), e))?;

        let mut report = CheckReport::default();

        let program = match parse_source(&source) {
            Ok(p) => p,
            Err(e) => {
                report.errors.push(e);
                return Ok(report);
            }
        };

        if let Err(e) = Validator::validate_program(&program) {
            report.errors.push(format!("Validation error: {}", e));
            return Ok(report);
        }

        if config.type_check {
            let mut checker = TypeChecker::new();
            match checker.check(&program) {
                Ok(()) => {}
                Err(errs) => {
                    if config.strict_types {
                        report.errors.extend(errs);
                    } else {
                        report.warnings.extend(errs);
                    }
                }
            }
        }

        Ok(report)
    }

    /// Full pipeline up to artifact emission. Does not execute.
    ///
    /// For `BuildTarget::Ast` this is equivalent to `check` (no artifact is
    /// produced for a non-compiled target; callers should use `run` instead).
    pub fn build(config: &BuildConfig) -> Result<BuildOutput, String> {
        let entry = ProjectConfig::resolve_entry(&config.input)?;
        let source = std::fs::read_to_string(&entry)
            .map_err(|e| format!("Cannot read '{}': {}", entry.display(), e))?;

        let mut program = parse_source(&source)?;
        validate_and_typecheck(&program, config)?;

        if config.optimize {
            use crate::compiler::optimizer::{OptimizationLevel, Optimizer};
            Optimizer::new(OptimizationLevel::Basic).optimize_ast(&mut program);
        }

        let bytecode = lower_to_bytecode(&program)?;
        let artifact_path = emit_artifact(&bytecode, &program, &entry, config)?;

        Ok(BuildOutput {
            entry_path: entry,
            target: config.target.clone(),
            artifact_path: Some(artifact_path),
        })
    }


    /// Create a properly configured `VirtualMachine` for REPL use.
    ///
    /// The REPL maintains the VM across multiple input lines (persistent state),
    /// so it cannot use `Builder::run()` directly.  This factory ensures the VM
    /// is configured identically to what `run()` would produce — same safe_mode,
    /// exec_allowed, strict_types, and env permissions.
    pub fn create_repl_vm(config: &BuildConfig) -> crate::runtime::vm::VirtualMachine {
        let effective_safe_mode = config.safe_mode || load_env_safe_mode();
        let exec_allowed = config.allow_exec && !effective_safe_mode;
        let mut vm = crate::runtime::vm::VirtualMachine::with_all_options(
            effective_safe_mode,
            config.debug,
            config.verbose,
        );
        vm.set_exec_allowed(exec_allowed);
        vm.set_strict_types(config.strict_types);
        apply_env_permissions_to_vm(&mut vm);
        if let Some(flag) = &config.cancel_flag {
            vm.set_cancel_flag(std::sync::Arc::clone(flag));
        }
        vm
    }

    /// Parse and validate a source file without executing it.
    ///
    /// Returns the validated [`Program`] ready for interpretation. Applies the
    /// same Lex → Parse → Validate pipeline used by `run()` and `check()`.
    ///
    /// Use this when you need to load code into an existing VM (e.g. REPL `:load`)
    /// rather than running through the full pipeline including a new VM.
    pub fn load_and_validate(path: &std::path::Path) -> Result<Program, String> {
        let source = std::fs::read_to_string(path)
            .map_err(|e| format!("Cannot read '{}': {}", path.display(), e))?;
        let program = parse_source(&source)?;
        Validator::validate_program(&program)
            .map_err(|e| format!("Validation error: {}", e))?;
        Ok(program)
    }

    /// Create a standalone `VirtualMachine` configured from `config`.
    ///
    /// Use this instead of `VirtualMachine::new()` directly at call sites that
    /// bypass the full pipeline (bench, embed). Centralises VM construction so
    /// all paths share the same permission/flag wiring.
    pub fn create_vm(config: &BuildConfig) -> crate::runtime::vm::VirtualMachine {
        let effective_safe_mode = config.safe_mode || load_env_safe_mode();
        let exec_allowed = config.allow_exec && !effective_safe_mode;
        let mut vm = crate::runtime::vm::VirtualMachine::with_all_options(
            effective_safe_mode,
            config.debug,
            config.verbose,
        );
        vm.set_exec_allowed(exec_allowed);
        vm.set_strict_types(config.strict_types);
        apply_env_permissions_to_vm(&mut vm);
        if let Some(flag) = &config.cancel_flag {
            vm.set_cancel_flag(std::sync::Arc::clone(flag));
        }
        vm
    }

    /// Full pipeline including execution. Returns the final `Value`.
    pub fn run(config: &BuildConfig) -> Result<Value, String> {
        let entry = ProjectConfig::resolve_entry(&config.input)?;
        let source = std::fs::read_to_string(&entry)
            .map_err(|e| format!("Cannot read '{}': {}", entry.display(), e))?;

        let program = parse_source(&source)?;

        // Type check before obfuscation (on the human-readable AST).
        run_type_check(&program, config)?;

        // Obfuscate if the compiler config requests it.
        let mut program = if config.obfuscate {
            use crate::security::obfuscator::Obfuscator;
            Obfuscator::new().obfuscate(&program)
        } else {
            program
        };

        // Semantic + restriction validation (on the final, potentially obfuscated AST).
        Validator::validate_program(&program)
            .map_err(|e| format!("Validation error: {}", e))?;

        // Apply constant folding and dead-branch elimination to the AST.
        // IrBuilder::apply_to_ast() mutates the program in place so the AST VM
        // immediately executes the folded code. No feature flag — always active.
        {
            use crate::ir::IrBuilder;
            let mut ir_builder = IrBuilder::new();
            ir_builder.apply_to_ast(&mut program);
            if config.verbose {
                eprintln!(
                    "[ir] {} constant folds; {} dead branches eliminated",
                    ir_builder.fold_count(),
                    ir_builder.dead_branch_count(),
                );
            }
        }

        execute(program, &source, config)
    }
}

// ── Internal pipeline stages ──────────────────────────────────────────────────

/// Stage 1+2+3: source → tokens → `Program`.
fn parse_source(source: &str) -> Result<Program, String> {
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize().map_err(|e| format!("Lex error: {}", e))?;
    let mut parser = Parser::new(tokens);
    parser.parse().map_err(|e| format!("Parse error: {}", e))
}

/// Stage 4+5: validator + optional type checker (used by `check` and `build`).
fn validate_and_typecheck(program: &Program, config: &BuildConfig) -> Result<(), String> {
    Validator::validate_program(program)
        .map_err(|e| format!("Validation error: {}", e))?;
    run_type_check(program, config)
}

/// Stage 5 only: optional type checker (used by `run` before obfuscation).
fn run_type_check(program: &Program, config: &BuildConfig) -> Result<(), String> {
    if config.type_check {
        let mut checker = TypeChecker::new();
        if config.strict_types {
            checker.check_strict(program)
                .map_err(|e| format!("Type error: {}", e))?;
        } else if let Err(errs) = checker.check(program) {
            let mut has_critical = false;
            for e in &errs {
                if is_critical_type_error(e) {
                    eprintln!("[ERROR] type: {}", e);
                    has_critical = true;
                } else {
                    eprintln!("[WARNING] type: {}", e);
                }
            }
            if has_critical {
                return Err(
                    "Critical type error(s) found; use --strict-types to make all type errors fatal".to_string()
                );
            }
        }
    }
    Ok(())
}

/// Returns true for type errors that indicate definite bugs (always fatal in advisory mode).
///
/// Covers: return-type mismatches and null-arithmetic dereferences — both indicate
/// code that will definitely error at runtime.
fn is_critical_type_error(e: &str) -> bool {
    e.contains("Return type mismatch")
        || e.contains("Return type error")
        || e.contains("null dereference in arithmetic")
}

/// Stage 6: lower AST → `Bytecode`.
fn lower_to_bytecode(program: &Program) -> Result<Bytecode, String> {
    let mut compiler = BytecodeCompiler::new();
    Ok(compiler.compile(program))
}

/// Stage 7 (bytecode feature): write artifact to disk.
///
/// `program` is the already-parsed (and optionally obfuscated) program.
/// It is used by the WasmText+ir path to avoid a TOCTOU re-read of the source file.
fn emit_artifact(
    bytecode: &Bytecode,
    _program: &Program,
    entry: &std::path::Path,
    config: &BuildConfig,
) -> Result<PathBuf, String> {
    match &config.target {
        BuildTarget::Bytecode => {
            let out = config.output.clone()
                .unwrap_or_else(|| entry.with_extension("txtc"));
            let bytes = bincode::serialize(bytecode)
                .map_err(|e| format!("Serialization error: {}", e))?;
            std::fs::write(&out, bytes)
                .map_err(|e| format!("Write error: {}", e))?;
            Ok(out)
        }
        BuildTarget::WasmText => {
            let out = config.output.clone()
                .unwrap_or_else(|| entry.with_extension("wat"));
            let mut wasm = crate::compiler::wasm::WasmCompiler::new();
            // S.3: prefer the IR-based backend when `ir` feature is enabled.
            // IR maps structured control flow directly to WAT structured blocks.
            // Use the already-parsed `program` — avoids TOCTOU re-read and
            // preserves any obfuscation applied before this stage.
            #[cfg(feature = "ir")]
            let wat = {
                use crate::ir::IrBuilder;
                let ir = IrBuilder::new().lower(program);
                wasm.compile_from_ir(&ir)
            };
            #[cfg(not(feature = "ir"))]
            let wat = wasm.compile(bytecode);
            std::fs::write(&out, wat)
                .map_err(|e| format!("Write error: {}", e))?;
            Ok(out)
        }
        #[cfg(feature = "wasm")]
        BuildTarget::WasmBinary => {
            let out = config.output.clone()
                .unwrap_or_else(|| entry.with_extension("wasm"));
            let bytes = crate::compiler::wasm_binary::compile_to_binary(bytecode)
                .map_err(|e| format!("WASM binary error: {}", e))?;
            std::fs::write(&out, bytes)
                .map_err(|e| format!("Write error: {}", e))?;
            Ok(out)
        }
        BuildTarget::Ast => {
            // Ast target has no artifact; caller should use run() instead.
            Err("BuildTarget::Ast does not produce a file artifact. Use Builder::run().".to_string())
        }
    }
}

/// Stage 8+9: execute via the appropriate runtime.
fn execute(program: Program, source: &str, config: &BuildConfig) -> Result<Value, String> {
    use crate::runtime::permissions::PermissionResource;
    use crate::runtime::vm::VirtualMachine;

    let effective_safe_mode = config.safe_mode || load_env_safe_mode();
    let exec_allowed = config.allow_exec && !effective_safe_mode;

    if effective_safe_mode {
        if let Err(e) = crate::runtime::sandbox::apply_sandbox(true) {
            eprintln!("[WARNING] OS sandbox could not be applied: {}", e);
        }
    }

    let mut vm = VirtualMachine::with_all_options(effective_safe_mode, config.debug, config.verbose);
    vm.set_exec_allowed(exec_allowed);
    vm.set_strict_types(config.strict_types);
    vm.runtime_security.hash_and_set_source(source.as_bytes());

    apply_env_permissions_to_vm(&mut vm);

    // Apply CLI --allow-fs / --allow-net / --allow-ffi allowlists.
    for path in &config.allow_fs {
        let scope = if path.ends_with('/') || path.ends_with('*') {
            format!("{}*", path.trim_end_matches(['/', '*']))
        } else {
            format!("{}/*", path)
        };
        vm.grant_permission(PermissionResource::FileSystem("read".to_string()), Some(scope.clone()));
        vm.grant_permission(PermissionResource::FileSystem("write".to_string()), Some(scope));
    }
    for host in &config.allow_net {
        vm.grant_permission(PermissionResource::Network("connect".to_string()), Some(host.clone()));
    }
    for lib_path in &config.allow_ffi {
        vm.grant_permission(PermissionResource::System("ffi".to_string()), Some(lib_path.clone()));
    }

    // Wire cancellation flag (used by timeout runner).
    if let Some(flag) = &config.cancel_flag {
        vm.set_cancel_flag(std::sync::Arc::clone(flag));
    }

    let result = vm.interpret(&program)
        .map_err(|e| format!("Runtime error: {}", e));

    // Write audit trail.
    // Explicit audit_log takes priority. In safe_mode with no explicit path (and
    // no_audit_log not set), auto-write to ~/.txtcode/audit/{ts}_{pid}.json.
    let effective_log: Option<PathBuf> = if config.no_audit_log {
        config.audit_log.clone()
    } else {
        config.audit_log.clone().or_else(|| {
            if effective_safe_mode {
                auto_audit_log_path()
            } else {
                None
            }
        })
    };
    if let Some(log_path) = effective_log {
        let json = vm.export_audit_trail_json();
        if let Err(e) = std::fs::write(&log_path, &json) {
            eprintln!("Warning: could not write audit log to '{}': {}", log_path.display(), e);
        } else if config.audit_log.is_none() {
            eprintln!("[audit] Log written to {}", log_path.display());
        }
    }

    result
}

/// Build the auto-audit-log path: `~/.txtcode/audit/{timestamp}_{pid}.json`.
/// Returns `None` if the home directory is unavailable or the dir cannot be created.
fn auto_audit_log_path() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    let dir = home.join(".txtcode").join("audit");
    std::fs::create_dir_all(&dir).ok()?;

    // Clean up audit logs older than 30 days (best-effort; ignore errors).
    if let Ok(entries) = std::fs::read_dir(&dir) {
        let cutoff = std::time::SystemTime::now()
            .checked_sub(std::time::Duration::from_secs(30 * 24 * 3600))
            .unwrap_or(std::time::UNIX_EPOCH);
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if let Ok(modified) = meta.modified() {
                    if modified < cutoff {
                        let _ = std::fs::remove_file(entry.path());
                    }
                }
            }
        }
    }

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let pid = std::process::id();
    Some(dir.join(format!("{}_{}.json", ts, pid)))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn load_env_safe_mode() -> bool {
    crate::config::Config::load_active_env()
        .map(|(_, _, cfg)| cfg.permissions.safe_mode)
        .unwrap_or(false)
}

fn apply_env_permissions_to_vm(vm: &mut crate::runtime::vm::VirtualMachine) {
    use crate::runtime::permissions::PermissionResource;

    let Some((_, _, env_cfg)) = crate::config::Config::load_active_env() else {
        return;
    };

    for perm_str in &env_cfg.permissions.allow {
        if let Ok(resource) = PermissionResource::from_string(perm_str) {
            vm.grant_permission(resource, None);
        }
    }
    for perm_str in &env_cfg.permissions.deny {
        if let Ok(resource) = PermissionResource::from_string(perm_str) {
            vm.deny_permission(resource, None);
        }
    }
}
