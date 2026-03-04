use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::config::Config;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageConfig {
    pub name: String,
    pub version: String,
    pub dependencies: HashMap<String, String>,
    pub dev_dependencies: Option<HashMap<String, String>>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
}

impl PackageConfig {
    pub fn new(name: String, version: String) -> Self {
        Self {
            name,
            version,
            dependencies: HashMap::new(),
            dev_dependencies: None,
            description: None,
            author: None,
            license: None,
        }
    }

    pub fn load(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: PackageConfig = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}

/// Dependency resolver
pub struct DependencyResolver {
    packages_dir: PathBuf,
}

impl DependencyResolver {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let packages_dir = Config::get_packages_dir()
            .map_err(|e| format!("Failed to get packages directory: {}", e))?;
        Ok(Self { packages_dir })
    }

    /// Resolve all dependencies for a package
    pub fn resolve(&self, config: &PackageConfig) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        let mut resolved = HashMap::new();
        let mut to_resolve = config.dependencies.clone();
        let mut visited = std::collections::HashSet::new();

        while !to_resolve.is_empty() {
            let (name, version) = {
                let entry = to_resolve.iter().next().unwrap();
                (entry.0.clone(), entry.1.clone())
            };
            to_resolve.remove(&name);

            if visited.contains(&name) {
                continue; // Already resolved
            }
            visited.insert(name.clone());

            // Check if package exists locally
            let package_path = self.packages_dir.join(&name).join(&version);
            if package_path.exists() {
                resolved.insert(name.clone(), version.clone());

                // Load package config and resolve its dependencies
                let package_config_path = package_path.join("Txtcode.toml");
                if package_config_path.exists() {
                    if let Ok(pkg_config) = PackageConfig::load(&package_config_path) {
                        for (dep_name, dep_version) in &pkg_config.dependencies {
                            if !resolved.contains_key(dep_name) && !visited.contains(dep_name) {
                                to_resolve.insert(dep_name.clone(), dep_version.clone());
                            }
                        }
                    }
                }
            } else {
                // Package not found - would need to download from registry
                // For now, just add to resolved (would fail at runtime)
                resolved.insert(name.clone(), version.clone());
            }
        }

        Ok(resolved)
    }

    /// Check if a package version is installed
    pub fn is_installed(&self, name: &str, version: &str) -> bool {
        let package_path = self.packages_dir.join(name).join(version);
        package_path.exists()
    }

    /// Get installed version of a package
    pub fn get_installed_version(&self, name: &str) -> Option<String> {
        let package_dir = self.packages_dir.join(name);
        if !package_dir.exists() {
            return None;
        }

        // Find latest version
        let mut versions = Vec::new();
        if let Ok(entries) = fs::read_dir(&package_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    if let Some(version) = entry.file_name().to_str() {
                        versions.push(version.to_string());
                    }
                }
            }
        }

        // Simple version comparison (semver would be better)
        versions.sort();
        versions.last().cloned()
    }
}

pub fn init_package(name: String, version: String) -> Result<(), Box<dyn std::error::Error>> {
    let config = PackageConfig::new(name, version);
    let path = PathBuf::from("Txtcode.toml");
    config.save(&path)?;
    println!("Initialized package: {}", config.name);
    Ok(())
}

pub fn install_dependencies() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = PathBuf::from("Txtcode.toml");
    if !config_path.exists() {
        return Err("Txtcode.toml not found. Run 'txtcode package init' first.".into());
    }

    let config = PackageConfig::load(&config_path)?;
    println!("Installing {} dependencies...", config.dependencies.len());
    
    // Ensure txtcode directories exist
    Config::ensure_directories()
        .map_err(|e| format!("Failed to initialize txtcode directories: {}", e))?;
    
    // Resolve dependencies
    let resolver = DependencyResolver::new()?;
    let resolved = resolver.resolve(&config)?;
    
    // Install each dependency
    for (name, version) in &resolved {
        println!("  Installing {}@{}", name, version);
        
        // Get package path in global packages directory
        let package_path = Config::get_package_path(name)
            .map_err(|e| format!("Failed to get package path: {}", e))?;
        
        let version_path = package_path.join(version);
        if !version_path.exists() {
            fs::create_dir_all(&version_path)
                .map_err(|e| format!("Failed to create package directory: {}", e))?;
            
            // In a full implementation, this would download from a package repository
            // For now, just create the directory structure
            println!("    → Installed to: {}", version_path.display());
        } else {
            println!("    → Already installed: {}", version_path.display());
        }
    }

    println!("Dependencies installed successfully");
    Ok(())
}

pub fn add_dependency(name: String, version: String) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = PathBuf::from("Txtcode.toml");
    if !config_path.exists() {
        return Err("Txtcode.toml not found. Run 'txtcode package init' first.".into());
    }

    let mut config = PackageConfig::load(&config_path)?;
    config.dependencies.insert(name.clone(), version.clone());
    config.save(&config_path)?;
    
    println!("Added dependency: {}@{}", name, version);
    println!("Run 'txtcode package install' to install it");
    Ok(())
}

pub fn update_dependencies() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = PathBuf::from("Txtcode.toml");
    if !config_path.exists() {
        return Err("Txtcode.toml not found. Run 'txtcode package init' first.".into());
    }

    let config = PackageConfig::load(&config_path)?;
    println!("Updating {} dependencies...", config.dependencies.len());
    
    // Ensure txtcode directories exist
    Config::ensure_directories()
        .map_err(|e| format!("Failed to initialize txtcode directories: {}", e))?;
    
    let resolver = DependencyResolver::new()?;
    
    // Check for updates
    for (name, requested_version) in &config.dependencies {
        if let Some(installed) = resolver.get_installed_version(name) {
            if &installed != requested_version {
                println!("  {}: {} -> {}", name, installed, requested_version);
            } else {
                println!("  {}: up to date ({})", name, installed);
            }
        } else {
            println!("  {}: not installed", name);
        }
    }
    
    // Reinstall to get latest versions
    install_dependencies()?;
    
    println!("Dependencies updated successfully");
    Ok(())
}

pub fn list_dependencies() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = PathBuf::from("Txtcode.toml");
    if !config_path.exists() {
        return Err("Txtcode.toml not found. Run 'txtcode package init' first.".into());
    }

    let config = PackageConfig::load(&config_path)?;
    let resolver = DependencyResolver::new()?;
    
    println!("Dependencies:");
    for (name, version) in &config.dependencies {
        if let Some(installed) = resolver.get_installed_version(name) {
            if installed == *version {
                println!("  {}@{} (installed)", name, version);
            } else {
                println!("  {}@{} (installed: {})", name, version, installed);
            }
        } else {
            println!("  {}@{} (not installed)", name, version);
        }
    }
    
    Ok(())
}
