use crate::runtime::{Value, RuntimeError};
use std::io::{self, Write};
use chrono::Local;

/// Logging library
pub struct LogLib;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }

    fn color_code(&self) -> &'static str {
        match self {
            LogLevel::Debug => "\x1b[36m", // Cyan
            LogLevel::Info => "\x1b[32m",  // Green
            LogLevel::Warn => "\x1b[33m",   // Yellow
            LogLevel::Error => "\x1b[31m",  // Red
        }
    }
}

impl LogLib {
    /// Call a logging library function
    pub fn call_function(name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
        let level = match name {
            "log_debug" | "debug" => LogLevel::Debug,
            "log_info" | "info" => LogLevel::Info,
            "log_warn" | "warn" => LogLevel::Warn,
            "log_error" | "error" => LogLevel::Error,
            "log" => {
                // log(level, message) - explicit level
                if args.len() != 2 {
                    return Err(RuntimeError::new("log requires 2 arguments (level, message)".to_string()));
                }
                let level_str = match &args[0] {
                    Value::String(s) => s.to_uppercase(),
                    _ => return Err(RuntimeError::new("log level must be a string".to_string())),
                };
                let level = match level_str.as_str() {
                    "DEBUG" | "DBG" => LogLevel::Debug,
                    "INFO" => LogLevel::Info,
                    "WARN" | "WARNING" => LogLevel::Warn,
                    "ERROR" | "ERR" => LogLevel::Error,
                    _ => return Err(RuntimeError::new(format!("Invalid log level: {}", level_str))),
                };
                let message = match &args[1] {
                    Value::String(s) => s.clone(),
                    v => v.to_string(),
                };
                Self::log_message(level, &message);
                return Ok(Value::Null);
            }
            _ => return Err(RuntimeError::new(format!("Unknown log function: {}", name))),
        };

        if args.is_empty() {
            return Err(RuntimeError::new(format!("{} requires at least 1 argument (message)", name)));
        }

        // Format message from arguments
        let message = if args.len() == 1 {
            match &args[0] {
                Value::String(s) => s.clone(),
                v => v.to_string(),
            }
        } else {
            // Multiple arguments - format them
            args.iter()
                .map(|v| v.to_string())
                .collect::<Vec<String>>()
                .join(" ")
        };

        Self::log_message(level, &message);
        Ok(Value::Null)
    }

    fn log_message(level: LogLevel, message: &str) {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let reset = "\x1b[0m";
        let color = level.color_code();
        
        let output = format!(
            "{}[{}] {} {}{}\n",
            color,
            timestamp,
            level.as_str(),
            message,
            reset
        );

        // Write to stderr for errors/warnings, stdout for info/debug
        let mut target: Box<dyn Write> = match level {
            LogLevel::Error | LogLevel::Warn => Box::new(io::stderr()),
            _ => Box::new(io::stdout()),
        };

        let _ = target.write_all(output.as_bytes());
        let _ = target.flush();
    }
}

