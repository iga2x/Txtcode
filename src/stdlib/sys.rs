use crate::runtime::{RuntimeError, Value};
use std::collections::HashMap;
use std::process::{Child, Command};
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref CHILD_PROCESSES: Mutex<HashMap<i64, Child>> = Mutex::new(HashMap::new());
    static ref PROCESS_COUNTER: Mutex<i64> = Mutex::new(0);
}

/// System standard library functions
pub struct SysLib;

impl SysLib {
    pub fn call_function(
        name: &str,
        args: &[Value],
        exec_allowed: bool,
        permission_checker: Option<&dyn crate::stdlib::permission_checker::PermissionChecker>,
    ) -> Result<Value, RuntimeError> {
        match name {
            "getenv" => {
                if let Some(Value::String(key)) = args.first() {
                    if let Some(checker) = permission_checker {
                        use crate::runtime::permissions::PermissionResource;
                        checker.check_permission(
                            &PermissionResource::System("env".to_string()),
                            Some(key.as_str()),
                        )?;
                    }
                    Ok(Value::String(std::env::var(key).unwrap_or_default()))
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
                        std::env::set_var(key, val);
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
            "platform" => Ok(Value::String(std::env::consts::OS.to_string())),
            "arch" => Ok(Value::String(std::env::consts::ARCH.to_string())),
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

                    let output = Command::new(executable)
                        .args(cmd_args)
                        .output()
                        .map_err(|e| RuntimeError::new(format!("exec() failed: {}", e)))?;
                    Ok(Value::String(
                        String::from_utf8_lossy(&output.stdout).to_string(),
                    ))
                } else {
                    Err(RuntimeError::new(
                        "exec() requires a string argument".to_string(),
                    ))
                }
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
                    .map(Value::String)
                    .collect();
                Ok(Value::Array(args))
            }
            "cwd" => match std::env::current_dir() {
                Ok(path) => Ok(Value::String(path.to_string_lossy().to_string())),
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
                        std::env::set_current_dir(path).map_err(|e| {
                            RuntimeError::new(format!("Failed to change directory: {}", e))
                        })?;
                        Ok(Value::Null)
                    }
                    _ => Err(RuntimeError::new(
                        "chdir requires a string path".to_string(),
                    )),
                }
            }
            "pid" => Ok(Value::Integer(std::process::id() as i64)),
            "user" => {
                #[cfg(unix)]
                {
                    use std::ffi::CStr;
                    unsafe {
                        let uid = libc::getuid();
                        let passwd = libc::getpwuid(uid);
                        if passwd.is_null() {
                            Ok(Value::String("unknown".to_string()))
                        } else {
                            let name = CStr::from_ptr((*passwd).pw_name)
                                .to_string_lossy()
                                .to_string();
                            Ok(Value::String(name))
                        }
                    }
                }
                #[cfg(not(unix))]
                {
                    Ok(Value::String(std::env::var("USER").unwrap_or_else(|_| {
                        std::env::var("USERNAME").unwrap_or_else(|_| "unknown".to_string())
                    })))
                }
            }
            "home" => match dirs::home_dir() {
                Some(path) => Ok(Value::String(path.to_string_lossy().to_string())),
                None => Err(RuntimeError::new(
                    "Failed to get home directory".to_string(),
                )),
            },
            "uid" => {
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
                                cmd.env(key, val_str);
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
                            Value::String(s) => match s.as_str() {
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
                            let mut result = HashMap::new();
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
                let vars: HashMap<String, Value> = std::env::vars()
                    .map(|(k, v)| (k, Value::String(v)))
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
                            Value::String(s) => match s.as_str() {
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
                let mut result = HashMap::new();
                result.insert(
                    "stdout".to_string(),
                    Value::String(String::from_utf8_lossy(&output.stdout).to_string()),
                );
                result.insert(
                    "stderr".to_string(),
                    Value::String(String::from_utf8_lossy(&output.stderr).to_string()),
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
                    let candidate = std::path::Path::new(dir).join(&binary);
                    if candidate.is_file() {
                        return Ok(Value::String(candidate.to_string_lossy().to_string()));
                    }
                }
                Ok(Value::Null)
            }
            "is_root" => {
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
                let count = std::thread::available_parallelism()
                    .map(|n| n.get() as i64)
                    .unwrap_or(1);
                Ok(Value::Integer(count))
            }
            "memory_available" => {
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
                let path = if args.is_empty() {
                    ".".to_string()
                } else {
                    match &args[0] {
                        Value::String(s) => s.clone(),
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
                    let mut map = HashMap::new();
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
            "os_name" => Ok(Value::String(std::env::consts::OS.to_string())),
            "os_version" => {
                // Read kernel/OS version from files — no external process spawn needed (2.1).
                #[cfg(target_os = "linux")]
                {
                    // Try /etc/os-release for pretty distro name first
                    if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
                        for line in content.lines() {
                            if line.starts_with("PRETTY_NAME=") {
                                let val = line.trim_start_matches("PRETTY_NAME=").trim_matches('"');
                                return Ok(Value::String(val.to_string()));
                            }
                        }
                    }
                    // Fallback: read kernel version from /proc/version
                    if let Ok(content) = std::fs::read_to_string("/proc/version") {
                        let version = content.split_whitespace().take(3).collect::<Vec<_>>().join(" ");
                        return Ok(Value::String(version));
                    }
                    Ok(Value::String("linux/unknown".to_string()))
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
                                    return Ok(Value::String(format!("macOS {}", ver)));
                                }
                            }
                        }
                    }
                    Ok(Value::String("macOS/unknown".to_string()))
                }
                #[cfg(not(any(target_os = "linux", target_os = "macos")))]
                {
                    Ok(Value::String(std::env::consts::OS.to_string()))
                }
            }
            _ => Err(RuntimeError::new(format!("Unknown sys function: {}", name))),
        }
    }
}
