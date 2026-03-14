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
use crate::runtime::bytecode_vm::BytecodeVM;
use crate::runtime::permissions::PermissionResource;
use crate::runtime::vm::VirtualMachine;
use crate::tools::logger;
use crate::validator::Validator;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

// ── Bytecode execution ────────────────────────────────────────────────────────

/// Execute a pre-compiled `.txtc` bytecode file.
///
/// The Bytecode VM runs the full 6-layer security pipeline (intent, capability,
/// rate limit, permission, audit trail, runtime security) identical to the AST VM.
fn run_compiled_file(
    file: &PathBuf,
    safe_mode: bool,
    allow_exec: bool,
    allow_fs: &[String],
    allow_net: &[String],
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

    vm.execute(&bytecode).map_err(|e| format!("Bytecode runtime error: {}", e))?;
    Ok(())
}

// ── Permission helpers ────────────────────────────────────────────────────────

/// Parse a permission string like "fs.read", "net.connect", "process.exec", "sys.getenv".
fn parse_permission_string(s: &str) -> Option<PermissionResource> {
    PermissionResource::from_string(s).ok()
}

/// Load the active env's allow/deny permission lists and apply them to the VM.
/// Called by run_file and start_repl so that project-level env.toml is enforced.
/// Same as `apply_env_permissions` but for the Bytecode VM.
/// Both VMs have identical grant/deny APIs; a shared trait is not yet available.
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
pub fn apply_cli_allowlists(vm: &mut VirtualMachine, allow_fs: &[String], allow_net: &[String]) {
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
}

// ── Core run ─────────────────────────────────────────────────────────────────

pub fn run_file(
    file: &PathBuf,
    safe_mode: bool,
    allow_exec: bool,
    debug: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    run_file_inner(file, safe_mode, allow_exec, debug, verbose, &[], &[], None)
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
    cancel_flag: Option<Arc<AtomicBool>>,
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
                logger::log_info(&format!("Running compiled bytecode: {}", file.display()));
                return run_compiled_file(file, safe_mode, allow_exec, allow_fs, allow_net, cancel_flag);
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
    let exec_allowed = if allow_exec { true } else { !effective_safe_mode };

    let mut vm = VirtualMachine::with_all_options(effective_safe_mode, debug, verbose);
    vm.set_exec_allowed(exec_allowed);

    // Attach cancellation flag so timeout can stop execution mid-run.
    if let Some(flag) = cancel_flag {
        vm.set_cancel_flag(flag);
    }

    // Activate source integrity checking: hash the source bytes so the security
    // layer can detect in-memory tampering and upgrades level to Full on Linux.
    vm.runtime_security.hash_and_set_source(source.as_bytes());

    apply_env_permissions(&mut vm);
    apply_cli_allowlists(&mut vm, allow_fs, allow_net);

    vm.interpret(&program)
        .map_err(|e| format!("Runtime error: {}", e))?;

    Ok(())
}

/// Run a file with optional filesystem/network path allowlists.
pub fn run_file_with_allowlists(
    file: &PathBuf,
    safe_mode: bool,
    allow_exec: bool,
    debug: bool,
    verbose: bool,
    allow_fs: &[String],
    allow_net: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    if allow_fs.is_empty() && allow_net.is_empty() {
        return run_file(file, safe_mode, allow_exec, debug, verbose);
    }
    run_file_inner(
        file, safe_mode, allow_exec, debug, verbose, allow_fs, allow_net, None,
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
    let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();

    std::thread::spawn(move || {
        let result = run_file_inner(
            &file, safe_mode, allow_exec, debug, verbose,
            &allow_fs, &allow_net,
            Some(cancel_flag_worker),
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
        file, safe_mode, allow_exec, debug, verbose, &allow_fs, &allow_net,
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
                file, safe_mode, allow_exec, debug, verbose, &allow_fs, &allow_net,
            );
            prev_mtime = cur;
        }
    }
}

// ── .env file loader ──────────────────────────────────────────────────────────

/// Parse a .env file (KEY=VALUE lines, # comments, blank lines ignored)
/// and set each key into the process environment.
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
            if !key.is_empty() {
                std::env::set_var(key, val);
            }
        }
    }
    Ok(())
}
