//! `txtcode run` — file execution, watch mode, timeout, env loading, permission helpers.
//!
//! Execution routing:
//!   `.tc` source files    → AST VM (VirtualMachine) — full security layers
//!   `.txtc` bytecode files → Bytecode VM (BytecodeVM) — full security layers
//!
//! Both VMs enforce the same 6-layer security pipeline:
//!   intent → capability token → rate limit → permission → audit trail → runtime security

use crate::config::Config;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::runtime::permissions::PermissionResource;
use crate::runtime::vm::VirtualMachine;
use crate::tools::logger;
use crate::typecheck::TypeChecker;
use crate::validator::Validator;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

#[cfg(feature = "bytecode")]
use crate::runtime::bytecode_vm::BytecodeVM;

// ── Bytecode execution ────────────────────────────────────────────────────────

/// Execute a pre-compiled `.txtc` bytecode file.
///
/// The Bytecode VM runs the full 6-layer security pipeline (intent, capability,
/// rate limit, permission, audit trail, runtime security) identical to the AST VM.
#[cfg(feature = "bytecode")]
fn run_compiled_file(
    file: &PathBuf,
    safe_mode: bool,
    allow_exec: bool,
    allow_fs: &[String],
    allow_net: &[String],
    allow_ffi: &[String],
    cancel_flag: Option<Arc<AtomicBool>>,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::compiler::bytecode::Bytecode;

    let data = fs::read(file)?;
    // Try binary (bincode) first, fall back to JSON.
    let bytecode: Bytecode = bincode::deserialize(&data)
        .or_else(|_| {
            let s = std::str::from_utf8(&data)
                .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;
            serde_json::from_str(s).map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))
        })?;

    let env_safe_mode = Config::load_active_env()
        .map(|(_, _, cfg)| cfg.permissions.safe_mode)
        .unwrap_or(false);
    let effective_safe_mode = safe_mode || env_safe_mode;

    let mut vm = BytecodeVM::new();
    vm.set_safe_mode(effective_safe_mode);

    // Attach cancellation flag so timeout can stop execution mid-run.
    if let Some(flag) = cancel_flag {
        vm.set_cancel_flag(flag);
    }

    // Activate bytecode integrity hashing — hash the raw bytecode bytes so the
    // runtime security layer can detect in-memory tampering (mirrors the source
    // hash computed for .tc files in run_file_inner).
    vm.runtime_security.hash_and_set_source(&data);

    // Deny exec unconditionally when safe mode is active — mirrors the AST VM behaviour
    // in VirtualMachine::with_all_options, where safe_mode=true always adds the deny
    // regardless of --allow-exec. The inline `self.safe_mode` guard in the bytecode
    // preflight is a first line of defense; this deny ensures PermissionManager also
    // rejects exec so both layers agree.
    if effective_safe_mode {
        vm.deny_permission(PermissionResource::System("exec".to_string()), None);
    }

    // Apply active env permission grants/denials (shared logic with AST VM path).
    apply_env_permissions_bytecode(&mut vm);

    // Apply CLI --allow-fs / --allow-net allowlists.
    for path in allow_fs {
        let scope = if path.ends_with('/') || path.ends_with('*') {
            format!("{}*", path.trim_end_matches(['/', '*']))
        } else {
            format!("{}/*", path)
        };
        vm.grant_permission(PermissionResource::FileSystem("read".to_string()), Some(scope.clone()));
        vm.grant_permission(PermissionResource::FileSystem("write".to_string()), Some(scope));
    }
    for host in allow_net {
        vm.grant_permission(PermissionResource::Network("connect".to_string()), Some(host.clone()));
    }
    for lib_path in allow_ffi {
        vm.grant_permission(PermissionResource::System("ffi".to_string()), Some(lib_path.clone()));
    }

    vm.execute(&bytecode).map_err(|e| format!("Bytecode runtime error: {}", e))?;
    Ok(())
}

// ── Permission helpers ────────────────────────────────────────────────────────

