//! `txtcode run` — file execution, watch mode, timeout, env loading, permission helpers.

use crate::config::Config;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::runtime::permissions::PermissionResource;
use crate::runtime::vm::VirtualMachine;
use crate::tools::logger;
use std::fs;
use std::path::{Path, PathBuf};

// ── Permission helpers ────────────────────────────────────────────────────────

/// Parse a permission string like "fs.read", "net.connect", "process.exec", "sys.getenv".
fn parse_permission_string(s: &str) -> Option<PermissionResource> {
    PermissionResource::from_string(s).ok()
}

/// Load the active env's allow/deny permission lists and apply them to the VM.
/// Called by run_file and start_repl so that project-level env.toml is enforced.
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
    run_file_inner(file, safe_mode, allow_exec, debug, verbose, &[], &[])
}

fn run_file_inner(
    file: &PathBuf,
    safe_mode: bool,
    allow_exec: bool,
    debug: bool,
    verbose: bool,
    allow_fs: &[String],
    allow_net: &[String],
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
            "tc" => {}
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

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().map_err(|e| format!("Lex error: {}", e))?;

    let mut parser = Parser::new(tokens);
    let program = parser.parse().map_err(|e| format!("Parse error: {}", e))?;

    let env_safe_mode = Config::load_active_env()
        .map(|(_, _, cfg)| cfg.permissions.safe_mode)
        .unwrap_or(false);
    let effective_safe_mode = safe_mode || env_safe_mode;
    let exec_allowed = if allow_exec { true } else { !effective_safe_mode };

    let mut vm = VirtualMachine::with_all_options(effective_safe_mode, debug, verbose);
    vm.set_exec_allowed(exec_allowed);

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
        file, safe_mode, allow_exec, debug, verbose, allow_fs, allow_net,
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
) -> Result<(), Box<dyn std::error::Error>> {
    let duration = parse_duration(timeout_str).ok_or_else(|| {
        format!(
            "Invalid timeout format '{}'. Use e.g. 30s, 500ms, 2m",
            timeout_str
        )
    })?;

    let file = file.to_path_buf();
    let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();

    std::thread::spawn(move || {
        let result =
            run_file(&file, safe_mode, allow_exec, debug, verbose).map_err(|e| e.to_string());
        let _ = tx.send(result);
    });

    match rx.recv_timeout(duration) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e.into()),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
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
