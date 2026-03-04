use crate::runtime::{Value, RuntimeError};
use std::process::{Command, Child};
use std::collections::HashMap;
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref CHILD_PROCESSES: Mutex<HashMap<i64, Child>> = Mutex::new(HashMap::new());
    static ref PROCESS_COUNTER: Mutex<i64> = Mutex::new(0);
}

/// System standard library functions
pub struct SysLib;

impl SysLib {
    pub fn call_function(name: &str, args: &[Value], exec_allowed: bool, permission_checker: Option<&dyn crate::stdlib::permission_checker::PermissionChecker>) -> Result<Value, RuntimeError> {
        match name {
            "getenv" => {
                if let Some(Value::String(key)) = args.first() {
                    Ok(Value::String(
                        std::env::var(key).unwrap_or_default()
                    ))
                } else {
                    Err(RuntimeError::new("getenv() requires a string argument".to_string()))
                }
            }
            "setenv" => {
                if args.len() >= 2 {
                    if let (Value::String(key), Value::String(val)) = (&args[0], &args[1]) {
                        std::env::set_var(key, val);
                        Ok(Value::Null)
                    } else {
                        Err(RuntimeError::new("setenv() requires two string arguments".to_string()))
                    }
                } else {
                    Err(RuntimeError::new("setenv() requires two arguments".to_string()))
                }
            }
            "platform" => {
                Ok(Value::String(std::env::consts::OS.to_string()))
            }
            "arch" => {
                Ok(Value::String(std::env::consts::ARCH.to_string()))
            }
            "exec" => {
                if !exec_allowed {
                    return Err(RuntimeError::new("exec() is disabled in safe mode".to_string()));
                }
                if let Some(Value::String(cmd)) = args.first() {
                    // Check permission if checker is available
                    if let Some(checker) = permission_checker {
                        use crate::runtime::permissions::PermissionResource;
                        // Parse command to get first part (command name)
                        let cmd_parts: Vec<&str> = cmd.split_whitespace().collect();
                        if let Some(first_cmd) = cmd_parts.first() {
                            checker.check_permission(
                                &PermissionResource::System("exec".to_string()),
                                Some(first_cmd)
                            )?;
                        }
                    }
                    
                    let output = Command::new("sh")
                        .arg("-c")
                        .arg(cmd)
                        .output()
                        .map_err(|e| RuntimeError::new(format!("exec() failed: {}", e)))?;
                    Ok(Value::String(String::from_utf8_lossy(&output.stdout).to_string()))
                } else {
                    Err(RuntimeError::new("exec() requires a string argument".to_string()))
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
                let args: Vec<Value> = std::env::args()
                    .skip(1) // Skip program name
                    .map(|s| Value::String(s))
                    .collect();
                Ok(Value::Array(args))
            }
            "cwd" => {
                match std::env::current_dir() {
                    Ok(path) => Ok(Value::String(path.to_string_lossy().to_string())),
                    Err(e) => Err(RuntimeError::new(format!("Failed to get current directory: {}", e))),
                }
            }
            "chdir" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("chdir requires 1 argument (path)".to_string()));
                }
                match &args[0] {
                    Value::String(path) => {
                        std::env::set_current_dir(path)
                            .map_err(|e| RuntimeError::new(format!("Failed to change directory: {}", e)))?;
                        Ok(Value::Null)
                    }
                    _ => Err(RuntimeError::new("chdir requires a string path".to_string())),
                }
            }
            "pid" => {
                Ok(Value::Integer(std::process::id() as i64))
            }
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
                            let name = CStr::from_ptr((*passwd).pw_name).to_string_lossy().to_string();
                            Ok(Value::String(name))
                        }
                    }
                }
                #[cfg(not(unix))]
                {
                    Ok(Value::String(std::env::var("USER").unwrap_or_else(|_| std::env::var("USERNAME").unwrap_or_else(|_| "unknown".to_string()))))
                }
            }
            "home" => {
                match dirs::home_dir() {
                    Some(path) => Ok(Value::String(path.to_string_lossy().to_string())),
                    None => Err(RuntimeError::new("Failed to get home directory".to_string())),
                }
            }
            "uid" => {
                #[cfg(unix)]
                {
                    unsafe {
                        Ok(Value::Integer(libc::getuid() as i64))
                    }
                }
                #[cfg(not(unix))]
                {
                    Ok(Value::Integer(0))
                }
            }
            "gid" => {
                #[cfg(unix)]
                {
                    unsafe {
                        Ok(Value::Integer(libc::getgid() as i64))
                    }
                }
                #[cfg(not(unix))]
                {
                    Ok(Value::Integer(0))
                }
            }
            "spawn" => {
                if !exec_allowed {
                    return Err(RuntimeError::new("spawn() is disabled in safe mode".to_string()));
                }
                if args.is_empty() {
                    return Err(RuntimeError::new("spawn requires at least 1 argument (command)".to_string()));
                }
                
                let cmd_str = match &args[0] {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::new("spawn command must be a string".to_string())),
                };
                
                let mut cmd = Command::new("sh");
                cmd.arg("-c").arg(&cmd_str);
                
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
                
                let child = cmd.spawn()
                    .map_err(|e| RuntimeError::new(format!("spawn() failed: {}", e)))?;
                
                let pid = child.id() as i64;
                
                // Store child process
                let mut processes = CHILD_PROCESSES.lock().unwrap();
                processes.insert(pid, child);
                
                Ok(Value::Integer(pid))
            }
            "kill" => {
                if !exec_allowed {
                    return Err(RuntimeError::new("kill() is disabled in safe mode".to_string()));
                }
                if args.len() < 1 || args.len() > 2 {
                    return Err(RuntimeError::new("kill requires 1 or 2 arguments (pid, signal?)".to_string()));
                }
                
                let pid = match &args[0] {
                    Value::Integer(p) => *p,
                    _ => return Err(RuntimeError::new("kill pid must be an integer".to_string())),
                };
                
                // Try to kill from our managed processes first
                let mut processes = CHILD_PROCESSES.lock().unwrap();
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
                            Value::String(s) => {
                                match s.as_str() {
                                    "SIGTERM" | "TERM" => libc::SIGTERM,
                                    "SIGKILL" | "KILL" => libc::SIGKILL,
                                    "SIGINT" | "INT" => libc::SIGINT,
                                    _ => libc::SIGTERM,
                                }
                            }
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
                    Err(RuntimeError::new("kill() not fully supported on this platform".to_string()))
                }
            }
            "wait" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("wait requires 1 argument (pid)".to_string()));
                }
                
                let pid = match &args[0] {
                    Value::Integer(p) => *p,
                    _ => return Err(RuntimeError::new("wait pid must be an integer".to_string())),
                };
                
                let mut processes = CHILD_PROCESSES.lock().unwrap();
                if let Some(mut child) = processes.remove(&pid) {
                    match child.wait() {
                        Ok(status) => {
                            let exit_code = status.code().unwrap_or(-1);
                            let mut result = HashMap::new();
                            result.insert("pid".to_string(), Value::Integer(pid));
                            result.insert("exit_code".to_string(), Value::Integer(exit_code as i64));
                            result.insert("success".to_string(), Value::Boolean(status.success()));
                            Ok(Value::Map(result))
                        }
                        Err(e) => Err(RuntimeError::new(format!("wait() failed: {}", e))),
                    }
                } else {
                    drop(processes);
                    Err(RuntimeError::new(format!("Process {} not found or already waited", pid)))
                }
            }
            "sleep" => {
                let ms = if let Some(Value::Integer(m)) = args.first() {
                    *m as u64
                } else if let Some(Value::Float(f)) = args.first() {
                    *f as u64
                } else {
                    return Err(RuntimeError::new("sleep() requires a number argument (milliseconds)".to_string()));
                };
                std::thread::sleep(std::time::Duration::from_millis(ms));
                Ok(Value::Null)
            }
            _ => Err(RuntimeError::new(format!("Unknown sys function: {}", name))),
        }
    }
}

