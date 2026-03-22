use crate::runtime::{RuntimeError, Value};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::sync::Arc;

lazy_static::lazy_static! {
    static ref CHILD_PROCESSES: Mutex<HashMap<i64, Child>> = Mutex::new(HashMap::new());
    static ref PROCESS_COUNTER: Mutex<i64> = Mutex::new(0);
}

/// System standard library functions
pub struct SysLib;

impl SysLib {
    /// Call a system library function.
    ///
    /// `permission_checker`: Must be `Some(checker)` in all VM-dispatched calls.
    /// Pass `None` only in trusted internal Rust contexts (unit tests, tool executors
    /// that perform their own permission checks upstream).
    pub fn call_function(
        name: &str,
        args: &[Value],
        exec_allowed: bool,
        permission_checker: Option<&dyn crate::stdlib::permission_checker::PermissionChecker>,
    ) -> Result<Value, RuntimeError> {
        #[cfg(debug_assertions)]
        if permission_checker.is_none() {
            crate::tools::logger::log_warn(&format!(
                "stdlib internal: '{}' called without permission_checker — trusted path only", name
            ));
        }
        match name {
            "getenv" => {
                if let Some(Value::String(key)) = args.first() {
                    if let Some(checker) = permission_checker {
                        use crate::runtime::permissions::PermissionResource;
                        checker.check_permission(
                            &PermissionResource::System("env".to_string()),
                            Some(key.as_ref()),
                        )?;
                    }
                    Ok(Value::String(Arc::from(std::env::var(key.as_ref()).unwrap_or_default())))
                } else {
                    Err(RuntimeError::new(
                        "getenv() requires a string argument".to_string(),
                    ))
                }
            }
            "setenv" => {
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(
                        &PermissionResource::System("env".to_string()),
                        None,
                    )?;
                }
                if args.len() >= 2 {
                    if let (Value::String(key), Value::String(val)) = (&args[0], &args[1]) {
                        std::env::set_var(key.as_ref(), val.as_ref());
                        Ok(Value::Null)
                    } else {
                        Err(RuntimeError::new(
                            "setenv() requires two string arguments".to_string(),
                        ))
                    }
                } else {
                    Err(RuntimeError::new(
                        "setenv() requires two arguments".to_string(),
                    ))
                }
            }
            "platform" => {
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(&PermissionResource::System("info".to_string()), None)?;
                }
                Ok(Value::String(Arc::from(std::env::consts::OS.to_string())))
            }
            "arch" => {
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(&PermissionResource::System("info".to_string()), None)?;
                }
                Ok(Value::String(Arc::from(std::env::consts::ARCH.to_string())))
            }
            "exec" => {
                if !exec_allowed {
                    return Err(RuntimeError::new(
                        "exec() is disabled in safe mode".to_string(),
                    ));
                }
                if let Some(Value::String(cmd)) = args.first() {
                    // Split into argv — no shell interpreter is invoked.
                    // Shell meta-characters (pipes, redirects, expansion) are treated
                    // as literals. This prevents shell injection via the command string.
                    let cmd_parts: Vec<&str> = cmd.split_whitespace().collect();
                    let (executable, cmd_args) = match cmd_parts.split_first() {
                        Some(pair) => pair,
                        None => return Err(RuntimeError::new(
                            "exec() requires a non-empty command".to_string(),
                        )),
                    };

                    // Permission scope is the actual executable — accurate now that
                    // there is no shell wrapping the real command.
                    if let Some(checker) = permission_checker {
                        use crate::runtime::permissions::PermissionResource;
                        checker.check_permission(
                            &PermissionResource::System("exec".to_string()),
                            Some(executable),
                        )?;
                    }

                    // Optional second argument: options map {stdin, capture_stderr}
                    let opts = args.get(1).and_then(|v| if let Value::Map(m) = v { Some(m) } else { None });
                    let stdin_input = opts.and_then(|m| m.get("stdin")).and_then(|v| if let Value::String(s) = v { Some(s.clone()) } else { None });
                    let capture_stderr = opts.and_then(|m| m.get("capture_stderr")).map(|v| matches!(v, Value::Boolean(true))).unwrap_or(false);

                    let mut child_cmd = Command::new(executable);
                    child_cmd.args(cmd_args);
                    if stdin_input.is_some() {
                        child_cmd.stdin(Stdio::piped());
                    }
                    if capture_stderr {
                        child_cmd.stderr(Stdio::piped());
                    }

                    let mut child = child_cmd.spawn()
                        .map_err(|e| RuntimeError::new(format!("exec() failed: {}", e)))?;
                    if let Some(input) = stdin_input {
                        if let Some(mut stdin_handle) = child.stdin.take() {
                            let _ = stdin_handle.write_all(input.as_bytes());
                        }
                    }
                    let output = child.wait_with_output()
                        .map_err(|e| RuntimeError::new(format!("exec() failed: {}", e)))?;
                    if capture_stderr {
                        let mut result = IndexMap::new();
                        result.insert("stdout".to_string(), Value::String(Arc::from(String::from_utf8_lossy(&output.stdout).to_string())));
                        result.insert("stderr".to_string(), Value::String(Arc::from(String::from_utf8_lossy(&output.stderr).to_string())));
                        result.insert("status".to_string(), Value::Integer(output.status.code().unwrap_or(0) as i64));
                        Ok(Value::Map(result))
                    } else {
                        Ok(Value::String(Arc::from(String::from_utf8_lossy(&output.stdout).to_string())))
                    }
                } else {
                    Err(RuntimeError::new(
                        "exec() requires a string argument".to_string(),
                    ))
                }
            }
            "exec_pipe" => {
                // exec_pipe(commands) — run a pipeline: ["grep foo", "sort", "uniq"]
                if !exec_allowed {
                    return Err(RuntimeError::new("exec_pipe() is disabled in safe mode".to_string()));
                }
                if args.len() != 1 {
                    return Err(RuntimeError::new("exec_pipe requires 1 argument (array of command strings)".to_string()));
                }
                let commands = match &args[0] {
                    Value::Array(v) => v.clone(),
                    _ => return Err(RuntimeError::new("exec_pipe: argument must be an array of strings".to_string())),
                };
                if commands.is_empty() {
                    return Err(RuntimeError::new("exec_pipe: command list must not be empty".to_string()));
                }
                let mut prev_stdout: Option<std::process::ChildStdout> = None;
                let mut children: Vec<std::process::Child> = Vec::new();
                for cmd_val in &commands {
                    let cmd_str = match cmd_val {
                        Value::String(s) => s.clone(),
                        _ => return Err(RuntimeError::new("exec_pipe: each command must be a string".to_string())),
                    };
                    let cmd_parts: Vec<&str> = cmd_str.split_whitespace().collect();
                    let (exe, c_args) = match cmd_parts.split_first() {
                        Some(pair) => pair,
                        None => return Err(RuntimeError::new("exec_pipe: empty command".to_string())),
                    };
                    if let Some(checker) = permission_checker {
                        use crate::runtime::permissions::PermissionResource;
                        checker.check_permission(&PermissionResource::System("exec".to_string()), Some(exe))?;
                    }
                    let stdin_cfg = if let Some(stdout) = prev_stdout.take() {
                        Stdio::from(stdout)
                    } else {
                        Stdio::null()
                    };
                    let mut child = Command::new(exe)
                        .args(c_args)
                        .stdin(stdin_cfg)
                        .stdout(Stdio::piped())
                        .spawn()
                        .map_err(|e| RuntimeError::new(format!("exec_pipe: failed to spawn '{}': {}", exe, e)))?;
                    prev_stdout = child.stdout.take();
                    children.push(child);
                }
                // Read final stdout
                let output = if let Some(mut stdout) = prev_stdout {
                    use std::io::Read;
                    let mut buf = String::new();
                    stdout.read_to_string(&mut buf).map_err(|e| RuntimeError::new(format!("exec_pipe: read error: {}", e)))?;
                    buf
                } else {
                    String::new()
                };
                for mut child in children {
                    let _ = child.wait();
                }
                Ok(Value::String(Arc::from(output)))
            }
            "exit" => {
                let code = if let Some(Value::Integer(c)) = args.first() {
                    *c as i32
                } else if let Some(Value::Float(f)) = args.first() {
                    *f as i32
                } else {
                    0
                };
                std::process::exit(code);
            }
            "args" => {
                // CLI arguments can contain secrets; gate behind sys.env like getenv (2.6)
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(
                        &PermissionResource::System("env".to_string()),
                        None,
                    )?;
                }
                let args: Vec<Value> = std::env::args()
                    .skip(1) // Skip program name
                    .map(|s| Value::String(Arc::from(s)))
                    .collect();
                Ok(Value::Array(args))
            }
            "cwd" => match std::env::current_dir() {
                Ok(path) => Ok(Value::String(Arc::from(path.to_string_lossy().to_string()))),
                Err(e) => Err(RuntimeError::new(format!(
                    "Failed to get current directory: {}",
                    e
                ))),
            },
            "chdir" => {
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(
                        &PermissionResource::System("env".to_string()),
                        None,
                    )?;
                }
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "chdir requires 1 argument (path)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(path) => {
                        std::env::set_current_dir(path.as_ref()).map_err(|e| {
                            RuntimeError::new(format!("Failed to change directory: {}", e))
                        })?;
                        Ok(Value::Null)
                    }
                    _ => Err(RuntimeError::new(
                        "chdir requires a string path".to_string(),
                    )),
                }
            }
            "pid" => {
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(&PermissionResource::System("info".to_string()), None)?;
                }
                Ok(Value::Integer(std::process::id() as i64))
            }
            "user" => {
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(&PermissionResource::System("info".to_string()), None)?;
                }
                #[cfg(unix)]
                {
                    use std::ffi::CStr;
                    unsafe {
                        let uid = libc::getuid();
                        let passwd = libc::getpwuid(uid);
                        if passwd.is_null() {
                            Ok(Value::String(Arc::from("unknown".to_string())))
                        } else {
                            let name = CStr::from_ptr((*passwd).pw_name)
                                .to_string_lossy()
                                .to_string();
                            Ok(Value::String(Arc::from(name)))
                        }
                    }
                }
                #[cfg(not(unix))]
                {
                    Ok(Value::String(Arc::from(std::env::var("USER").unwrap_or_else(|_| {
                        std::env::var("USERNAME").unwrap_or_else(|_| "unknown".to_string())
                    }))))
                }
            }
            "home" => match dirs::home_dir() {
                Some(path) => Ok(Value::String(Arc::from(path.to_string_lossy().to_string()))),
                None => Err(RuntimeError::new(
                    "Failed to get home directory".to_string(),
                )),
            },
            "uid" => {
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(&PermissionResource::System("info".to_string()), None)?;
                }
                #[cfg(unix)]
                {
                    unsafe { Ok(Value::Integer(libc::getuid() as i64)) }
                }
                #[cfg(not(unix))]
                {
                    Ok(Value::Integer(0))
                }
            }
            "gid" => {
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(&PermissionResource::System("info".to_string()), None)?;
                }
                #[cfg(unix)]
                {
                    unsafe { Ok(Value::Integer(libc::getgid() as i64)) }
                }
                #[cfg(not(unix))]
                {
                    Ok(Value::Integer(0))
                }
            }
            "spawn" => {
                if !exec_allowed {
                    return Err(RuntimeError::new(
                        "spawn() is disabled in safe mode".to_string(),
                    ));
                }
                if args.is_empty() {
                    return Err(RuntimeError::new(
                        "spawn requires at least 1 argument (command)".to_string(),
                    ));
                }

                let cmd_str = match &args[0] {
                    Value::String(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            "spawn command must be a string".to_string(),
                        ))
                    }
                };

                // Split into argv — no shell interpreter is invoked.
                let cmd_parts: Vec<&str> = cmd_str.split_whitespace().collect();
                let (executable, spawn_args) = match cmd_parts.split_first() {
                    Some(pair) => pair,
                    None => return Err(RuntimeError::new(
                        "spawn() requires a non-empty command".to_string(),
                    )),
                };

                // Permission check — previously missing from spawn().
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(
                        &PermissionResource::System("exec".to_string()),
                        Some(executable),
                    )?;
                }

                let mut cmd = Command::new(executable);
                cmd.args(spawn_args);

                // Add environment variables if provided
                if args.len() > 1 {
                    if let Value::Map(env_vars) = &args[1] {
                        for (key, value) in env_vars {
                            if let Value::String(val_str) = value {
                                cmd.env(key, val_str.as_ref());
                            }
                        }
                    }
                }

                let child = cmd
                    .spawn()
                    .map_err(|e| RuntimeError::new(format!("spawn() failed: {}", e)))?;

                let pid = child.id() as i64;

                // Store child process
                let mut processes = CHILD_PROCESSES.lock().unwrap_or_else(|e| e.into_inner());
                processes.insert(pid, child);

                Ok(Value::Integer(pid))
            }
            "kill" => {
                if !exec_allowed {
                    return Err(RuntimeError::new(
                        "kill() is disabled in safe mode".to_string(),
                    ));
                }
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(
                        &PermissionResource::System("exec".to_string()),
                        None,
                    )?;
                }
                if args.is_empty() || args.len() > 2 {
                    return Err(RuntimeError::new(
                        "kill requires 1 or 2 arguments (pid, signal?)".to_string(),
                    ));
                }

                let pid = match &args[0] {
                    Value::Integer(p) => *p,
                    _ => return Err(RuntimeError::new("kill pid must be an integer".to_string())),
                };

                // Try to kill from our managed processes first
                let mut processes = CHILD_PROCESSES.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(mut child) = processes.remove(&pid) {
                    let _ = child.kill();
                    return Ok(Value::Boolean(true));
                }
                drop(processes);

                // Otherwise try to kill by system PID
                #[cfg(unix)]
                {
                    let signal = if args.len() == 2 {
                        match &args[1] {
                            Value::Integer(s) => *s as i32,
                            Value::String(s) => match s.as_ref() {
                                "SIGTERM" | "TERM" => libc::SIGTERM,
                                "SIGKILL" | "KILL" => libc::SIGKILL,
                                "SIGINT" | "INT" => libc::SIGINT,
                                _ => libc::SIGTERM,
                            },
                            _ => libc::SIGTERM,
                        }
                    } else {
                        libc::SIGTERM
                    };

                    unsafe {
                        let result = libc::kill(pid as libc::pid_t, signal);
                        Ok(Value::Boolean(result == 0))
                    }
                }
                #[cfg(not(unix))]
                {
                    // On Windows, we can't easily kill arbitrary PIDs
                    Err(RuntimeError::new(
                        "kill() not fully supported on this platform".to_string(),
                    ))
                }
            }
            "wait" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "wait requires 1 argument (pid)".to_string(),
                    ));
                }

                let pid = match &args[0] {
                    Value::Integer(p) => *p,
                    _ => return Err(RuntimeError::new("wait pid must be an integer".to_string())),
                };

                let mut processes = CHILD_PROCESSES.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(mut child) = processes.remove(&pid) {
                    match child.wait() {
                        Ok(status) => {
                            let exit_code = status.code().unwrap_or(-1);
                            let mut result = IndexMap::new();
                            result.insert("pid".to_string(), Value::Integer(pid));
                            result
                                .insert("exit_code".to_string(), Value::Integer(exit_code as i64));
                            result.insert("success".to_string(), Value::Boolean(status.success()));
                            Ok(Value::Map(result))
                        }
                        Err(e) => Err(RuntimeError::new(format!("wait() failed: {}", e))),
                    }
                } else {
                    drop(processes);
                    Err(RuntimeError::new(format!(
                        "Process {} not found or already waited",
                        pid
                    )))
                }
            }
            "sleep" => {
                let ms = if let Some(Value::Integer(m)) = args.first() {
                    *m as u64
                } else if let Some(Value::Float(f)) = args.first() {
                    *f as u64
                } else {
                    return Err(RuntimeError::new(
                        "sleep() requires a number argument (milliseconds)".to_string(),
                    ));
                };
                std::thread::sleep(std::time::Duration::from_millis(ms));
                Ok(Value::Null)
            }
            "env_list" => {
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(
                        &PermissionResource::System("env".to_string()),
                        None,
                    )?;
                }
                let vars: IndexMap<String, Value> = std::env::vars()
                    .map(|(k, v)| (k, Value::String(Arc::from(v))))
                    .collect();
                Ok(Value::Map(vars))
            }
            "signal_send" => {
                if !exec_allowed {
                    return Err(RuntimeError::new(
                        "signal_send() is disabled in safe mode".to_string(),
                    ));
                }
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(
                        &PermissionResource::System("exec".to_string()),
                        None,
                    )?;
                }
                if args.is_empty() || args.len() > 2 {
                    return Err(RuntimeError::new(
                        "signal_send requires 1-2 arguments (pid, signal?)".to_string(),
                    ));
                }
                let pid = match &args[0] {
                    Value::Integer(p) => *p,
                    _ => {
                        return Err(RuntimeError::new(
                            "signal_send pid must be an integer".to_string(),
                        ))
                    }
                };
                #[cfg(unix)]
                {
                    let signal = if args.len() == 2 {
                        match &args[1] {
                            Value::Integer(s) => *s as i32,
                            Value::String(s) => match s.as_ref() {
                                "SIGTERM" | "TERM" => libc::SIGTERM,
                                "SIGKILL" | "KILL" => libc::SIGKILL,
                                "SIGINT" | "INT" => libc::SIGINT,
                                "SIGHUP" | "HUP" => libc::SIGHUP,
                                "SIGUSR1" | "USR1" => libc::SIGUSR1,
                                "SIGUSR2" | "USR2" => libc::SIGUSR2,
                                other => {
                                    return Err(RuntimeError::new(format!(
                                        "Unknown signal: {}",
                                        other
                                    )))
                                }
                            },
                            _ => libc::SIGTERM,
                        }
                    } else {
                        libc::SIGTERM
                    };
                    unsafe {
                        let result = libc::kill(pid as libc::pid_t, signal);
                        Ok(Value::Boolean(result == 0))
                    }
                }
                #[cfg(not(unix))]
                {
                    let _ = pid;
                    Err(RuntimeError::new(
                        "signal_send is not supported on this platform".to_string(),
                    ))
                }
            }
            "pipe_exec" => {
                if !exec_allowed {
                    return Err(RuntimeError::new(
                        "pipe_exec() is disabled in safe mode".to_string(),
                    ));
                }
                if args.is_empty() || args.len() > 2 {
                    return Err(RuntimeError::new(
                        "pipe_exec requires 1-2 arguments (cmd, stdin_input?)".to_string(),
                    ));
                }
                let cmd_str = match &args[0] {
                    Value::String(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            "pipe_exec command must be a string".to_string(),
                        ))
                    }
                };
                let stdin_input = if args.len() == 2 {
                    match &args[1] {
                        Value::String(s) => Some(s.clone()),
                        _ => None,
                    }
                } else {
                    None
                };
                // Split into argv — no shell interpreter is invoked.
                let cmd_parts: Vec<&str> = cmd_str.split_whitespace().collect();
                let (executable, pipe_args) = match cmd_parts.split_first() {
                    Some(pair) => pair,
                    None => return Err(RuntimeError::new(
                        "pipe_exec() requires a non-empty command".to_string(),
                    )),
                };

                // Permission check — previously missing from pipe_exec().
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(
                        &PermissionResource::System("exec".to_string()),
                        Some(executable),
                    )?;
                }

                use std::io::Write;
                use std::process::Stdio;
                let mut cmd = Command::new(executable);
                cmd.args(pipe_args)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());
                if stdin_input.is_some() {
                    cmd.stdin(Stdio::piped());
                } else {
                    cmd.stdin(Stdio::null());
                }
                let mut child = cmd
                    .spawn()
                    .map_err(|e| RuntimeError::new(format!("pipe_exec failed to start: {}", e)))?;
                if let Some(input) = stdin_input {
                    if let Some(mut stdin) = child.stdin.take() {
                        let _ = stdin.write_all(input.as_bytes());
                    }
                }
                let output = child
                    .wait_with_output()
                    .map_err(|e| RuntimeError::new(format!("pipe_exec failed: {}", e)))?;
                let mut result = IndexMap::new();
                result.insert(
                    "stdout".to_string(),
                    Value::String(Arc::from(String::from_utf8_lossy(&output.stdout).to_string())),
                );
                result.insert(
                    "stderr".to_string(),
                    Value::String(Arc::from(String::from_utf8_lossy(&output.stderr).to_string())),
                );
                result.insert(
                    "exit_code".to_string(),
                    Value::Integer(output.status.code().unwrap_or(-1) as i64),
                );
                result.insert(
                    "success".to_string(),
                    Value::Boolean(output.status.success()),
                );
                Ok(Value::Map(result))
            }
            "which" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "which requires 1 argument (binary)".to_string(),
                    ));
                }
                let binary = match &args[0] {
                    Value::String(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            "which requires a string argument".to_string(),
                        ))
                    }
                };
                let sep = if cfg!(windows) { ';' } else { ':' };
                let path_var = std::env::var("PATH").unwrap_or_default();
                for dir in path_var.split(sep) {
                    let candidate = std::path::Path::new(dir).join(binary.as_ref());
                    if candidate.is_file() {
                        return Ok(Value::String(Arc::from(candidate.to_string_lossy().to_string())));
                    }
                }
                Ok(Value::Null)
            }
            "is_root" => {
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(&PermissionResource::System("info".to_string()), None)?;
                }
                #[cfg(unix)]
                {
                    unsafe { Ok(Value::Boolean(libc::getuid() == 0)) }
                }
                #[cfg(not(unix))]
                {
                    Ok(Value::Boolean(false))
                }
            }
            "cpu_count" => {
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(&PermissionResource::System("info".to_string()), None)?;
                }
                let count = std::thread::available_parallelism()
                    .map(|n| n.get() as i64)
                    .unwrap_or(1);
                Ok(Value::Integer(count))
            }
            "memory_available" => {
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(&PermissionResource::System("info".to_string()), None)?;
                }
                #[cfg(target_os = "linux")]
                {
                    match std::fs::read_to_string("/proc/meminfo") {
                        Ok(content) => {
                            for line in content.lines() {
                                if line.starts_with("MemAvailable:") {
                                    let parts: Vec<&str> = line.split_whitespace().collect();
                                    if parts.len() >= 2 {
                                        if let Ok(kb) = parts[1].parse::<i64>() {
                                            return Ok(Value::Integer(kb * 1024));
                                        }
                                    }
                                }
                            }
                            Ok(Value::Integer(0))
                        }
                        Err(_) => Ok(Value::Integer(0)),
                    }
                }
                #[cfg(not(target_os = "linux"))]
                {
                    Ok(Value::Integer(0))
                }
            }
            "disk_space" => {
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(&PermissionResource::System("info".to_string()), None)?;
                }
                let path = if args.is_empty() {
                    ".".to_string()
                } else {
                    match &args[0] {
                        Value::String(s) => s.to_string(),
                        _ => {
                            return Err(RuntimeError::new(
                                "disk_space path must be a string".to_string(),
                            ))
                        }
                    }
                };
                #[cfg(unix)]
                {
                    use std::ffi::CString;
                    let c_path = CString::new(path.as_str()).map_err(|_| {
                        RuntimeError::new("Invalid path for disk_space".to_string())
                    })?;
                    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
                    let rc = unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) };
                    if rc != 0 {
                        return Err(RuntimeError::new(format!(
                            "disk_space failed for path: {}",
                            path
                        )));
                    }
                    let total = (stat.f_blocks as i64).saturating_mul(stat.f_frsize as i64);
                    let available = (stat.f_bavail as i64).saturating_mul(stat.f_frsize as i64);
                    let mut map = IndexMap::new();
                    map.insert("total".to_string(), Value::Integer(total));
                    map.insert("available".to_string(), Value::Integer(available));
                    map.insert(
                        "used".to_string(),
                        Value::Integer(total.saturating_sub(available)),
                    );
                    Ok(Value::Map(map))
                }
                #[cfg(not(unix))]
                {
                    let _ = path;
                    Err(RuntimeError::new(
                        "disk_space is not supported on this platform".to_string(),
                    ))
                }
            }
            "os_name" => {
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(&PermissionResource::System("info".to_string()), None)?;
                }
                Ok(Value::String(Arc::from(std::env::consts::OS.to_string())))
            }
            "os_version" => {
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(&PermissionResource::System("info".to_string()), None)?;
                }
                // Read kernel/OS version from files — no external process spawn needed (2.1).
                #[cfg(target_os = "linux")]
                {
                    // Try /etc/os-release for pretty distro name first
                    if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
                        for line in content.lines() {
                            if line.starts_with("PRETTY_NAME=") {
                                let val = line.trim_start_matches("PRETTY_NAME=").trim_matches('"');
                                return Ok(Value::String(Arc::from(val.to_string())));
                            }
                        }
                    }
                    // Fallback: read kernel version from /proc/version
                    if let Ok(content) = std::fs::read_to_string("/proc/version") {
                        let version = content.split_whitespace().take(3).collect::<Vec<_>>().join(" ");
                        return Ok(Value::String(Arc::from(version)));
                    }
                    Ok(Value::String(Arc::from("linux/unknown".to_string())))
                }
                #[cfg(target_os = "macos")]
                {
                    // Read macOS version from /System/Library/CoreServices/SystemVersion.plist (XML)
                    if let Ok(content) = std::fs::read_to_string(
                        "/System/Library/CoreServices/SystemVersion.plist",
                    ) {
                        // Simple key/value extraction — no XML parser needed for this flat plist
                        let lines: Vec<&str> = content.lines().collect();
                        for (i, line) in lines.iter().enumerate() {
                            if line.contains("ProductUserVisibleVersion") {
                                if let Some(next) = lines.get(i + 1) {
                                    let ver = next
                                        .trim()
                                        .trim_start_matches("<string>")
                                        .trim_end_matches("</string>");
                                    return Ok(Value::String(Arc::from(format!("macOS {}", ver))));
                                }
                            }
                        }
                    }
                    Ok(Value::String(Arc::from("macOS/unknown".to_string())))
                }
                #[cfg(not(any(target_os = "linux", target_os = "macos")))]
                {
                    Ok(Value::String(Arc::from(std::env::consts::OS.to_string())))
                }
            }
            // ── Subprocess IPC helpers ──────────────────────────────────────────
            //
            // exec_status(cmd)  → integer exit code (0 = success)
            // exec_lines(cmd)   → array of stdout lines (stderr discarded)
            // exec_json(cmd)    → stdout parsed as JSON → Value
            //
            // All three share the same permission as exec(): sys.exec.
            // None of them invoke a shell — the command is split on whitespace
            // and passed directly to the OS, preventing shell injection.

            "exec_status" => {
                if !exec_allowed {
                    return Err(RuntimeError::new(
                        "exec_status() is disabled in safe mode".to_string(),
                    ));
                }
                let cmd_str = match args.first() {
                    Some(Value::String(s)) => s.clone(),
                    _ => return Err(RuntimeError::new("exec_status() requires a string argument".to_string())),
                };
                let parts: Vec<&str> = cmd_str.split_whitespace().collect();
                let (exe, rest) = parts.split_first().ok_or_else(|| {
                    RuntimeError::new("exec_status() requires a non-empty command".to_string())
                })?;
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(&PermissionResource::System("exec".to_string()), Some(exe))?;
                }
                let status = Command::new(exe).args(rest).status()
                    .map_err(|e| RuntimeError::new(format!("exec_status() failed: {}", e)))?;
                Ok(Value::Integer(status.code().unwrap_or(-1) as i64))
            }

            "exec_lines" => {
                if !exec_allowed {
                    return Err(RuntimeError::new(
                        "exec_lines() is disabled in safe mode".to_string(),
                    ));
                }
                let cmd_str = match args.first() {
                    Some(Value::String(s)) => s.clone(),
                    _ => return Err(RuntimeError::new("exec_lines() requires a string argument".to_string())),
                };
                let parts: Vec<&str> = cmd_str.split_whitespace().collect();
                let (exe, rest) = parts.split_first().ok_or_else(|| {
                    RuntimeError::new("exec_lines() requires a non-empty command".to_string())
                })?;
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(&PermissionResource::System("exec".to_string()), Some(exe))?;
                }
                let output = Command::new(exe).args(rest).output()
                    .map_err(|e| RuntimeError::new(format!("exec_lines() failed: {}", e)))?;
                let stdout = String::from_utf8_lossy(&output.stdout);
                let lines: Vec<Value> = stdout
                    .lines()
                    .map(|l| Value::String(Arc::from(l.to_string())))
                    .collect();
                Ok(Value::Array(lines))
            }

            "exec_json" => {
                if !exec_allowed {
                    return Err(RuntimeError::new(
                        "exec_json() is disabled in safe mode".to_string(),
                    ));
                }
                let cmd_str = match args.first() {
                    Some(Value::String(s)) => s.clone(),
                    _ => return Err(RuntimeError::new("exec_json() requires a string argument".to_string())),
                };
                let parts: Vec<&str> = cmd_str.split_whitespace().collect();
                let (exe, rest) = parts.split_first().ok_or_else(|| {
                    RuntimeError::new("exec_json() requires a non-empty command".to_string())
                })?;
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(&PermissionResource::System("exec".to_string()), Some(exe))?;
                }
                let output = Command::new(exe).args(rest).output()
                    .map_err(|e| RuntimeError::new(format!("exec_json() failed: {}", e)))?;
                let stdout = String::from_utf8_lossy(&output.stdout);
                let json_val: serde_json::Value = serde_json::from_str(stdout.trim())
                    .map_err(|e| RuntimeError::new(format!("exec_json() stdout is not valid JSON: {}", e)))?;
                Ok(json_to_value(&json_val))
            }

            // ── Task 17.5: Advanced process control ──────────────────────────

            // proc_run(cmd, options_map?) → {stdout, stderr, status}
            //
            // options_map keys (all optional):
            //   "stdin"   → String fed to the process stdin
            //   "env"     → Map<String,String> extra environment variables
            //   "cwd"     → String working directory
            //   "timeout" → Integer milliseconds before kill (default: no timeout)
            "proc_run" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(RuntimeError::new(
                        "proc_run requires 1-2 arguments (cmd, options?)".to_string(),
                    ));
                }
                // Permission check
                let cmd_val = &args[0];
                let cmd_str = match cmd_val {
                    Value::String(s) => s.clone(),
                    Value::Array(a) => a.first().and_then(|v| if let Value::String(s) = v { Some(s.clone()) } else { None }).unwrap_or_default(),
                    _ => return Err(RuntimeError::new("proc_run: cmd must be a string or array".to_string())),
                };
                if !exec_allowed {
                    return Err(RuntimeError::new(
                        "proc_run: exec is not allowed in safe mode (use --allow-exec flag)".to_string(),
                    ));
                }
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(
                        &PermissionResource::System("exec".to_string()),
                        Some(&cmd_str),
                    )?;
                }
                // Parse options
                let opts: indexmap::IndexMap<String, Value> = if args.len() == 2 {
                    match &args[1] {
                        Value::Map(m) => m.clone(),
                        _ => return Err(RuntimeError::new("proc_run: options must be a map".to_string())),
                    }
                } else {
                    indexmap::IndexMap::new()
                };
                let stdin_data = opts.get("stdin").and_then(|v| if let Value::String(s) = v { Some(s.to_string()) } else { None });
                let cwd = opts.get("cwd").and_then(|v| if let Value::String(s) = v { Some(s.to_string()) } else { None });
                let timeout_ms = opts.get("timeout").and_then(|v| match v {
                    Value::Integer(n) => Some(*n as u64),
                    Value::Float(f) => Some(*f as u64),
                    _ => None,
                });
                let extra_env: Vec<(String, String)> = match opts.get("env") {
                    Some(Value::Map(m)) => m.iter().filter_map(|(k, v)| {
                        if let Value::String(s) = v { Some((k.clone(), s.to_string())) } else { None }
                    }).collect(),
                    _ => vec![],
                };

                // Build command
                let (exe, cmd_args): (String, Vec<String>) = match &args[0] {
                    Value::String(s) => {
                        let parts: Vec<&str> = s.split_whitespace().collect();
                        (parts[0].to_string(), parts[1..].iter().map(|s| s.to_string()).collect())
                    }
                    Value::Array(a) => {
                        let strs: Vec<String> = a.iter().map(|v| match v {
                            Value::String(s) => s.to_string(),
                            other => other.to_string(),
                        }).collect();
                        (strs[0].clone(), strs[1..].to_vec())
                    }
                    _ => unreachable!(),
                };

                let mut child_cmd = std::process::Command::new(&exe);
                child_cmd.args(&cmd_args);
                for (k, v) in &extra_env { child_cmd.env(k, v); }
                if let Some(ref dir) = cwd { child_cmd.current_dir(dir); }
                child_cmd.stdin(std::process::Stdio::piped());
                child_cmd.stdout(std::process::Stdio::piped());
                child_cmd.stderr(std::process::Stdio::piped());

                let mut child = child_cmd.spawn()
                    .map_err(|e| RuntimeError::new(format!("proc_run: failed to spawn '{}': {}", exe, e)))?;

                // Write stdin
                if let Some(ref input) = stdin_data {
                    if let Some(mut stdin_pipe) = child.stdin.take() {
                        use std::io::Write;
                        let _ = stdin_pipe.write_all(input.as_bytes());
                    }
                }

                // Wait with optional timeout
                let output = if let Some(ms) = timeout_ms {
                    use std::time::{Duration, Instant};
                    let deadline = Instant::now() + Duration::from_millis(ms);
                    loop {
                        match child.try_wait() {
                            Ok(Some(_)) => break child.wait_with_output()
                                .map_err(|e| RuntimeError::new(format!("proc_run: {}", e)))?,
                            Ok(None) => {
                                if Instant::now() >= deadline {
                                    let _ = child.kill();
                                    return Err(RuntimeError::new(format!("proc_run: '{}' timed out after {}ms", exe, ms)));
                                }
                                std::thread::sleep(std::time::Duration::from_millis(10));
                            }
                            Err(e) => return Err(RuntimeError::new(format!("proc_run: {}", e))),
                        }
                    }
                } else {
                    child.wait_with_output()
                        .map_err(|e| RuntimeError::new(format!("proc_run: {}", e)))?
                };

                let mut result = indexmap::IndexMap::new();
                result.insert("stdout".to_string(), Value::String(Arc::from(String::from_utf8_lossy(&output.stdout).to_string())));
                result.insert("stderr".to_string(), Value::String(Arc::from(String::from_utf8_lossy(&output.stderr).to_string())));
                result.insert("status".to_string(), Value::Integer(output.status.code().unwrap_or(-1) as i64));
                Ok(Value::Map(result))
            }

            // proc_pipe(commands_array) → {stdout, stderr, status}
            // Each element is either a string or an array of strings.
            // Connects stdout of each command to stdin of the next.
            "proc_pipe" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("proc_pipe requires 1 argument (commands array)".to_string()));
                }
                let cmds = match &args[0] {
                    Value::Array(a) => a.clone(),
                    _ => return Err(RuntimeError::new("proc_pipe: argument must be an array of commands".to_string())),
                };
                if cmds.is_empty() {
                    return Err(RuntimeError::new("proc_pipe: commands array must not be empty".to_string()));
                }
                if !exec_allowed {
                    return Err(RuntimeError::new("proc_pipe: exec is not allowed in safe mode".to_string()));
                }
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(&PermissionResource::System("exec".to_string()), None)?;
                }

                let parse_cmd = |v: &Value| -> (String, Vec<String>) {
                    match v {
                        Value::String(s) => {
                            let parts: Vec<&str> = s.split_whitespace().collect();
                            (parts[0].to_string(), parts[1..].iter().map(|s| s.to_string()).collect())
                        }
                        Value::Array(a) => {
                            let strs: Vec<String> = a.iter().map(|v| match v {
                                Value::String(s) => s.to_string(),
                                other => other.to_string(),
                            }).collect();
                            (strs[0].clone(), strs[1..].to_vec())
                        }
                        other => (other.to_string(), vec![]),
                    }
                };

                let mut prev_stdout: Option<std::process::ChildStdout> = None;
                let mut children: Vec<std::process::Child> = Vec::new();

                for (idx, cmd_val) in cmds.iter().enumerate() {
                    let (exe, cmd_args) = parse_cmd(cmd_val);
                    let mut cmd = std::process::Command::new(&exe);
                    cmd.args(&cmd_args);
                    if let Some(prev) = prev_stdout.take() {
                        cmd.stdin(prev);
                    } else {
                        cmd.stdin(std::process::Stdio::null());
                    }
                    let is_last = idx == cmds.len() - 1;
                    if is_last {
                        cmd.stdout(std::process::Stdio::piped());
                        cmd.stderr(std::process::Stdio::piped());
                    } else {
                        cmd.stdout(std::process::Stdio::piped());
                        cmd.stderr(std::process::Stdio::null());
                    }
                    let mut child = cmd.spawn()
                        .map_err(|e| RuntimeError::new(format!("proc_pipe: failed to spawn '{}': {}", exe, e)))?;
                    if !is_last {
                        prev_stdout = child.stdout.take();
                    }
                    children.push(child);
                }

                let last = children.pop().unwrap();
                // Reap earlier children (they may have already exited once stdout EOF was hit)
                for mut child in children {
                    let _ = child.wait();
                }
                let output = last.wait_with_output()
                    .map_err(|e| RuntimeError::new(format!("proc_pipe: {}", e)))?;

                let mut result = indexmap::IndexMap::new();
                result.insert("stdout".to_string(), Value::String(Arc::from(String::from_utf8_lossy(&output.stdout).to_string())));
                result.insert("stderr".to_string(), Value::String(Arc::from(String::from_utf8_lossy(&output.stderr).to_string())));
                result.insert("status".to_string(), Value::Integer(output.status.code().unwrap_or(-1) as i64));
                Ok(Value::Map(result))
            }

            // ── Task 17.4: CLI argument parsing ──────────────────────────────
            // cli_parse(args_array, spec_map) → result_map
            //
            // spec_map keys:
            //   "flags"    → Array<String>  — boolean flags (--verbose, --dry-run)
            //   "options"  → Array<String>  — value-taking options (--output FILE)
            //   "positionals" → Array<String> — positional arg names (name only, no --)
            //
            // Returns a Map with:
            //   each flag → Boolean
            //   each option → String or Null
            //   each positional → String or Null
            //   "_rest" → Array<String> of unrecognised args
            "cli_parse" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "cli_parse requires 2 arguments (args_array, spec_map)".to_string(),
                    ));
                }
                let raw_args = match &args[0] {
                    Value::Array(a) => a.clone(),
                    _ => return Err(RuntimeError::new("cli_parse: first argument must be an array".to_string())),
                };
                let spec = match &args[1] {
                    Value::Map(m) => m.clone(),
                    _ => return Err(RuntimeError::new("cli_parse: second argument must be a map".to_string())),
                };
                let flags: Vec<String> = match spec.get("flags") {
                    Some(Value::Array(a)) => a.iter().filter_map(|v| {
                        if let Value::String(s) = v { Some(s.to_string()) } else { None }
                    }).collect(),
                    _ => vec![],
                };
                let options: Vec<String> = match spec.get("options") {
                    Some(Value::Array(a)) => a.iter().filter_map(|v| {
                        if let Value::String(s) = v { Some(s.to_string()) } else { None }
                    }).collect(),
                    _ => vec![],
                };
                let positional_names: Vec<String> = match spec.get("positionals") {
                    Some(Value::Array(a)) => a.iter().filter_map(|v| {
                        if let Value::String(s) = v { Some(s.to_string()) } else { None }
                    }).collect(),
                    _ => vec![],
                };

                let mut result: indexmap::IndexMap<String, Value> = indexmap::IndexMap::new();
                // Initialise defaults
                for f in &flags   { result.insert(f.clone(), Value::Boolean(false)); }
                for o in &options  { result.insert(o.clone(), Value::Null); }
                for p in &positional_names { result.insert(p.clone(), Value::Null); }
                result.insert("_rest".to_string(), Value::Array(vec![]));

                let mut positional_idx = 0;
                let mut rest: Vec<Value> = vec![];
                let mut iter = raw_args.iter().peekable();
                while let Some(arg_val) = iter.next() {
                    let arg = match arg_val {
                        Value::String(s) => s.to_string(),
                        other => other.to_string(),
                    };
                    let key = arg.trim_start_matches('-').to_string();
                    if arg.starts_with("--") || (arg.starts_with('-') && arg.len() == 2) {
                        if flags.contains(&key) {
                            result.insert(key, Value::Boolean(true));
                        } else if options.contains(&key) {
                            let val = iter.next().map(|v| match v {
                                Value::String(s) => Value::String(Arc::from(s.clone())),
                                other => Value::String(Arc::from(other.to_string())),
                            }).unwrap_or(Value::Null);
                            result.insert(key, val);
                        } else {
                            rest.push(Value::String(Arc::from(arg)));
                        }
                    } else if positional_idx < positional_names.len() {
                        result.insert(positional_names[positional_idx].clone(), Value::String(Arc::from(arg)));
                        positional_idx += 1;
                    } else {
                        rest.push(Value::String(Arc::from(arg)));
                    }
                }
                result.insert("_rest".to_string(), Value::Array(rest));
                Ok(Value::Map(result))
            }

            _ => Err(RuntimeError::new(format!("Unknown sys function: {}", name))),
        }
    }
}

/// Recursively convert a serde_json::Value into a runtime Value.
fn json_to_value(j: &serde_json::Value) -> Value {
    match j {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Boolean(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Integer(i)
            } else {
                Value::Float(n.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::String(s) => Value::String(Arc::from(s.clone())),
        serde_json::Value::Array(arr) => Value::Array(arr.iter().map(json_to_value).collect()),
        serde_json::Value::Object(obj) => {
            let map = obj.iter().map(|(k, v)| (k.clone(), json_to_value(v))).collect();
            Value::Map(map)
        }
    }
}