/// Parse a permission string like "fs.read", "net.connect", "process.exec", "sys.getenv".
fn parse_permission_string(s: &str) -> Option<PermissionResource> {
    PermissionResource::from_string(s).ok()
}

/// Load the active env's allow/deny permission lists and apply them to the Bytecode VM.
/// Called by run_file so that project-level env.toml is enforced.
#[cfg(feature = "bytecode")]
fn apply_env_permissions_bytecode(vm: &mut BytecodeVM) {
    if let Some((_env_dir, _name, cfg)) = Config::load_active_env() {
        for perm_str in &cfg.permissions.allow {
            if let Some(resource) = parse_permission_string(perm_str) {
                vm.grant_permission(resource, None);
            }
        }
        for perm_str in &cfg.permissions.deny {
            if let Some(resource) = parse_permission_string(perm_str) {
                vm.deny_permission(resource, None);
            }
        }
    }
}

pub fn apply_env_permissions(vm: &mut VirtualMachine) {
    if let Some((_env_dir, _name, cfg)) = Config::load_active_env() {
        for perm_str in &cfg.permissions.allow {
            if let Some(resource) = parse_permission_string(perm_str) {
                vm.grant_permission(resource, None);
            }
        }
        for perm_str in &cfg.permissions.deny {
            if let Some(resource) = parse_permission_string(perm_str) {
                vm.deny_permission(resource, None);
            }
        }
    }
}

/// Grant scoped permissions from CLI --allow-fs / --allow-net flags.
///
/// --allow-fs=/tmp  → grants fs.read + fs.write with scope "/tmp/*"
/// --allow-net=api.example.com → grants net.connect with scope "api.example.com"
pub fn apply_cli_allowlists(
    vm: &mut VirtualMachine,
    allow_fs: &[String],
    allow_net: &[String],
    allow_ffi: &[String],
) {
    for path in allow_fs {
        let scope = if path.ends_with('/') || path.ends_with('*') {
            format!("{}*", path.trim_end_matches(['/', '*']))
        } else {
            format!("{}/*", path)
        };
        vm.grant_permission(
            PermissionResource::FileSystem("read".to_string()),
            Some(scope.clone()),
        );
        vm.grant_permission(
            PermissionResource::FileSystem("write".to_string()),
            Some(scope),
        );
    }
    for host in allow_net {
        vm.grant_permission(
            PermissionResource::Network("connect".to_string()),
            Some(host.clone()),
        );
    }
    for lib_path in allow_ffi {
        vm.grant_permission(
            PermissionResource::System("ffi".to_string()),
            Some(lib_path.clone()),
        );
    }
}

// ── Core run ─────────────────────────────────────────────────────────────────

pub fn run_file(
    file: &PathBuf,
    safe_mode: bool,
    allow_exec: bool,
    debug: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    run_file_inner(file, safe_mode, allow_exec, debug, verbose, &[], &[], &[], None, false, None, false, false)
}

