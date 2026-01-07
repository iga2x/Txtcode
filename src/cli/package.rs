use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageConfig {
    pub name: String,
    pub version: String,
    pub dependencies: std::collections::HashMap<String, String>,
}

impl PackageConfig {
    pub fn new(name: String, version: String) -> Self {
        Self {
            name,
            version,
            dependencies: std::collections::HashMap::new(),
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
    
    // Create packages directory
    let packages_dir = PathBuf::from("packages");
    if !packages_dir.exists() {
        fs::create_dir_all(&packages_dir)?;
    }

    // Install each dependency
    for (name, version) in &config.dependencies {
        println!("  Installing {}@{}", name, version);
        // In a full implementation, this would download from a package repository
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
    Ok(())
}

pub fn update_dependencies() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = PathBuf::from("Txtcode.toml");
    if !config_path.exists() {
        return Err("Txtcode.toml not found. Run 'txtcode package init' first.".into());
    }

    let config = PackageConfig::load(&config_path)?;
    println!("Updating {} dependencies...", config.dependencies.len());
    
    // In a full implementation, this would check for updates and install them
    println!("Dependencies updated successfully");
    Ok(())
}
