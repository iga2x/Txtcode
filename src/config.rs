use std::path::PathBuf;
use std::fs;
use serde::{Deserialize, Serialize};

/// Configuration and directory management for txtcode runtime
pub struct Config;

impl Config {
    /// Get the txtcode home directory
    /// 
    /// - Linux/macOS: `~/.txtcode`
    /// - Windows: `%APPDATA%\txtcode`
    pub fn get_txtcode_home() -> Result<PathBuf, String> {
        #[cfg(target_os = "windows")]
        {
            // On Windows, use AppData\Roaming\txtcode
            let appdata = std::env::var("APPDATA")
                .map_err(|_| "Could not find APPDATA directory".to_string())?;
            Ok(PathBuf::from(appdata).join("txtcode"))
        }

        #[cfg(not(target_os = "windows"))]
        {
            // On Unix-like systems, use ~/.txtcode
            let home = dirs::home_dir()
                .ok_or_else(|| "Could not find home directory".to_string())?;
            Ok(home.join(".txtcode"))
        }
    }

    /// Get the packages directory
    /// 
    /// Stores installed packages: `~/.txtcode/packages/`
    pub fn get_packages_dir() -> Result<PathBuf, String> {
        let dir = Self::get_txtcode_home()?.join("packages");
        Ok(dir)
    }

    /// Get the cache directory
    /// 
    /// Stores compiled bytecode cache: `~/.txtcode/cache/`
    pub fn get_cache_dir() -> Result<PathBuf, String> {
        let dir = Self::get_txtcode_home()?.join("cache");
        Ok(dir)
    }

    /// Get the config directory
    /// 
    /// Stores configuration files: `~/.txtcode/`
    pub fn get_config_dir() -> Result<PathBuf, String> {
        Self::get_txtcode_home()
    }

    /// Get the logs directory
    /// 
    /// Stores runtime logs: `~/.txtcode/logs/`
    pub fn get_logs_dir() -> Result<PathBuf, String> {
        let dir = Self::get_txtcode_home()?.join("logs");
        Ok(dir)
    }

    /// Get the path to the main config file
    /// 
    /// Returns: `~/.txtcode/config.toml`
    pub fn get_config_file() -> Result<PathBuf, String> {
        let file = Self::get_config_dir()?.join("config.toml");
        Ok(file)
    }

    /// Ensure all required directories exist
    /// 
    /// Creates the txtcode home directory and all subdirectories
    pub fn ensure_directories() -> Result<(), String> {
        let home = Self::get_txtcode_home()?;
        
        // Create home directory
        fs::create_dir_all(&home)
            .map_err(|e| format!("Failed to create txtcode home directory: {}", e))?;

        // Create subdirectories
        let dirs = vec![
            Self::get_packages_dir()?,
            Self::get_cache_dir()?,
            Self::get_logs_dir()?,
        ];

        for dir in dirs {
            fs::create_dir_all(&dir)
                .map_err(|e| format!("Failed to create directory {:?}: {}", dir, e))?;
        }

        Ok(())
    }

    /// Initialize default configuration file if it doesn't exist
    pub fn init_default_config() -> Result<(), String> {
        let config_file = Self::get_config_file()?;
        
        if config_file.exists() {
            return Ok(()); // Config already exists
        }

        // Ensure config directory exists
        if let Some(parent) = config_file.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }

        // Create default config
        let default_config = r#"# Txt-code Configuration File
# This file contains user preferences for the txtcode compiler and runtime

[compiler]
# Default optimization level: none, basic, aggressive
optimization = "basic"

# Default target: bytecode, native, wasm
target = "bytecode"

# Enable obfuscation by default
obfuscate = false

# Enable encryption by default
encrypt = false

[runtime]
# Safe mode (disables exec() function)
safe_mode = false

# Allow exec() function (overrides safe_mode)
allow_exec = false

# Enable debug output
debug = false

# Enable verbose output
verbose = false

[package]
# Package repository URL (for future use)
repository_url = "https://packages.txtcode.dev"

# Cache compiled packages
cache_packages = true

[paths]
# Custom packages directory (overrides default ~/.txtcode/packages)
# packages = ""

# Custom cache directory (overrides default ~/.txtcode/cache)
# cache = ""

# Custom logs directory (overrides default ~/.txtcode/logs)
# logs = ""
"#;

        fs::write(&config_file, default_config)
            .map_err(|e| format!("Failed to write config file: {}", e))?;

