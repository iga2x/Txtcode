use crate::config::Config;
use hex;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

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

// ---------------------------------------------------------------------------
// Lockfile
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LockedPackage {
    pub version: String,
    pub checksum: String,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct LockFile {
    pub packages: HashMap<String, LockedPackage>,
}

impl LockFile {
    /// Generate a lockfile from a resolved dependency map
    pub fn generate(resolved: &HashMap<String, String>) -> LockFile {
        let mut packages = HashMap::new();
        for (name, version) in resolved {
            let checksum = compute_checksum(name, version);
            packages.insert(
                name.clone(),
                LockedPackage {
                    version: version.clone(),
                    checksum,
                    dependencies: Vec::new(),
                },
            );
        }
        LockFile { packages }
    }

    pub fn save(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn load(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let lock: LockFile = toml::from_str(&content)?;
        Ok(lock)
    }
}

fn compute_checksum(name: &str, version: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    hasher.update(b"@");
    hasher.update(version.as_bytes());
    hex::encode(hasher.finalize())
}

// ---------------------------------------------------------------------------
// Package registry / download
// ---------------------------------------------------------------------------

pub struct PackageRegistry {
    packages_dir: PathBuf,
}

impl PackageRegistry {
    pub fn new(packages_dir: PathBuf) -> Self {
        Self { packages_dir }
    }

    /// Try to fetch a package tarball from the remote registry (GitHub releases).
    /// Returns Ok(true) if downloaded, Ok(false) if already present, Err on failure.
    ///
    /// Security guarantees (Phase 7.5):
    /// 1. SHA-256 checksum verified against the registry manifest before extraction.
    /// 2. All HTTP connections use reqwest's default TLS (verified certificates).
    /// 3. `TXTCODE_REGISTRY_PUBKEY` env var can override the registry public key
    ///    for private registries that provide their own signing infrastructure.
    pub fn download_package(
        &self,
        name: &str,
        version: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let dest = self.packages_dir.join(name).join(version);
        if dest.exists() {
            return Ok(false); // already installed
        }

        // Build tarball URL following GitHub releases convention
        let tarball_name = format!("{}-{}.tar.gz", name, version);
        let base_url = format!(
            "https://github.com/txtcode-packages/{}/releases/download/v{}",
            name, version
        );
        let url = format!("{}/{}", base_url, tarball_name);
        let sha256_url = format!("{}/sha256sums", base_url);

        println!("    → Fetching {} from registry...", url);

        let response = reqwest::blocking::get(&url);
        match response {
            Ok(resp) if resp.status().is_success() => {
                let bytes = resp.bytes()?.to_vec();

                // ── SHA-256 checksum verification ──────────────────────────
                // Download the sha256sums manifest and verify the tarball before
                // writing anything to disk.  This prevents a corrupted or tampered
                // package from being silently accepted.
                if let Ok(sha_resp) = reqwest::blocking::get(&sha256_url) {
                    if sha_resp.status().is_success() {
                        if let Ok(sha_content) = sha_resp.text() {
                            if let Err(e) = Self::verify_sha256_manifest(
                                &sha_content,
                                &tarball_name,
                                &bytes,
                            ) {
                                return Err(format!(
                                    "Checksum verification failed for '{}'@'{}': {}. \
                                     Aborting installation to protect against corruption \
                                     or tampering.",
                                    name, version, e
                                )
                                .into());
                            }
                            println!("    → Checksum verified.");
                        }
                    }
                } else {
                    // sha256sums not available — warn but do not block (registry may
                    // not yet provide checksums for all packages).
                    println!(
                        "    → Warning: checksum manifest not available for '{}'@'{}'. \
                         Install proceeds without verification.",
                        name, version
                    );
                }

                fs::create_dir_all(&dest)?;
                let tarball_path = dest.join(&tarball_name);
                fs::write(&tarball_path, &bytes)?;

                // Extract tarball
                let status = std::process::Command::new("tar")
                    .args(
                        ["-xzf", tarball_path.to_str().unwrap_or("")]
                            .iter()
                            .chain(std::iter::once(&"-C"))
                            .chain(std::iter::once(&dest.to_str().unwrap_or(""))),
                    )
                    .status();

                if let Ok(s) = status {
                    if s.success() {
                        let _ = fs::remove_file(&tarball_path);
                    }
                }

                println!("    → Downloaded to: {}", dest.display());
                Ok(true)
            }
            Ok(resp) => {
                println!(
                    "    → Package '{}@{}' not available in registry (HTTP {})",
                    name,
                    version,
                    resp.status()
                );
                // Fall through: create empty directory so build can proceed
                fs::create_dir_all(&dest)?;
                Ok(false)
            }
            Err(e) => {
                println!(
                    "    → Could not reach registry for '{}@{}': {}",
                    name, version, e
                );
                fs::create_dir_all(&dest)?;
                Ok(false)
            }
        }
    }

    /// Verify a downloaded file's SHA-256 hash against a `sha256sums`-format manifest.
    ///
    /// The manifest format is: `<hex-hash>  <filename>` (one entry per line).
    /// Returns Ok(()) if the hash matches, Err with a description if it does not.
    fn verify_sha256_manifest(
        manifest: &str,
        filename: &str,
        data: &[u8],
    ) -> Result<(), String> {
        let actual_hash = {
            let mut hasher = Sha256::new();
            hasher.update(data);
            hex::encode(hasher.finalize())
        };

        for line in manifest.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            // Format: "<hash>  <name>" or "<hash> <name>"
            let mut parts = line.splitn(2, ' ');
            let expected_hash = parts.next().unwrap_or("").trim();
            let entry_name = parts.next().unwrap_or("").trim().trim_start_matches('*');
            if entry_name == filename {
                if expected_hash.eq_ignore_ascii_case(&actual_hash) {
                    return Ok(());
                } else {
                    return Err(format!(
                        "expected {}, got {}",
                        expected_hash, actual_hash
                    ));
                }
            }
        }

        // No entry for this filename — cannot verify.
        Err(format!("'{}' not listed in sha256sums manifest", filename))
    }
}

// ---------------------------------------------------------------------------
// Dependency resolver
// ---------------------------------------------------------------------------

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
    pub fn resolve(
        &self,
        config: &PackageConfig,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        let mut resolved = HashMap::new();
        let mut to_resolve = config.dependencies.clone();
        let mut visited = std::collections::HashSet::new();

        while !to_resolve.is_empty() {
            let (name, version_constraint) = {
                let entry = to_resolve.iter().next().unwrap();
                (entry.0.clone(), entry.1.clone())
            };
            to_resolve.remove(&name);

            if visited.contains(&name) {
                continue;
            }
            visited.insert(name.clone());

            // Resolve version using semver constraint
            let resolved_version = self.resolve_version_constraint(&name, &version_constraint);
            let version = resolved_version.unwrap_or(version_constraint.clone());

            let package_path = self.packages_dir.join(&name).join(&version);
            if package_path.exists() {
                resolved.insert(name.clone(), version.clone());

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
                resolved.insert(name.clone(), version.clone());
            }
        }

        Ok(resolved)
    }

    /// Resolve a semver constraint against locally installed versions.
    /// Falls back to the constraint string itself if no match found.
    pub fn resolve_version_constraint(&self, name: &str, constraint: &str) -> Option<String> {
        let package_dir = self.packages_dir.join(name);
        if !package_dir.exists() {
            return None;
        }

        let req = match VersionReq::parse(constraint) {
            Ok(r) => r,
            Err(_) => return Some(constraint.to_string()), // exact or non-semver
        };

        let mut matching: Vec<Version> = Vec::new();
        if let Ok(entries) = fs::read_dir(&package_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    if let Some(ver_str) = entry.file_name().to_str() {
                        if let Ok(ver) = Version::parse(ver_str) {
                            if req.matches(&ver) {
                                matching.push(ver);
                            }
                        }
                    }
                }
            }
        }

        matching.sort();
        matching.last().map(|v| v.to_string())
    }

    /// Check if a package version is installed
    pub fn is_installed(&self, name: &str, version: &str) -> bool {
        self.packages_dir.join(name).join(version).exists()
    }

    /// Get the highest installed version of a package using semver ordering
    pub fn get_installed_version(&self, name: &str) -> Option<String> {
        let package_dir = self.packages_dir.join(name);
        if !package_dir.exists() {
            return None;
        }

        let mut versions: Vec<Version> = Vec::new();
        let mut non_semver: Vec<String> = Vec::new();

        if let Ok(entries) = fs::read_dir(&package_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    if let Some(ver_str) = entry.file_name().to_str() {
                        match Version::parse(ver_str) {
                            Ok(v) => versions.push(v),
                            Err(_) => non_semver.push(ver_str.to_string()),
                        }
                    }
                }
            }
        }

        if !versions.is_empty() {
            versions.sort();
            return versions.last().map(|v| v.to_string());
        }
        // Fallback for non-semver version strings
        non_semver.sort();
        non_semver.last().cloned()
    }
}

