//! `txtcode run` — file execution, watch mode, timeout, env loading, permission helpers.
//!
//! This module is a thin CLI shell. All pipeline stages (lex → parse → type-check →
//! validate → execute) are delegated to [`crate::builder::Builder`].  This file
//! owns only CLI-specific concerns:
//!   - File extension / size guards
//!   - `.txtc` bytecode routing (bytecode feature)
//!   - Timeout runner (spawns a thread + sets cancellation flag)
//!   - Watch mode loop
//!   - `--permissions-report` (parse-only scan, exits before execution)
//!   - `.env` file loading
//!   - Audit trail path helpers (kept here for auto-path generation)

use crate::builder::{BuildConfig, Builder};
use crate::runtime::permissions::PermissionResource;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

use crate::runtime::bytecode_vm::BytecodeVM;
use crate::config::Config;

// ── Bytecode execution ────────────────────────────────────────────────────────

/// Execute a pre-compiled `.txtc` bytecode file.
fn run_compiled_file(
    file: &PathBuf,
    safe_mode: bool,
    _allow_exec: bool,
    allow_fs: &[String],
    allow_net: &[String],
    allow_ffi: &[String],
    cancel_flag: Option<Arc<AtomicBool>>,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::compiler::bytecode::Bytecode;

    let data = fs::read(file)?;
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

    if let Some(flag) = cancel_flag {
        vm.set_cancel_flag(flag);
    }
    vm.runtime_security.hash_and_set_source(&data);

    if effective_safe_mode {
        vm.deny_permission(PermissionResource::System("exec".to_string()), None);
    }

    apply_env_permissions_bytecode(&mut vm);

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

fn parse_permission_string(s: &str) -> Option<PermissionResource> {
    PermissionResource::from_string(s).ok()
}

fn apply_env_permissions_bytecode(vm: &mut BytecodeVM) {
    if let Some((_env_dir, _name, cfg)) = crate::config::Config::load_active_env() {
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

/// Called by `txtcode.rs` to apply active-env permissions to the AST VM.
///
/// The AST VM path now goes through [`Builder::run`] which calls
/// `apply_env_permissions_to_vm` internally, so this function is only kept
/// for external callers (e.g. the test runner and REPL).
pub fn apply_env_permissions(vm: &mut crate::runtime::vm::VirtualMachine) {
    if let Some((_env_dir, _name, cfg)) = crate::config::Config::load_active_env() {
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

/// Grant scoped permissions from CLI `--allow-fs` / `--allow-net` flags.
pub fn apply_cli_allowlists(
    vm: &mut crate::runtime::vm::VirtualMachine,
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
        vm.grant_permission(PermissionResource::FileSystem("read".to_string()), Some(scope.clone()));
        vm.grant_permission(PermissionResource::FileSystem("write".to_string()), Some(scope));
    }
    for host in allow_net {
        vm.grant_permission(PermissionResource::Network("connect".to_string()), Some(host.clone()));
    }
    for lib_path in allow_ffi {
        vm.grant_permission(PermissionResource::System("ffi".to_string()), Some(lib_path.clone()));
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
    use crate::tools::logger;
    logger::log_info(&format!("Running file: {}", file.display()));

    if file.is_dir() {
        return Err(format!(
            "'{}' is a directory, not a file.\n  To run tests: txtcode test {}\n  To run a file: txtcode src/main.tc",
            file.display(), file.display()
        ).into());
    }
    if !file.exists() {
        return Err(format!("File '{}' not found", file.display()).into());
    }

    if let Some(ext) = file.extension().and_then(|e| e.to_str()) {
        match ext {
            "tc" => {}
            "txtc" => {
                logger::log_info(&format!("Running compiled bytecode: {}", file.display()));
                return run_compiled_file(file, safe_mode, allow_exec, allow_fs, allow_net, allow_ffi, cancel_flag);
            }
            "txt" => {
                return Err(format!(
                    "'{}' has a .txt extension which is a plain text file.\n  Txt-code source files use the .tc extension.",
                    file.display()
                ).into());
            }
            "rs" | "py" | "js" | "ts" | "go" | "rb" | "java" | "c" | "cpp" => {
                return Err(format!(
                    "'{}' is a {} file, not a Txt-code file.\n  Txt-code source files use the .tc extension.",
                    file.display(), ext
                ).into());
            }
            _ => {}
        }
    }

    const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;
    let metadata = fs::metadata(file)?;
    if metadata.len() > MAX_FILE_SIZE {
        return Err(format!(
            "File '{}' is too large ({} bytes, max 10MB)", file.display(), metadata.len()
        ).into());
    }

    let obfuscate = crate::config::Config::load_config()
        .map(|cfg| cfg.compiler.obfuscate)
        .unwrap_or(false);

    let config = BuildConfig {
        input: file.clone(),
        safe_mode,
        allow_exec,
        debug,
        verbose,
        type_check: !no_type_check,
        strict_types,
        allow_fs: allow_fs.to_vec(),
        allow_net: allow_net.to_vec(),
        allow_ffi: allow_ffi.to_vec(),
        cancel_flag,
        audit_log: audit_log.map(|p| p.to_path_buf()),
        no_audit_log,
        obfuscate,
        ..Default::default()
    };

    Builder::run(&config).map(|_| ()).map_err(|e| e.into())
}

// ── Permissions report ────────────────────────────────────────────────────────

/// Scan the AST for all privileged stdlib calls and print a permissions report.
/// Called by `txtcode run --permissions-report` before execution.
pub fn print_permissions_report(program: &crate::parser::ast::Program, json: bool) {
    use crate::validator::RestrictionChecker;

    let calls = RestrictionChecker::collect_privileged_calls_pub(&program.statements);

    let mut seen = std::collections::BTreeMap::<String, Vec<String>>::new();
    for call in &calls {
        if let Some(perm) = RestrictionChecker::required_capability_pub(call) {
            seen.entry(perm.to_string()).or_default().push(call.clone());
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
        println!("{:<20} Functions", "Permission");
        println!("{}", "-".repeat(60));
        for (perm, fns) in &seen {
            println!("{:<20} {}", perm, fns.join(", "));
        }
    }
}

// ── .env file loader ──────────────────────────────────────────────────────────

const RESERVED_ENV_KEYS: &[&str] = &[
    "LD_PRELOAD", "LD_AUDIT", "LD_LIBRARY_PATH",
    "DYLD_INSERT_LIBRARIES", "DYLD_FORCE_FLAT_NAMESPACE", "DYLD_LIBRARY_PATH",
    "_FRIDA_AGENT", "FRIDA_TRANSPORT", "FRIDA_LISTEN",
];

pub fn load_env_file(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    if !path.exists() {
        return Err(format!("env-file '{}' not found", path.display()).into());
    }
    let content = fs::read_to_string(path)?;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        if let Some(eq) = line.find('=') {
            let key = line[..eq].trim();
            let mut val = line[eq + 1..].trim();
            if (val.starts_with('"') && val.ends_with('"'))
                || (val.starts_with('\'') && val.ends_with('\''))
            {
                val = &val[1..val.len() - 1];
            }
            if key.is_empty() { continue; }
            if RESERVED_ENV_KEYS.contains(&key) {
                return Err(format!(
                    "Forbidden env key '{}': this key controls the dynamic linker \
                     and cannot be set from a .env file", key
                ).into());
            }
            std::env::set_var(key, val);
        }
    }
    Ok(())
}

// ── Timeout ───────────────────────────────────────────────────────────────────

fn parse_duration(s: &str) -> Option<std::time::Duration> {
    let s = s.trim();
    if let Some(ms) = s.strip_suffix("ms") {
        return ms.parse::<u64>().ok().map(std::time::Duration::from_millis);
    }
    if let Some(m) = s.strip_suffix('m') {
        return m.parse::<u64>().ok().map(|n| std::time::Duration::from_secs(n * 60));
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
        format!("Invalid timeout format '{}'. Use e.g. 30s, 500ms, 2m", timeout_str)
    })?;

    let cancel_flag = Arc::new(AtomicBool::new(false));
    let cancel_flag_worker = Arc::clone(&cancel_flag);

    let file = file.to_path_buf();
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
            false,
        ).map_err(|e| e.to_string());
        let _ = tx.send(result);
    });

    match rx.recv_timeout(duration) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e.into()),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            cancel_flag.store(true, Ordering::Relaxed);
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
    println!("Watching '{}' for changes (Ctrl+C to stop)...\n", file.display());

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

// ── Public run_file variants ──────────────────────────────────────────────────

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
    run_file_inner(file, safe_mode, allow_exec, debug, verbose, allow_fs, allow_net, allow_ffi, None, strict_types, audit_log, no_type_check, no_audit_log)
}