        Ok(())
    }

    /// Get the full path to a package
    /// 
    /// Returns: `~/.txtcode/packages/{name}/`
    pub fn get_package_path(name: &str) -> Result<PathBuf, String> {
        let packages_dir = Self::get_packages_dir()?;
        Ok(packages_dir.join(name))
    }

    /// Get the full path to a cached bytecode file
    /// 
    /// Returns: `~/.txtcode/cache/{hash}.txtc`
    pub fn get_cache_path(hash: &str) -> Result<PathBuf, String> {
        let cache_dir = Self::get_cache_dir()?;
        Ok(cache_dir.join(format!("{}.txtc", hash)))
    }

    /// Get the full path to a log file
    /// 
    /// Returns: `~/.txtcode/logs/{name}.log`
    pub fn get_log_path(name: &str) -> Result<PathBuf, String> {
        let logs_dir = Self::get_logs_dir()?;
        Ok(logs_dir.join(format!("{}.log", name)))
    }

    /// Load user configuration from ~/.txtcode/config.toml
    /// 
    /// Returns default config if file doesn't exist or can't be parsed
    pub fn load_config() -> Result<UserConfig, String> {
        let config_file = Self::get_config_file()?;
        
        if !config_file.exists() {
            // Return default config if file doesn't exist
            return Ok(UserConfig::default());
        }

        let content = fs::read_to_string(&config_file)
            .map_err(|e| format!("Failed to read config file: {}", e))?;
        
        let config: UserConfig = toml::from_str(&content)
            .map_err(|e| format!("Failed to parse config file: {}", e))?;
        
        Ok(config)
    }
}

/// User configuration loaded from ~/.txtcode/config.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    #[serde(default)]
    pub compiler: CompilerConfig,
    #[serde(default)]
    pub runtime: RuntimeConfig,
    #[serde(default)]
    pub package: PackageConfig,
    #[serde(default)]
    pub paths: PathsConfig,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            compiler: CompilerConfig::default(),
            runtime: RuntimeConfig::default(),
            package: PackageConfig::default(),
            paths: PathsConfig::default(),
        }
    }
}

/// Compiler configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerConfig {
    #[serde(default = "default_optimization")]
    pub optimization: String,
    #[serde(default = "default_target")]
    pub target: String,
    #[serde(default)]
    pub obfuscate: bool,
    #[serde(default)]
    pub encrypt: bool,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            optimization: "basic".to_string(),
            target: "bytecode".to_string(),
            obfuscate: false,
            encrypt: false,
        }
    }
}

/// Runtime configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    #[serde(default)]
    pub safe_mode: bool,
    #[serde(default)]
    pub allow_exec: bool,
    #[serde(default)]
    pub debug: bool,
    #[serde(default)]
    pub verbose: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            safe_mode: false,
            allow_exec: false,
            debug: false,
            verbose: false,
        }
    }
}

/// Package configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageConfig {
    #[serde(default = "default_repository_url")]
    pub repository_url: String,
    #[serde(default = "default_true")]
    pub cache_packages: bool,
}

impl Default for PackageConfig {
    fn default() -> Self {
        Self {
            repository_url: "https://packages.txtcode.dev".to_string(),
            cache_packages: true,
        }
    }
}

/// Paths configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    pub packages: Option<String>,
    pub cache: Option<String>,
    pub logs: Option<String>,
}

impl Default for PathsConfig {
    fn default() -> Self {
        Self {
            packages: None,
            cache: None,
            logs: None,
        }
    }
}

fn default_optimization() -> String {
    "basic".to_string()
}

fn default_target() -> String {
    "bytecode".to_string()
}

fn default_repository_url() -> String {
    "https://packages.txtcode.dev".to_string()
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_txtcode_home() {
        let home = Config::get_txtcode_home();
        assert!(home.is_ok());
        let path = home.unwrap();
        assert!(path.to_string_lossy().contains("txtcode"));
    }

    #[test]
    fn test_get_packages_dir() {
        let dir = Config::get_packages_dir();
        assert!(dir.is_ok());
        let path = dir.unwrap();
        assert!(path.to_string_lossy().contains("packages"));
    }

    #[test]
    fn test_ensure_directories() {
        // This test will actually create directories
        let result = Config::ensure_directories();
        assert!(result.is_ok());
        
        // Verify directories exist
        assert!(Config::get_packages_dir().unwrap().exists());
        assert!(Config::get_cache_dir().unwrap().exists());
        assert!(Config::get_logs_dir().unwrap().exists());
    }
}