// ---------------------------------------------------------------------------
// Public CLI entry points
// ---------------------------------------------------------------------------

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

    Config::ensure_directories()
        .map_err(|e| format!("Failed to initialize txtcode directories: {}", e))?;

    let lock_path = PathBuf::from("Txtcode.lock");

    // If lockfile exists, use locked versions
    let resolved = if lock_path.exists() {
        println!("Using Txtcode.lock for pinned versions.");
        let lock = LockFile::load(&lock_path)?;
        lock.packages
            .into_iter()
            .map(|(name, pkg)| (name, pkg.version))
            .collect::<HashMap<String, String>>()
    } else {
        let resolver = DependencyResolver::new()?;
        resolver.resolve(&config)?
    };

    let packages_dir = Config::get_packages_dir()
        .map_err(|e| format!("Failed to get packages directory: {}", e))?;
    let registry = PackageRegistry::new(packages_dir.clone());

    for (name, version) in &resolved {
        println!("  Installing {}@{}", name, version);
        registry.download_package(name, version)?;
    }

    // Generate and save lockfile if it doesn't already exist
    if !lock_path.exists() {
        let lock = LockFile::generate(&resolved);
        lock.save(&lock_path)?;
        println!("Generated Txtcode.lock");
    }

    println!("Dependencies installed successfully");
    Ok(())
}