#[allow(clippy::too_many_arguments)]
fn run_file_inner(
    file: &PathBuf,
    safe_mode: bool,
    allow_exec: bool,
    debug: bool,
    verbose: bool,
    allow_fs: &[String],
    allow_net: &[String],
    allow_ffi: &[String],
    cancel_flag: Option<Arc<AtomicBool>>,
    strict_types: bool,
    audit_log: Option<&std::path::Path>,
    no_type_check: bool,
    no_audit_log: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    logger::log_info(&format!("Running file: {}", file.display()));

    if file.is_dir() {
        return Err(format!(
            "'{}' is a directory, not a file.\n  To run tests: txtcode test {}\n  To run a file: txtcode src/main.tc",
            file.display(),
            file.display()
        )
        .into());
    }

    if !file.exists() {
        return Err(format!("File '{}' not found", file.display()).into());
    }

    if let Some(ext) = file.extension().and_then(|e| e.to_str()) {
        match ext {
            "tc" => {} // source file — continue to AST VM path below
            "txtc" => {
                // Pre-compiled bytecode — route to Bytecode VM and return early.
                #[cfg(feature = "bytecode")]
                {
                    logger::log_info(&format!("Running compiled bytecode: {}", file.display()));
                    return run_compiled_file(file, safe_mode, allow_exec, allow_fs, allow_net, allow_ffi, cancel_flag);
                }
                #[cfg(not(feature = "bytecode"))]
                return Err(
                    "Running pre-compiled .txtc files requires the 'bytecode' feature. \
                     Rebuild with: cargo build --features bytecode"
                        .into(),
                );
            }
            "txt" => {
                return Err(format!(
                    "'{}' has a .txt extension which is a plain text file.\n  Txt-code source files use the .tc extension.",
                    file.display()
                )
                .into());
            }
            "rs" | "py" | "js" | "ts" | "go" | "rb" | "java" | "c" | "cpp" => {
                return Err(format!(
                    "'{}' is a {} file, not a Txt-code file.\n  Txt-code source files use the .tc extension.",
                    file.display(),
                    ext
                )
                .into());
            }
            _ => {}
        }
    }

    const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;
    let metadata = fs::metadata(file)?;
    if metadata.len() > MAX_FILE_SIZE {
        return Err(format!(
            "File '{}' is too large ({} bytes, max 10MB)",
            file.display(),
            metadata.len()
        )
        .into());
    }

    let source = fs::read_to_string(file)?;

    let mut lexer = Lexer::new(source.clone());
    let tokens = lexer.tokenize().map_err(|e| format!("Lex error: {}", e))?;

    let mut parser = Parser::new(tokens);
    let program = parser.parse().map_err(|e| format!("Parse error: {}", e))?;

    // Static type check on .tc source files.
    // K.2: Two modes:
    //   default             — advisory; emits [WARNING] type: but continues execution
    //   --strict-types      — halts with exit code 2 on first type error
    //   --no-type-check     — skip entirely
    if !no_type_check && file.extension().and_then(|e| e.to_str()) == Some("tc") {
        let mut checker = TypeChecker::new();
        if strict_types {
            // Strict mode: halt before execution on first type error
            if let Err(err) = checker.check_strict(&program) {
                eprintln!("[TYPE ERROR] {}", err);
                eprintln!("hint: Fix the type error or run without --strict-types");
                std::process::exit(2);
            }
        } else {
            // Advisory mode: warn but continue
            if let Err(type_errors) = checker.check(&program) {
                for err in &type_errors {
                    eprintln!("[WARNING] type: {}", err);
                }
                eprintln!("hint: Use --strict-types to halt on type errors, --no-type-check to suppress");
            }
        }
    }

    // Apply obfuscation if requested by the user compiler config.
    let program = {
        let should_obfuscate = crate::config::Config::load_config()
            .map(|cfg| cfg.compiler.obfuscate)
            .unwrap_or(false);
        if should_obfuscate {
            use crate::security::obfuscator::Obfuscator;
            Obfuscator::new().obfuscate(&program)
        } else {
            program
        }
    };

    // Validate before execution: syntax rules, semantic checks, security restrictions.
    Validator::validate_program(&program)
        .map_err(|e| format!("Validation error: {}", e))?;

    let env_safe_mode = Config::load_active_env()
        .map(|(_, _, cfg)| cfg.permissions.safe_mode)
        .unwrap_or(false);
    let effective_safe_mode = safe_mode || env_safe_mode;
    // exec requires explicit --allow-exec or in-script grant_permission("sys.exec", null)
    let exec_allowed = allow_exec && !effective_safe_mode;

    // ── OS-level sandbox (Group 25.2) ──────────────────────────────────────────
    // Apply seccomp-BPF + prctl hardening when --sandbox is active.
    // Must be called BEFORE the VM starts interpreting user code.
    if effective_safe_mode {
        if let Err(e) = crate::runtime::sandbox::apply_sandbox(true) {
            // Non-fatal: language-level permissions still apply.
            logger::log_warn(&format!("OS sandbox could not be applied: {}", e));
        }
    }

    let mut vm = VirtualMachine::with_all_options(effective_safe_mode, debug, verbose);
    if exec_allowed {
        vm.set_exec_allowed(true);
    }
    vm.set_strict_types(strict_types);

    // Attach cancellation flag so timeout can stop execution mid-run.
    if let Some(flag) = cancel_flag {
        vm.set_cancel_flag(flag);
    }

    // Activate source integrity checking: hash the source bytes so the security
    // layer can detect in-memory tampering and upgrades level to Full on Linux.
    vm.runtime_security.hash_and_set_source(source.as_bytes());

    apply_env_permissions(&mut vm);
    apply_cli_allowlists(&mut vm, allow_fs, allow_net, allow_ffi);

    let result = vm.interpret(&program)
        .map_err(|e| format!("Runtime error: {}", e));

    // Write audit trail to file.
    // Explicit --audit-log takes priority. When safe_mode is active and no explicit
    // path is given (and --no-audit-log is not set), auto-write to
    // ~/.txtcode/audit/{timestamp}_{pid}.json.
    let effective_log_path: Option<PathBuf> = if no_audit_log {
        audit_log.map(|p| p.to_path_buf())
    } else {
        audit_log.map(|p| p.to_path_buf()).or_else(|| {
            if effective_safe_mode {
                auto_audit_log_path()
            } else {
                None
            }
        })
    };

    if let Some(log_path) = effective_log_path {
        let json = vm.export_audit_trail_json();
        if let Err(e) = fs::write(&log_path, &json) {
            eprintln!("Warning: could not write audit log to '{}': {}", log_path.display(), e);
        } else if audit_log.is_none() {
            // Auto-written in safe mode — print path so users can find it.
            eprintln!("[audit] Log written to {}", log_path.display());
        }
    }

    result?;
    Ok(())
}

