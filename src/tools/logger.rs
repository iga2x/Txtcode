use crate::config::Config;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

/// Log levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
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
}

/// Simple file-based logger
pub struct Logger {
    log_file: PathBuf,
    enabled: bool,
    min_level: LogLevel,
}

impl Logger {
    /// Create a new logger instance
    ///
    /// `name` - Base name for the log file (e.g., "txtcode" → ~/.txtcode/logs/txtcode.log)
    pub fn new(name: &str) -> Result<Self, String> {
        let log_file = Config::get_log_path(name)?;

        // Ensure log directory exists
        if let Some(parent) = log_file.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create log directory: {}", e))?;
        }

        Ok(Self {
            log_file,
            enabled: true,
            min_level: LogLevel::Info,
        })
    }

    /// Set the minimum log level
    pub fn set_min_level(&mut self, level: LogLevel) {
        self.min_level = level;
    }

    /// Enable or disable logging
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Log a message at the specified level
    pub fn log(&self, level: LogLevel, message: &str) -> Result<(), String> {
        if !self.enabled || level < self.min_level {
            return Ok(());
        }

        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let log_entry = format!("[{}] [{}] {}\n", timestamp, level.as_str(), message);

        // Write to file
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file)
            .map_err(|e| format!("Failed to open log file: {}", e))?;

        file.write_all(log_entry.as_bytes())
            .map_err(|e| format!("Failed to write to log file: {}", e))?;

        // Also print to stderr for errors and warnings
        match level {
            LogLevel::Error => eprintln!("{}", message),
            LogLevel::Warn => eprintln!("Warning: {}", message),
            _ => {}
        }

        Ok(())
    }

    /// Log a debug message
    pub fn debug(&self, message: &str) -> Result<(), String> {
        self.log(LogLevel::Debug, message)
    }

    /// Log an info message
    pub fn info(&self, message: &str) -> Result<(), String> {
        self.log(LogLevel::Info, message)
    }

    /// Log a warning message
    pub fn warn(&self, message: &str) -> Result<(), String> {
        self.log(LogLevel::Warn, message)
    }

    /// Log an error message
    pub fn error(&self, message: &str) -> Result<(), String> {
        self.log(LogLevel::Error, message)
    }
}

impl PartialOrd for LogLevel {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let self_val = match self {
            LogLevel::Debug => 0,
            LogLevel::Info => 1,
            LogLevel::Warn => 2,
            LogLevel::Error => 3,
        };
        let other_val = match other {
            LogLevel::Debug => 0,
            LogLevel::Info => 1,
            LogLevel::Warn => 2,
            LogLevel::Error => 3,
        };
        self_val.partial_cmp(&other_val)
    }
}

/// Global logger instance (thread-safe)
static GLOBAL_LOGGER: Mutex<Option<Logger>> = Mutex::new(None);

/// Initialize the global logger
pub fn init_logger(name: &str) -> Result<(), String> {
    let logger = Logger::new(name)?;
    let mut global = GLOBAL_LOGGER.lock().unwrap();
    *global = Some(logger);
    Ok(())
}

/// Log a debug message using the global logger
pub fn log_debug(message: &str) {
    let mut global = GLOBAL_LOGGER.lock().unwrap();
    if global.is_none() {
        if let Ok(logger) = Logger::new("txtcode") {
            *global = Some(logger);
        } else {
            return;
        }
    }
    if let Some(ref logger) = *global {
        let _ = logger.debug(message);
    }
}

/// Log an info message using the global logger
pub fn log_info(message: &str) {
    let mut global = GLOBAL_LOGGER.lock().unwrap();
    if global.is_none() {
        if let Ok(logger) = Logger::new("txtcode") {
            *global = Some(logger);
        } else {
            return;
        }
    }
    if let Some(ref logger) = *global {
        let _ = logger.info(message);
    }
}

/// Log a warning message using the global logger
pub fn log_warn(message: &str) {
    let mut global = GLOBAL_LOGGER.lock().unwrap();
    if global.is_none() {
        if let Ok(logger) = Logger::new("txtcode") {
            *global = Some(logger);
        } else {
            return;
        }
    }
    if let Some(ref logger) = *global {
        let _ = logger.warn(message);
    }
}

/// Log an error message using the global logger
pub fn log_error(message: &str) {
    let mut global = GLOBAL_LOGGER.lock().unwrap();
    if global.is_none() {
        if let Ok(logger) = Logger::new("txtcode") {
            *global = Some(logger);
        } else {
            return;
        }
    }
    if let Some(ref logger) = *global {
        let _ = logger.error(message);
    }
}