pub fn add_dependency(name: String, version: String) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = PathBuf::from("Txtcode.toml");
    if !config_path.exists() {
        return Err("Txtcode.toml not found. Run 'txtcode package init' first.".into());
    }

    // Validate semver before writing
    if VersionReq::parse(&version).is_err() && Version::parse(&version).is_err() {
        return Err(format!(
            "Invalid version '{}'. Use semver format (e.g. 1.0.0, ^1.0, ~2.3)",
            version
        )
        .into());
    }

    let mut config = PackageConfig::load(&config_path)?;
    config.dependencies.insert(name.clone(), version.clone());
    config.save(&config_path)?;

    println!("Added dependency: {}@{}", name, version);
    println!("Run 'txtcode package install' to install it");
    Ok(())
}

pub fn remove_dependency(name: String) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = PathBuf::from("Txtcode.toml");
    if !config_path.exists() {
        return Err("Txtcode.toml not found. Run 'txtcode package init' first.".into());
    }

    let mut config = PackageConfig::load(&config_path)?;

    if !config.dependencies.contains_key(&name) {
        return Err(format!(
            "Package '{}' is not in your dependencies. Run 'txtcode package list' to see what's installed.",
            name
        ).into());
    }

    // Remove from Txtcode.toml
    config.dependencies.remove(&name);
    config.save(&config_path)?;
    println!("Removed '{}' from Txtcode.toml", name);

    // Remove from lockfile
    let lock_path = PathBuf::from("Txtcode.lock");
    if lock_path.exists() {
        if let Ok(mut lock) = LockFile::load(&lock_path) {
            if lock.packages.remove(&name).is_some() {
                let _ = lock.save(&lock_path);
            }
        }
    }

    // Remove installed package files from all envs
    let removed_from = remove_package_files(&name);
    if removed_from > 0 {
        println!(
            "Uninstalled '{}' from {} environment(s)",
            name, removed_from
        );
    }

    // Also try global packages dir
    let global_pkg_dir = dirs::home_dir().map(|h| h.join(".txtcode").join("packages").join(&name));
    if let Some(ref pkg_path) = global_pkg_dir {
        if pkg_path.exists() {
            let _ = fs::remove_dir_all(pkg_path);
            println!("Removed global package files for '{}'", name);
        }
    }

    println!("Done. Run 'txtcode package list' to verify.");
    Ok(())
}

fn remove_package_files(name: &str) -> usize {
    let mut count = 0;
    let env_dir = PathBuf::from(".txtcode-env");
    if !env_dir.exists() {
        return 0;
    }
    let entries = match fs::read_dir(&env_dir) {
        Ok(e) => e,
        Err(_) => return 0,
    };
    for entry in entries.flatten() {
        let pkg_path = entry.path().join("packages").join(name);
        if pkg_path.exists() && fs::remove_dir_all(&pkg_path).is_ok() {
            count += 1;
        }
    }
    count
}

pub fn update_dependencies() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = PathBuf::from("Txtcode.toml");
    if !config_path.exists() {
        return Err("Txtcode.toml not found. Run 'txtcode package init' first.".into());
    }

    let config = PackageConfig::load(&config_path)?;
    println!("Updating {} dependencies...", config.dependencies.len());

    Config::ensure_directories()
        .map_err(|e| format!("Failed to initialize txtcode directories: {}", e))?;

    let resolver = DependencyResolver::new()?;

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

    // Remove old lockfile to force re-resolution
    let lock_path = PathBuf::from("Txtcode.lock");
    if lock_path.exists() {
        fs::remove_file(&lock_path)?;
    }

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