/// Build the auto-audit-log path: `~/.txtcode/audit/{timestamp}_{pid}.json`.
/// Returns `None` if the directory cannot be created.
fn auto_audit_log_path() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    let dir = home.join(".txtcode").join("audit");
    fs::create_dir_all(&dir).ok()?;

    // Clean up audit logs older than 30 days (best-effort; ignore errors).
    if let Ok(entries) = fs::read_dir(&dir) {
        let cutoff = std::time::SystemTime::now()
            .checked_sub(std::time::Duration::from_secs(30 * 24 * 3600))
            .unwrap_or(std::time::UNIX_EPOCH);
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if let Ok(modified) = meta.modified() {
                    if modified < cutoff {
                        let _ = fs::remove_file(entry.path());
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

/// Run a file with optional filesystem/network/ffi path allowlists.
pub fn run_file_with_allowlists(
    file: &PathBuf,
    safe_mode: bool,
    allow_exec: bool,
    debug: bool,
    verbose: bool,
    allow_fs: &[String],
    allow_net: &[String],
    allow_ffi: &[String],
    strict_types: bool,
    audit_log: Option<&std::path::Path>,
    no_type_check: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    run_file_with_allowlists_full(file, safe_mode, allow_exec, debug, verbose, allow_fs, allow_net, allow_ffi, strict_types, audit_log, no_type_check, false)
}

pub fn run_file_with_allowlists_full(
    file: &PathBuf,
    safe_mode: bool,
    allow_exec: bool,
    debug: bool,
    verbose: bool,
    allow_fs: &[String],
    allow_net: &[String],
    allow_ffi: &[String],
    strict_types: bool,
    audit_log: Option<&std::path::Path>,
    no_type_check: bool,
    no_audit_log: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if allow_fs.is_empty() && allow_net.is_empty() && allow_ffi.is_empty() {
        return run_file_inner(file, safe_mode, allow_exec, debug, verbose, &[], &[], &[], None, strict_types, audit_log, no_type_check, no_audit_log);
    }
    run_file_inner(
        file, safe_mode, allow_exec, debug, verbose, allow_fs, allow_net, allow_ffi, None, strict_types, audit_log, no_type_check, no_audit_log,
    )
}

// ── Timeout ───────────────────────────────────────────────────────────────────

/// Parse duration strings like "30s", "500ms", "2m" into std::time::Duration.
fn parse_duration(s: &str) -> Option<std::time::Duration> {
    let s = s.trim();
    if let Some(ms) = s.strip_suffix("ms") {
        return ms.parse::<u64>().ok().map(std::time::Duration::from_millis);
    }
    if let Some(m) = s.strip_suffix('m') {
        return m
            .parse::<u64>()
            .ok()
            .map(|n| std::time::Duration::from_secs(n * 60));
    }
    if let Some(sec) = s.strip_suffix('s') {
        return sec.parse::<u64>().ok().map(std::time::Duration::from_secs);
    }
    s.parse::<u64>().ok().map(std::time::Duration::from_secs)
}

pub fn run_file_with_timeout(
    file: &Path,
    safe_mode: bool,
    allow_exec: bool,
    debug: bool,
    verbose: bool,
    timeout_str: &str,
    allow_fs: &[String],
    allow_net: &[String],
    allow_ffi: &[String],
    strict_types: bool,
    audit_log: Option<std::path::PathBuf>,
    no_type_check: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let duration = parse_duration(timeout_str).ok_or_else(|| {
        format!(
            "Invalid timeout format '{}'. Use e.g. 30s, 500ms, 2m",
            timeout_str
        )
    })?;

    // Shared cancellation flag: main thread sets it to `true` when the timeout
    // expires; the worker VM checks it at every statement/instruction boundary
    // and terminates its own execution loop.  This guarantees the thread exits
    // cleanly rather than running forever in the background.
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let cancel_flag_worker = Arc::clone(&cancel_flag);

    let file = file.to_path_buf();
    // Clone allowlists so they can be moved into the worker thread.
    let allow_fs = allow_fs.to_vec();
    let allow_net = allow_net.to_vec();
    let allow_ffi = allow_ffi.to_vec();
    let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();

    std::thread::spawn(move || {
        let result = run_file_inner(
            &file, safe_mode, allow_exec, debug, verbose,
            &allow_fs, &allow_net, &allow_ffi,
            Some(cancel_flag_worker),
            strict_types,
            audit_log.as_deref(),
            no_type_check,
            false, // no_audit_log: timeout runner always allows auto-audit
        )
        .map_err(|e| e.to_string());
        let _ = tx.send(result);
    });

    match rx.recv_timeout(duration) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e.into()),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            // Signal the worker to stop.  It will check the flag at the next
            // statement/instruction boundary and exit its execution loop.
            cancel_flag.store(true, Ordering::Relaxed);
            // Give the thread a brief window to exit cleanly before we return.
            let _ = rx.recv_timeout(std::time::Duration::from_millis(500));
            Err(format!("Execution timed out after {}", timeout_str).into())
        }
        Err(e) => Err(format!("Thread error: {}", e).into()),
    }
}

// ── Watch mode ────────────────────────────────────────────────────────────────

pub fn run_file_watch(
    file: &PathBuf,
    safe_mode: bool,
    allow_exec: bool,
    debug: bool,
    verbose: bool,
    allow_fs: Vec<String>,
    allow_net: Vec<String>,
    allow_ffi: Vec<String>,
) {
    println!(
        "Watching '{}' for changes (Ctrl+C to stop)...\n",
        file.display()
    );

    let get_mtime = |p: &PathBuf| -> Option<std::time::SystemTime> {
        fs::metadata(p).ok().and_then(|m| m.modified().ok())
    };

    let mut prev_mtime = get_mtime(file);
    let _ = run_file_with_allowlists(
        file, safe_mode, allow_exec, debug, verbose, &allow_fs, &allow_net, &allow_ffi, false, None, false,
    );

    loop {
        std::thread::sleep(std::time::Duration::from_secs(2));
        let cur = get_mtime(file);
        let changed = match (prev_mtime, cur) {
            (Some(old), Some(new)) => old != new,
            (None, Some(_)) => true,
            _ => false,
        };
        if changed {
            println!("\n── file changed, re-running ──\n");
            let _ = run_file_with_allowlists(
                file, safe_mode, allow_exec, debug, verbose, &allow_fs, &allow_net, &allow_ffi, false, None, false,
            );
            prev_mtime = cur;
        }
    }
}

// ── Permissions report ────────────────────────────────────────────────────────

/// Scan the AST for all privileged stdlib calls and print a permissions report.
/// With `json=true`, emits a JSON object; otherwise a human-readable table.
/// Called by `txtcode run --permissions-report` before execution.
pub fn print_permissions_report(program: &crate::parser::ast::Program, json: bool) {
    use crate::validator::RestrictionChecker;

    // Re-use the validator's privileged-call collector.
    let calls = RestrictionChecker::collect_privileged_calls_pub(&program.statements);

    // Deduplicate and map each call to its required permission.
    let mut seen = std::collections::BTreeMap::<String, Vec<String>>::new();
    for call in &calls {
        if let Some(perm) = RestrictionChecker::required_capability_pub(call) {
            seen.entry(perm.to_string())
                .or_default()
                .push(call.clone());
        }
    }

    if json {
        let entries: Vec<String> = seen.iter().map(|(perm, fns)| {
            let fn_list = fns.iter().map(|f| format!("\"{}\"", f)).collect::<Vec<_>>().join(", ");
            format!("  \"{}\": [{}]", perm, fn_list)
        }).collect();
        println!("{{\n{}\n}}", entries.join(",\n"));
    } else {
        if seen.is_empty() {
            println!("No privileged permissions required.");
            return;
        }
        println!("Permissions required by this script:");
        println!("{:<20} {}", "Permission", "Functions");
        println!("{}", "-".repeat(60));
        for (perm, fns) in &seen {
            println!("{:<20} {}", perm, fns.join(", "));
        }
    }
}

// ── .env file loader ──────────────────────────────────────────────────────────

/// Env keys that must never be set from a `.env` file — they control the dynamic
/// linker and debugger attachment, allowing a malicious `.env` to execute arbitrary
/// code via library injection before the process even starts a function.
const RESERVED_ENV_KEYS: &[&str] = &[
    "LD_PRELOAD",
    "LD_AUDIT",
    "LD_LIBRARY_PATH",
    "DYLD_INSERT_LIBRARIES",
    "DYLD_FORCE_FLAT_NAMESPACE",
    "DYLD_LIBRARY_PATH",
    "_FRIDA_AGENT",
    "FRIDA_TRANSPORT",
    "FRIDA_LISTEN",
];

/// Parse a .env file (KEY=VALUE lines, # comments, blank lines ignored)
/// and set each key into the process environment.
///
/// Rejects reserved keys that could be used for dynamic-linker injection.
pub fn load_env_file(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    if !path.exists() {
        return Err(format!("env-file '{}' not found", path.display()).into());
    }
    let content = fs::read_to_string(path)?;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(eq) = line.find('=') {
            let key = line[..eq].trim();
            let mut val = line[eq + 1..].trim();
            if (val.starts_with('"') && val.ends_with('"'))
                || (val.starts_with('\'') && val.ends_with('\''))
            {
                val = &val[1..val.len() - 1];
            }
            if key.is_empty() {
                continue;
            }
            // Block dynamic-linker injection keys (0.6)
            if RESERVED_ENV_KEYS.contains(&key) {
                return Err(format!(
                    "Forbidden env key '{}': this key controls the dynamic linker \
                     and cannot be set from a .env file",
                    key
                )
                .into());
            }
            std::env::set_var(key, val);
        }
    }
    Ok(())
}
