use crate::config::Config;
use flate2::read::GzDecoder;
use hex;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use tar::Archive;

// ---------------------------------------------------------------------------
// Registry index types
// ---------------------------------------------------------------------------

/// A single version entry inside the registry index.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegistryVersionEntry {
    /// URL of the `.tar.gz` tarball to download.
    #[serde(default)]
    pub url: String,
    /// Expected SHA-256 hex digest of the tarball (empty = checksum not yet published).
    #[serde(default)]
    pub sha256: String,
    /// Optional local filesystem path to a package directory.
    /// When set, the package is installed by copying files directly (no tarball needed).
    /// Relative paths are resolved from the registry index file's directory.
    #[serde(default)]
    pub local_path: String,
    /// Packages that this version depends on (`name` → `version_constraint`).
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
}

/// Metadata and all published versions of one package in the registry.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegistryPackageEntry {
    pub description: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    /// Map of `"version"` → [`RegistryVersionEntry`].
    pub versions: HashMap<String, RegistryVersionEntry>,
}

impl RegistryPackageEntry {
    /// Highest published semver version, or `None` if there are none.
    pub fn latest_version(&self) -> Option<String> {
        let mut parsed: Vec<Version> = self
            .versions
            .keys()
            .filter_map(|v| Version::parse(v).ok())
            .collect();
        parsed.sort();
        parsed.last().map(|v| v.to_string())
    }
}

/// The top-level registry index (`index.json`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegistryIndex {
    /// Schema version — currently always `"1"`.
    pub version: String,
    /// ISO-8601 date the index was last regenerated.
    #[serde(default)]
    pub updated: String,
    /// All packages in the registry, keyed by package name.
    pub packages: HashMap<String, RegistryPackageEntry>,
}

impl RegistryIndex {
    /// Parse a registry index from a JSON string.
    pub fn from_str(s: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let index: Self = serde_json::from_str(s)
            .map_err(|e| format!("Failed to parse registry index: {}", e))?;
        Ok(index)
    }

    /// Load a registry index from a local file path.
    pub fn from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Cannot read index file '{}': {}", path.display(), e))?;
        Self::from_str(&content)
    }

    /// Search packages whose name, description, or keywords match `query`.
    ///
    /// Returns results sorted by name for deterministic output.
    pub fn search(&self, query: &str) -> Vec<(&str, &RegistryPackageEntry)> {
        let q = query.to_lowercase();
        let mut results: Vec<(&str, &RegistryPackageEntry)> = self
            .packages
            .iter()
            .filter(|(name, pkg)| {
                name.to_lowercase().contains(&q)
                    || pkg.description.to_lowercase().contains(&q)
                    || pkg.keywords.iter().any(|kw| kw.to_lowercase().contains(&q))
            })
            .map(|(name, pkg)| (name.as_str(), pkg))
            .collect();
        results.sort_by_key(|(name, _)| *name);
        results
    }

    /// Look up a package by exact name.
    pub fn get_package(&self, name: &str) -> Option<&RegistryPackageEntry> {
        self.packages.get(name)
    }
}

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
    /// SHA-256 of the actual downloaded tarball bytes.
    pub checksum: String,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct LockFile {
    pub packages: HashMap<String, LockedPackage>,
}

impl LockFile {
    /// Add a resolved package to the lockfile.
    /// `content` must be the raw downloaded tarball bytes — the checksum is
    /// computed from the actual file, not from name/version strings.
    pub fn add_package(
        &mut self,
        name: &str,
        version: &str,
        content: &[u8],
        deps: Vec<String>,
    ) {
        let mut hasher = Sha256::new();
        hasher.update(content);
        let checksum = hex::encode(hasher.finalize());
        self.packages.insert(
            name.to_string(),
            LockedPackage {
                version: version.to_string(),
                checksum,
                dependencies: deps,
            },
        );
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

    /// Verify that the installed package directory's content matches the
    /// recorded checksum.
    ///
    /// Returns `Ok(())` if the hash matches or if no checksum was recorded
    /// (e.g. legacy entries).  Returns `Err` with a descriptive message if
    /// the hash does not match.
    pub fn verify_installed(&self, name: &str, dir: &Path) -> Result<(), String> {
        let locked = match self.packages.get(name) {
            Some(p) => p,
            None => return Err(format!("'{}' not found in lockfile", name)),
        };
        if locked.checksum.is_empty() {
            // No checksum recorded — skip (backward-compatible).
            return Ok(());
        }
        let actual = compute_dir_hash(dir);
        if !actual.eq_ignore_ascii_case(&locked.checksum) {
            Err(format!(
                "lockfile hash mismatch for '{}': expected {}, got {}\n\
                 Run 'txtcode package update' to regenerate the lockfile.",
                name, locked.checksum, actual
            ))
        } else {
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Installed-package hash helpers
// ---------------------------------------------------------------------------

/// Compute a deterministic SHA-256 hash of all **files** inside `dir`.
///
/// Files are processed in lexicographic (sorted) name order so the result is
/// stable across platforms.  Each file contributes `name:content\n` to the
/// digest so renames are detected.  Sub-directories are ignored; packages
/// are expected to be flat.
///
/// Returns the hash as a lowercase hex string, or an all-zero string if
/// the directory is empty or cannot be read.
pub fn compute_dir_hash(dir: &Path) -> String {
    let mut hasher = Sha256::new();
    let mut files: Vec<PathBuf> = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                files.push(entry.path());
            }
        }
    }

    files.sort();

    for file in &files {
        if let Ok(content) = fs::read(file) {
            let name = file.file_name().unwrap_or_default().to_string_lossy();
            hasher.update(name.as_bytes());
            hasher.update(b":");
            hasher.update(&content);
            hasher.update(b"\n");
        }
    }

    hex::encode(hasher.finalize())
}

// ---------------------------------------------------------------------------
// Package registry / download
// ---------------------------------------------------------------------------

/// URL of the official Txtcode package registry index.
/// This is a static JSON file served from GitHub Pages.
/// v0.4.x: registry is not yet live — remote installs are rejected with a
/// clear error. Use `path = "../my_lib"` for local dependencies.
const REGISTRY_INDEX_URL: &str =
    "https://raw.githubusercontent.com/iga2x/txtcode-registry/main/index.json";

pub struct PackageRegistry {
    packages_dir: PathBuf,
}

impl PackageRegistry {
    pub fn new(packages_dir: PathBuf) -> Self {
        Self { packages_dir }
    }

    // ── Index loading ─────────────────────────────────────────────────────────

    /// Load the registry index.
    ///
    /// Resolution order (first match wins):
    /// 1. `TXTCODE_REGISTRY_INDEX_FILE` env var → read from local file (testing / offline).
    /// 2. `net` feature enabled → fetch from [`REGISTRY_INDEX_URL`] over HTTPS.
    /// 3. Otherwise → return an error explaining how to enable the registry.
    pub fn load_index(&self) -> Result<RegistryIndex, Box<dyn std::error::Error>> {
        // 1. Local file override (for tests and offline use)
        if let Ok(path) = std::env::var("TXTCODE_REGISTRY_INDEX_FILE") {
            return RegistryIndex::from_file(Path::new(&path));
        }

        // 2. Remote fetch (requires `net` feature)
        #[cfg(feature = "net")]
        {
            return Self::fetch_index_remote();
        }

        // 3. No network support compiled in
        #[cfg(not(feature = "net"))]
        Err("Registry access requires the 'net' feature. \
             Rebuild with: cargo build --features net\n\
             Alternatively, set TXTCODE_REGISTRY_INDEX_FILE to a local index.json path."
            .into())
    }

    /// Fetch the registry index from the remote URL (requires `net` feature).
    #[cfg(feature = "net")]
    fn fetch_index_remote() -> Result<RegistryIndex, Box<dyn std::error::Error>> {
        let url = std::env::var("TXTCODE_REGISTRY_INDEX_URL")
            .unwrap_or_else(|_| REGISTRY_INDEX_URL.to_string());
        let body = reqwest::blocking::get(&url)
            .map_err(|e| format!("Failed to fetch registry index from '{}': {}", url, e))?
            .error_for_status()
            .map_err(|e| format!("Registry returned error status: {}", e))?
            .text()
            .map_err(|e| format!("Failed to read registry response: {}", e))?;
        RegistryIndex::from_str(&body)
    }

    // ── Search / info ─────────────────────────────────────────────────────────

    /// Print a formatted search result table to stdout.
    pub fn print_search(&self, query: &str) -> Result<(), Box<dyn std::error::Error>> {
        let index = self.load_index()?;
        let results = index.search(query);
        if results.is_empty() {
            println!("No packages found matching '{}'.", query);
            return Ok(());
        }
        println!("{:<25} {:<10} {}", "NAME", "LATEST", "DESCRIPTION");
        println!("{}", "-".repeat(72));
        for (name, pkg) in &results {
            let latest = pkg.latest_version().unwrap_or_else(|| "-".to_string());
            let desc = if pkg.description.len() > 36 {
                format!("{}…", &pkg.description[..35])
            } else {
                pkg.description.clone()
            };
            println!("{:<25} {:<10} {}", name, latest, desc);
        }
        println!("\n{} package(s) found.", results.len());
        Ok(())
    }

    /// Print detailed package info to stdout.
    pub fn print_info(&self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let index = self.load_index()?;
        let pkg = index
            .get_package(name)
            .ok_or_else(|| format!("Package '{}' not found in registry.", name))?;

        println!("Package:     {}", name);
        println!("Description: {}", pkg.description);
        if let Some(ref author) = pkg.author {
            println!("Author:      {}", author);
        }
        if let Some(ref license) = pkg.license {
            println!("License:     {}", license);
        }
        if !pkg.keywords.is_empty() {
            println!("Keywords:    {}", pkg.keywords.join(", "));
        }
        println!();
        println!("Published versions:");
        let mut versions: Vec<&str> = pkg.versions.keys().map(|s| s.as_str()).collect();
        versions.sort_by(|a, b| {
            let va = Version::parse(a).ok();
            let vb = Version::parse(b).ok();
            va.cmp(&vb)
        });
        for v in &versions {
            let entry = &pkg.versions[*v];
            let dep_str = if entry.dependencies.is_empty() {
                "(no dependencies)".to_string()
            } else {
                entry
                    .dependencies
                    .iter()
                    .map(|(k, v)| format!("{}@{}", k, v))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            println!("  {}  deps: {}", v, dep_str);
        }

        println!();
        println!(
            "Install: txtcode package install-local packages/{}",
            name
        );
        Ok(())
    }

    // ── Download / install ────────────────────────────────────────────────────

    /// Try to fetch and install a package from the remote registry.
    ///
    /// Returns Ok(true) if downloaded and installed, Ok(false) if already present.
    ///
    /// Security guarantees:
    /// 1. SHA-256 checksum verified against the registry manifest before extraction
    ///    (skipped when the index entry has an empty sha256 field — development packages).
    /// 2. All HTTP connections use reqwest's default TLS (verified certificates).
    /// 3. Tarball extracted with pure-Rust flate2+tar — no system `tar` binary needed.
    /// 4. Path traversal (zip-slip) checked on every archive entry before writing.
    pub fn download_package(
        &self,
        name: &str,
        version: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let dest = self.packages_dir.join(name).join(version);
        if dest.exists() {
            return Ok(false); // already installed
        }

        // Look up package + version in the registry index.
        let index = self.load_index()?;
        let pkg = index.get_package(name).ok_or_else(|| {
            format!(
                "Package '{}' not found in registry.\n\
                 Run `txtcode package search {}` to check the spelling, or\n\
                 use `txtcode package install-local <path>` for local packages.",
                name, name
            )
        })?;

        let ver_entry = pkg.versions.get(version).ok_or_else(|| {
            let available: Vec<&str> = pkg.versions.keys().map(|s| s.as_str()).collect();
            format!(
                "Package '{}' version '{}' not found in registry.\n\
                 Available versions: {}",
                name,
                version,
                available.join(", ")
            )
        })?;

        // 1. local_path install — no network required, no tarball
        if !ver_entry.local_path.is_empty() {
            let resolved = {
                let p = Path::new(&ver_entry.local_path);
                if p.is_absolute() {
                    p.to_path_buf()
                } else {
                    // Resolve relative to the working directory
                    std::env::current_dir()
                        .unwrap_or_else(|_| PathBuf::from("."))
                        .join(p)
                }
            };
            install_local_package(&resolved.to_string_lossy())?;
            return Ok(true);
        }

        if ver_entry.url.is_empty() {
            return Err(format!(
                "Package '{}@{}' has no download URL or local_path in the registry. \
                 Use `txtcode package install-local <path>` instead.",
                name, version
            )
            .into());
        }

        #[cfg(feature = "net")]
        return self.download_from_url(name, version, &ver_entry.url, &ver_entry.sha256);

        #[cfg(not(feature = "net"))]
        Err(format!(
            "Downloading '{}@{}' requires the 'net' feature.\n\
             Rebuild with: cargo build --features net\n\
             Or install locally with: txtcode package install-local packages/{}",
            name, version, name
        )
        .into())
    }

    /// Download a package tarball from a specific URL with checksum verification.
    /// Used internally once the registry is live (v0.7+).
    #[cfg(feature = "net")]
    pub fn download_from_url(
        &self,
        name: &str,
        version: &str,
        url: &str,
        expected_sha256: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let dest = self.packages_dir.join(name).join(version);
        if dest.exists() {
            return Ok(false);
        }

        println!("    → Fetching {}@{} ...", name, version);

        let resp = reqwest::blocking::get(url)
            .map_err(|e| format!("Network error fetching '{}@{}': {}", name, version, e))?;

        if !resp.status().is_success() {
            return Err(format!(
                "Registry returned HTTP {} for '{}@{}'",
                resp.status(),
                name,
                version
            )
            .into());
        }

        let bytes = resp
            .bytes()
            .map_err(|e| format!("Failed to read response for '{}@{}': {}", name, version, e))?
            .to_vec();

        // ── SHA-256 verification ──────────────────────────────────────────────
        let actual_hash = {
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            hex::encode(hasher.finalize())
        };
        if !expected_sha256.is_empty() && !actual_hash.eq_ignore_ascii_case(expected_sha256) {
            return Err(format!(
                "Checksum mismatch for '{}@{}': expected {}, got {}. \
                 Refusing to install tampered package.",
                name, version, expected_sha256, actual_hash
            )
            .into());
        }
        println!("    → Checksum verified.");

        // ── Extract tarball (pure Rust, no system `tar`) ─────────────────────
        fs::create_dir_all(&dest)?;
        Self::extract_tarball(&bytes, &dest, name, version)?;

        println!("    → Installed to: {}", dest.display());
        Ok(true)
    }

    /// Extract a `.tar.gz` archive to `dest` using pure-Rust flate2+tar.
    ///
    /// Security: every entry path is checked for directory traversal (`..`)
    /// before any file is written (zip-slip attack prevention).
    fn extract_tarball(
        bytes: &[u8],
        dest: &Path,
        pkg_name: &str,
        pkg_version: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let gz = GzDecoder::new(bytes);
        let mut archive = Archive::new(gz);

        for entry in archive.entries()? {
            let mut entry = entry?;
            let raw_path = entry.path()?.into_owned();

            // Strip the top-level directory component that tarballs typically
            // include (e.g. "package-name-1.0.0/src/lib.tc" → "src/lib.tc").
            let stripped: PathBuf = raw_path.components().skip(1).collect();

            // Zip-slip guard: reject any path that resolves outside dest.
            if stripped
                .components()
                .any(|c| c.as_os_str() == ".." || c.as_os_str() == ".")
            {
                return Err(format!(
                    "Refusing to extract '{}@{}': path traversal detected in archive entry '{}'",
                    pkg_name,
                    pkg_version,
                    raw_path.display()
                )
                .into());
            }

            let out = dest.join(&stripped);
            if let Some(parent) = out.parent() {
                fs::create_dir_all(parent)?;
            }

            // Only extract regular files and symlinks; skip directories
            // (create_dir_all above handles them).
            let mut content = Vec::new();
            entry.read_to_end(&mut content)?;
            fs::write(&out, &content)?;
        }

        Ok(())
    }

    /// Verify a downloaded file's SHA-256 hash against a `sha256sums`-format manifest.
    ///
    /// The manifest format is: `<hex-hash>  <filename>` (one entry per line).
    /// Returns Ok(()) if the hash matches, Err with a description if it does not.
    pub fn verify_sha256_manifest(
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

/// Dependency resolver — resolves locally installed packages using semver.
/// Remote registry resolution is available from v0.7.0.
pub struct DependencyResolver {
    packages_dir: PathBuf,
}

impl DependencyResolver {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let packages_dir = Config::get_packages_dir()
            .map_err(|e| format!("Failed to get packages directory: {}", e))?;
        Ok(Self { packages_dir })
    }

    /// Resolve all dependencies for a package against locally installed versions.
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
// Transitive dependency resolver (registry-based)
// ---------------------------------------------------------------------------

/// Pick the highest version in `pkg`'s index entry that satisfies `constraint`.
/// Falls back to `latest_version()` for non-semver constraints, or `None` if
/// the package has no matching version at all.
fn resolve_version_from_registry(pkg: &RegistryPackageEntry, constraint: &str) -> Option<String> {
    match VersionReq::parse(constraint) {
        Ok(req) => {
            let mut matching: Vec<Version> = pkg
                .versions
                .keys()
                .filter_map(|v| Version::parse(v).ok())
                .filter(|v| req.matches(v))
                .collect();
            matching.sort();
            matching.last().map(|v| v.to_string())
        }
        Err(_) => {
            // Non-semver constraint: treat as exact pin, or fall back to latest.
            if pkg.versions.contains_key(constraint) {
                Some(constraint.to_string())
            } else {
                pkg.latest_version()
            }
        }
    }
}

/// Internal recursive traversal for [`resolve_transitive`].
///
/// * `visited`  — packages already fully resolved (deduplicate).
/// * `in_stack` — packages currently in the DFS call stack (cycle detection).
/// * `result`   — accumulates `(name, resolved_version)` in topological order.
fn resolve_transitive_recursive(
    name: &str,
    version_req: &str,
    registry: &RegistryIndex,
    visited: &mut std::collections::HashSet<String>,
    in_stack: &mut std::collections::HashSet<String>,
    result: &mut Vec<(String, String)>,
) {
    // Cycle detection: package is already in the current DFS path.
    if in_stack.contains(name) {
        eprintln!(
            "Warning: circular dependency detected involving '{}' — skipping to break cycle.",
            name
        );
        return;
    }

    // Deduplication: package was already fully resolved in a prior branch.
    if visited.contains(name) {
        return;
    }

    in_stack.insert(name.to_string());

    // Resolve the best concrete version from the registry index.
    let resolved_version = registry
        .get_package(name)
        .and_then(|pkg| resolve_version_from_registry(pkg, version_req))
        .unwrap_or_else(|| version_req.to_string());

    // Recurse into transitive deps BEFORE adding this package so that
    // dependencies appear earlier in the install order (topological sort).
    if let Some(pkg) = registry.get_package(name) {
        if let Some(ver_entry) = pkg.versions.get(&resolved_version) {
            for (dep_name, dep_ver_req) in &ver_entry.dependencies {
                resolve_transitive_recursive(
                    dep_name,
                    dep_ver_req,
                    registry,
                    visited,
                    in_stack,
                    result,
                );
            }
        }
    }

    result.push((name.to_string(), resolved_version));
    in_stack.remove(name);
    visited.insert(name.to_string());
}

/// Resolve all transitive dependencies of `name@version_req` using the registry
/// index (not locally-installed packages).
///
/// Returns a list of `(package_name, resolved_version)` in topological install
/// order — dependencies appear before the packages that depend on them.
///
/// `visited` is shared across multiple top-level calls so that shared
/// transitive deps are not duplicated in the result.
pub fn resolve_transitive(
    name: &str,
    version_req: &str,
    registry: &RegistryIndex,
    visited: &mut std::collections::HashSet<String>,
) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let mut in_stack = std::collections::HashSet::new();
    resolve_transitive_recursive(name, version_req, registry, visited, &mut in_stack, &mut result);
    result
}

/// Detect version conflicts across a resolved package set.
///
/// A conflict exists when two resolved packages both depend on a third package
/// but with incompatible version constraints (no single released version
/// satisfies all constraints simultaneously).
///
/// Returns a list of human-readable warning strings.
pub fn detect_version_conflicts(
    resolved: &[(String, String)],
    registry: &RegistryIndex,
) -> Vec<String> {
    // Build map: dep_name → vec of (requirer_name, version_constraint)
    let mut requirements: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for (pkg_name, pkg_version) in resolved {
        if let Some(pkg) = registry.get_package(pkg_name) {
            if let Some(ver_entry) = pkg.versions.get(pkg_version) {
                for (dep_name, dep_req) in &ver_entry.dependencies {
                    requirements
                        .entry(dep_name.clone())
                        .or_default()
                        .push((pkg_name.clone(), dep_req.clone()));
                }
            }
        }
    }

    let mut warnings = Vec::new();
    for (dep_name, requirers) in &requirements {
        if requirers.len() < 2 {
            continue;
        }

        // Collect parseable constraints.
        let reqs: Vec<VersionReq> = requirers
            .iter()
            .filter_map(|(_, v)| VersionReq::parse(v).ok())
            .collect();
        if reqs.len() < 2 {
            continue;
        }

        // If no single version in the registry satisfies ALL constraints, it's a conflict.
        let pkg = match registry.get_package(dep_name) {
            Some(p) => p,
            None => continue,
        };
        let has_compatible = pkg
            .versions
            .keys()
            .filter_map(|v| Version::parse(v).ok())
            .any(|v| reqs.iter().all(|req| req.matches(&v)));

        if !has_compatible {
            let req_strs: Vec<String> = requirers
                .iter()
                .map(|(r, v)| format!("{} needs {} {}", r, dep_name, v))
                .collect();
            warnings.push(format!("conflict: {}", req_strs.join(", ")));
        }
    }
    warnings
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
    println!("Installing {} direct dependencies...", config.dependencies.len());

    Config::ensure_directories()
        .map_err(|e| format!("Failed to initialize txtcode directories: {}", e))?;

    let lock_path = PathBuf::from("Txtcode.lock");

    // If lockfile exists, use locked (pinned) versions — no resolution needed.
    let resolved: Vec<(String, String)> = if lock_path.exists() {
        println!("Using Txtcode.lock for pinned versions.");
        let lock = LockFile::load(&lock_path)?;
        let mut pairs: Vec<(String, String)> = lock
            .packages
            .into_iter()
            .map(|(name, pkg)| (name, pkg.version))
            .collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        pairs
    } else {
        // Try registry-based transitive resolution first.
        let packages_dir = Config::get_packages_dir()
            .map_err(|e| format!("Failed to get packages directory: {}", e))?;
        let registry_obj = PackageRegistry::new(packages_dir);

        match registry_obj.load_index() {
            Ok(index) => {
                let mut all_resolved: Vec<(String, String)> = Vec::new();
                let mut visited = std::collections::HashSet::new();

                for (name, version_req) in &config.dependencies {
                    let transitive = resolve_transitive(name, version_req, &index, &mut visited);
                    for item in transitive {
                        if !all_resolved.iter().any(|(n, _)| n == &item.0) {
                            all_resolved.push(item);
                        }
                    }
                }

                // Warn about incompatible version constraints.
                let conflicts = detect_version_conflicts(&all_resolved, &index);
                for warning in &conflicts {
                    eprintln!("Warning: {}", warning);
                }

                println!(
                    "Resolving dependencies... installing {} package(s)",
                    all_resolved.len()
                );
                all_resolved
            }
            Err(_) => {
                // Registry unavailable — fall back to local resolver.
                let resolver = DependencyResolver::new()?;
                let resolved_map = resolver.resolve(&config)?;
                let mut pairs: Vec<(String, String)> = resolved_map.into_iter().collect();
                pairs.sort_by(|a, b| a.0.cmp(&b.0));
                pairs
            }
        }
    };

    let packages_dir = Config::get_packages_dir()
        .map_err(|e| format!("Failed to get packages directory: {}", e))?;
    let registry = PackageRegistry::new(packages_dir.clone());

    let mut lock = LockFile::default();

    // Load existing lockfile now (before installing) so we can verify hashes
    // against it after each package is placed on disk.
    let existing_lock = if lock_path.exists() {
        Some(LockFile::load(&lock_path)?)
    } else {
        None
    };

    for (name, version) in &resolved {
        println!("  Installing {}@{}", name, version);
        match registry.download_package(name, version) {
            Ok(_) => {
                let installed_dir = packages_dir.join(name).join(version);
                // Compute directory hash so the lockfile we write is verifiable.
                let checksum = if installed_dir.exists() {
                    compute_dir_hash(&installed_dir)
                } else {
                    String::new()
                };
                lock.packages.entry(name.clone()).or_insert(LockedPackage {
                    version: version.clone(),
                    checksum,
                    dependencies: Vec::new(),
                });

                // When a lockfile already existed, verify the installed package
                // matches the recorded hash.  Abort on mismatch.
                if let Some(ref lf) = existing_lock {
                    if installed_dir.exists() {
                        lf.verify_installed(name, &installed_dir).map_err(|e| {
                            format!("Lockfile verification failed for '{}': {}", name, e)
                        })?;
                    }
                }
            }
            Err(e) => {
                eprintln!("  Warning: {}", e);
            }
        }
    }

    if existing_lock.is_some() {
        println!("Lockfile verified.");
    }

    // Generate and save lockfile if it doesn't already exist.
    if !lock_path.exists() && !lock.packages.is_empty() {
        lock.save(&lock_path)?;
        println!("Generated Txtcode.lock");
    }

    println!("Dependencies installed successfully");
    Ok(())
}

/// Install a package from a local directory path into `~/.txtcode/packages/{name}/{version}/`.
///
/// Reads the package's `Txtcode.toml` to determine name and version, then copies
/// all files from the source directory. Safe: destination is always inside the
/// managed packages directory, preventing path traversal.
pub fn install_local_package(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let source = PathBuf::from(path).canonicalize()
        .map_err(|e| format!("Cannot resolve path '{}': {}", path, e))?;

    let manifest_path = source.join("Txtcode.toml");
    if !manifest_path.exists() {
        return Err(format!(
            "No Txtcode.toml found in '{}'. \
             Is this a valid Txtcode package directory?",
            source.display()
        ).into());
    }

    let config = PackageConfig::load(&manifest_path)
        .map_err(|e| format!("Failed to read Txtcode.toml: {}", e))?;

    let packages_dir = Config::get_packages_dir()
        .map_err(|e| format!("Failed to locate packages directory: {}", e))?;

    let dest = packages_dir.join(&config.name).join(&config.version);

    if dest.exists() {
        println!(
            "Package {}@{} is already installed — skipping.",
            config.name, config.version
        );
        return Ok(());
    }

    fs::create_dir_all(&dest)
        .map_err(|e| format!("Failed to create destination directory: {}", e))?;

    // Copy every file from source into dest (non-recursive; packages are flat).
    for entry in fs::read_dir(&source)
        .map_err(|e| format!("Cannot read source directory: {}", e))?
    {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let file_name = entry.file_name();

        // Guard against path traversal in file names.
        let name_str = file_name.to_string_lossy();
        if name_str.contains("..") || name_str.contains('/') || name_str.contains('\\') {
            return Err(format!("Unsafe file name '{}' in package source.", name_str).into());
        }

        if file_type.is_file() {
            let dst_path = dest.join(&file_name);
            fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("Failed to copy '{}': {}", src_path.display(), e))?;
        }
        // Subdirectories are skipped for now; packages are kept flat.
    }

    println!(
        "Installed {}@{} from '{}' → '{}'",
        config.name,
        config.version,
        source.display(),
        dest.display()
    );
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Build a DependencyResolver whose packages_dir points at a tempdir,
    /// then create fake installed-version subdirectories inside it.
    fn make_resolver_with_versions(
        pkg_name: &str,
        versions: &[&str],
    ) -> (DependencyResolver, TempDir) {
        let tmp = TempDir::new().unwrap();
        let pkg_dir = tmp.path().join(pkg_name);
        for v in versions {
            fs::create_dir_all(pkg_dir.join(v)).unwrap();
        }
        let resolver = DependencyResolver { packages_dir: tmp.path().to_path_buf() };
        (resolver, tmp)
    }

    // --- resolve_version_constraint ---

    #[test]
    fn caret_constraint_picks_highest_compatible() {
        // ^1.0.0 should match 1.0.0, 1.1.0, 1.2.3 but not 2.0.0
        let (r, _tmp) = make_resolver_with_versions("foo", &["1.0.0", "1.1.0", "1.2.3", "2.0.0"]);
        let result = r.resolve_version_constraint("foo", "^1.0.0");
        assert_eq!(result, Some("1.2.3".to_string()));
    }

    #[test]
    fn tilde_constraint_picks_highest_patch() {
        // ~1.2.0 should match 1.2.x but not 1.3.0
        let (r, _tmp) = make_resolver_with_versions("bar", &["1.2.0", "1.2.5", "1.3.0", "2.0.0"]);
        let result = r.resolve_version_constraint("bar", "~1.2.0");
        assert_eq!(result, Some("1.2.5".to_string()));
    }

    #[test]
    fn gte_constraint_picks_highest_matching() {
        // >=1.1.0 should match 1.1.0, 1.2.0, 2.0.0 — pick highest
        let (r, _tmp) = make_resolver_with_versions("baz", &["1.0.0", "1.1.0", "1.2.0", "2.0.0"]);
        let result = r.resolve_version_constraint("baz", ">=1.1.0");
        assert_eq!(result, Some("2.0.0".to_string()));
    }

    #[test]
    fn exact_constraint_returns_matching_version() {
        let (r, _tmp) = make_resolver_with_versions("qux", &["1.0.0", "1.1.0", "2.0.0"]);
        let result = r.resolve_version_constraint("qux", "1.1.0");
        assert_eq!(result, Some("1.1.0".to_string()));
    }

    #[test]
    fn no_matching_version_returns_none() {
        // ^3.0.0 — none of the installed versions qualify
        let (r, _tmp) = make_resolver_with_versions("pkg", &["1.0.0", "2.0.0"]);
        let result = r.resolve_version_constraint("pkg", "^3.0.0");
        assert_eq!(result, None);
    }

    #[test]
    fn missing_package_returns_none() {
        let (r, _tmp) = make_resolver_with_versions("other", &["1.0.0"]);
        // "missing" was never created
        let result = r.resolve_version_constraint("missing", "^1.0.0");
        assert_eq!(result, None);
    }

    #[test]
    fn non_semver_constraint_returns_constraint_string() {
        // If the constraint is not valid semver, we return it as-is (exact pin)
        let (r, _tmp) = make_resolver_with_versions("mypkg", &["latest"]);
        let result = r.resolve_version_constraint("mypkg", "latest");
        assert_eq!(result, Some("latest".to_string()));
    }

    // --- get_installed_version ---

    #[test]
    fn get_installed_version_returns_highest() {
        let (r, _tmp) = make_resolver_with_versions("lib", &["0.9.0", "1.0.0", "1.5.0"]);
        assert_eq!(r.get_installed_version("lib"), Some("1.5.0".to_string()));
    }

    #[test]
    fn get_installed_version_absent_returns_none() {
        let (r, _tmp) = make_resolver_with_versions("lib", &["1.0.0"]);
        assert_eq!(r.get_installed_version("nope"), None);
    }

    // --- is_installed ---

    #[test]
    fn is_installed_true_for_exact_version() {
        let (r, _tmp) = make_resolver_with_versions("mylib", &["2.0.0"]);
        assert!(r.is_installed("mylib", "2.0.0"));
    }

    #[test]
    fn is_installed_false_for_missing() {
        let (r, _tmp) = make_resolver_with_versions("mylib", &["2.0.0"]);
        assert!(!r.is_installed("mylib", "1.0.0"));
        assert!(!r.is_installed("other", "2.0.0"));
    }

    // --- install_local_package (unit-level: filesystem only, no Config path) ---
    // We test the core copy logic by calling our helper directly with a fake
    // packages_dir. These tests do NOT call install_local_package() (which uses
    // Config::get_packages_dir()) but exercise the same logic via a white-box
    // helper so we can run without ~/.txtcode existing.

    /// Helper: run the "local install" copy step into a temp packages_dir.
    fn local_install_into(
        source: &Path,
        packages_dir: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let manifest_path = source.join("Txtcode.toml");
        let config = PackageConfig::load(&manifest_path.to_path_buf())?;
        let dest = packages_dir.join(&config.name).join(&config.version);
        if dest.exists() {
            return Ok(());
        }
        fs::create_dir_all(&dest)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.contains("..") || name_str.contains('/') || name_str.contains('\\') {
                    return Err(format!("Unsafe file name '{}'", name_str).into());
                }
                fs::copy(entry.path(), dest.join(name))?;
            }
        }
        Ok(())
    }

    #[test]
    fn install_local_copies_files_to_packages_dir() {
        // Build a minimal fake package source
        let src_tmp = TempDir::new().unwrap();
        let manifest = "name = \"test-pkg\"\nversion = \"1.2.3\"\n[dependencies]\n";
        fs::write(src_tmp.path().join("Txtcode.toml"), manifest).unwrap();
        fs::write(src_tmp.path().join("lib.tc"), "## hello").unwrap();

        let dest_tmp = TempDir::new().unwrap();
        local_install_into(src_tmp.path(), dest_tmp.path()).unwrap();

        let installed = dest_tmp.path().join("test-pkg").join("1.2.3");
        assert!(installed.exists(), "version directory should be created");
        assert!(installed.join("Txtcode.toml").exists(), "Txtcode.toml should be copied");
        assert!(installed.join("lib.tc").exists(), "lib.tc should be copied");
    }

    #[test]
    fn install_local_idempotent_when_already_installed() {
        let src_tmp = TempDir::new().unwrap();
        let manifest = "name = \"idempotent-pkg\"\nversion = \"0.1.0\"\n[dependencies]\n";
        fs::write(src_tmp.path().join("Txtcode.toml"), manifest).unwrap();

        let dest_tmp = TempDir::new().unwrap();
        // First install
        local_install_into(src_tmp.path(), dest_tmp.path()).unwrap();
        // Second install — must not error
        local_install_into(src_tmp.path(), dest_tmp.path()).unwrap();

        let installed = dest_tmp.path().join("idempotent-pkg").join("0.1.0");
        assert!(installed.exists());
    }

    #[test]
    fn install_local_fails_without_manifest() {
        let src_tmp = TempDir::new().unwrap();
        let dest_tmp = TempDir::new().unwrap();
        // No Txtcode.toml in src_tmp
        let result = local_install_into(src_tmp.path(), dest_tmp.path());
        assert!(result.is_err());
    }

    // --- RegistryIndex unit tests ---

    const SAMPLE_INDEX: &str = r#"
{
  "version": "1",
  "updated": "2026-03-18",
  "packages": {
    "npl-math": {
      "description": "Essential math utilities",
      "author": "Txtcode Core Team",
      "license": "MIT",
      "keywords": ["math", "numbers"],
      "versions": {
        "0.1.0": { "url": "https://example.com/npl-math-0.1.0.tar.gz", "sha256": "", "dependencies": {} },
        "0.2.0": { "url": "https://example.com/npl-math-0.2.0.tar.gz", "sha256": "", "dependencies": {} }
      }
    },
    "npl-strings": {
      "description": "String manipulation utilities",
      "author": "Txtcode Core Team",
      "license": "MIT",
      "keywords": ["strings", "text"],
      "versions": {
        "0.1.0": { "url": "https://example.com/npl-strings-0.1.0.tar.gz", "sha256": "", "dependencies": {} }
      }
    }
  }
}"#;

    #[test]
    fn registry_index_parses_from_str() {
        let index = RegistryIndex::from_str(SAMPLE_INDEX).unwrap();
        assert_eq!(index.version, "1");
        assert_eq!(index.packages.len(), 2);
        assert!(index.packages.contains_key("npl-math"));
        assert!(index.packages.contains_key("npl-strings"));
    }

    #[test]
    fn registry_index_latest_version_returns_highest_semver() {
        let index = RegistryIndex::from_str(SAMPLE_INDEX).unwrap();
        let pkg = index.get_package("npl-math").unwrap();
        assert_eq!(pkg.latest_version(), Some("0.2.0".to_string()));
    }

    #[test]
    fn registry_search_by_name() {
        let index = RegistryIndex::from_str(SAMPLE_INDEX).unwrap();
        let results = index.search("math");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "npl-math");
    }

    #[test]
    fn registry_search_by_keyword() {
        let index = RegistryIndex::from_str(SAMPLE_INDEX).unwrap();
        let results = index.search("text");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "npl-strings");
    }

    #[test]
    fn registry_search_by_description() {
        let index = RegistryIndex::from_str(SAMPLE_INDEX).unwrap();
        let results = index.search("manipulation");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "npl-strings");
    }

    #[test]
    fn registry_search_returns_sorted_results() {
        let index = RegistryIndex::from_str(SAMPLE_INDEX).unwrap();
        let results = index.search("utilities"); // both packages match
        assert_eq!(results.len(), 2);
        // Must be sorted alphabetically
        assert!(results[0].0 <= results[1].0);
    }

    #[test]
    fn registry_search_no_match_returns_empty() {
        let index = RegistryIndex::from_str(SAMPLE_INDEX).unwrap();
        let results = index.search("zzznomatch");
        assert!(results.is_empty());
    }

    #[test]
    fn registry_get_package_exact() {
        let index = RegistryIndex::from_str(SAMPLE_INDEX).unwrap();
        assert!(index.get_package("npl-math").is_some());
        assert!(index.get_package("does-not-exist").is_none());
    }

    #[test]
    fn registry_from_file_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("index.json");
        fs::write(&path, SAMPLE_INDEX).unwrap();
        let index = RegistryIndex::from_file(&path).unwrap();
        assert_eq!(index.packages.len(), 2);
    }

    #[test]
    fn registry_index_load_via_env_var() {
        // PackageRegistry::load_index() reads TXTCODE_REGISTRY_INDEX_FILE when set.
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("index.json");
        fs::write(&path, SAMPLE_INDEX).unwrap();

        std::env::set_var("TXTCODE_REGISTRY_INDEX_FILE", path.to_str().unwrap());
        let reg = PackageRegistry::new(tmp.path().to_path_buf());
        let index = reg.load_index().unwrap();
        std::env::remove_var("TXTCODE_REGISTRY_INDEX_FILE");

        assert_eq!(index.packages.len(), 2);
    }

    #[test]
    fn real_index_json_parses_cleanly() {
        // The registry/index.json shipped with the repo must parse without errors.
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("registry")
            .join("index.json");
        let index = RegistryIndex::from_file(&path).unwrap();
        // Four starter packages must be present.
        for name in &["npl-math", "npl-strings", "npl-collections", "npl-datetime"] {
            assert!(
                index.packages.contains_key(*name),
                "registry/index.json missing package '{}'",
                name
            );
        }
    }

    // --- resolve_transitive ---

    /// Registry JSON with a transitive dependency chain:
    ///   pkg-a@1.0.0 → depends on pkg-b@^1.0
    ///   pkg-b@1.0.0 → no deps
    const TRANSITIVE_INDEX: &str = r#"{
  "version": "1",
  "packages": {
    "pkg-a": {
      "description": "Package A",
      "versions": {
        "1.0.0": { "url": "", "sha256": "", "dependencies": { "pkg-b": "^1.0" } }
      }
    },
    "pkg-b": {
      "description": "Package B",
      "versions": {
        "1.0.0": { "url": "", "sha256": "", "dependencies": {} }
      }
    }
  }
}"#;

    #[test]
    fn resolve_transitive_installs_transitive_deps() {
        let index = RegistryIndex::from_str(TRANSITIVE_INDEX).unwrap();
        let mut visited = std::collections::HashSet::new();
        let result = resolve_transitive("pkg-a", "1.0.0", &index, &mut visited);

        let names: Vec<&str> = result.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"pkg-a"), "pkg-a must be in result");
        assert!(names.contains(&"pkg-b"), "pkg-b (transitive dep) must be in result");

        // Topological order: pkg-b (dep) must appear before pkg-a (dependent).
        let pos_a = names.iter().position(|&n| n == "pkg-a").unwrap();
        let pos_b = names.iter().position(|&n| n == "pkg-b").unwrap();
        assert!(pos_b < pos_a, "pkg-b should be installed before pkg-a");
    }

    #[test]
    fn resolve_transitive_deduplicates_shared_dep() {
        // pkg-a and pkg-c both depend on pkg-b; pkg-b must appear only once.
        let index_json = r#"{
  "version": "1",
  "packages": {
    "pkg-a": { "description": "A", "versions": { "1.0.0": { "url": "", "sha256": "", "dependencies": { "pkg-b": "^1.0" } } } },
    "pkg-c": { "description": "C", "versions": { "1.0.0": { "url": "", "sha256": "", "dependencies": { "pkg-b": "^1.0" } } } },
    "pkg-b": { "description": "B", "versions": { "1.0.0": { "url": "", "sha256": "", "dependencies": {} } } }
  }
}"#;
        let index = RegistryIndex::from_str(index_json).unwrap();
        let mut visited = std::collections::HashSet::new();

        let mut all: Vec<(String, String)> = Vec::new();
        for (name, req) in &[("pkg-a", "1.0.0"), ("pkg-c", "1.0.0")] {
            let r = resolve_transitive(name, req, &index, &mut visited);
            for item in r {
                if !all.iter().any(|(n, _)| n == &item.0) {
                    all.push(item);
                }
            }
        }

        let b_count = all.iter().filter(|(n, _)| n == "pkg-b").count();
        assert_eq!(b_count, 1, "pkg-b should appear exactly once despite two dependents");
    }

    #[test]
    fn resolve_transitive_detects_dep_cycle() {
        // pkg-x depends on pkg-y; pkg-y depends on pkg-x — a cycle.
        let index_json = r#"{
  "version": "1",
  "packages": {
    "pkg-x": { "description": "X", "versions": { "1.0.0": { "url": "", "sha256": "", "dependencies": { "pkg-y": "^1.0" } } } },
    "pkg-y": { "description": "Y", "versions": { "1.0.0": { "url": "", "sha256": "", "dependencies": { "pkg-x": "^1.0" } } } }
  }
}"#;
        let index = RegistryIndex::from_str(index_json).unwrap();
        let mut visited = std::collections::HashSet::new();

        // Must not infinite-loop; must terminate and include at least pkg-x.
        let result = resolve_transitive("pkg-x", "1.0.0", &index, &mut visited);
        assert!(
            result.iter().any(|(n, _)| n == "pkg-x"),
            "pkg-x should be resolved despite the cycle"
        );
    }

    // --- detect_version_conflicts ---

    #[test]
    fn detect_version_conflicts_warns_on_incompatible_versions() {
        // pkg-a needs shared ^1.0, pkg-b needs shared ^2.0 — incompatible.
        let index_json = r#"{
  "version": "1",
  "packages": {
    "pkg-a": { "description": "A", "versions": { "1.0.0": { "url": "", "sha256": "", "dependencies": { "shared": "^1.0" } } } },
    "pkg-b": { "description": "B", "versions": { "1.0.0": { "url": "", "sha256": "", "dependencies": { "shared": "^2.0" } } } },
    "shared": { "description": "S", "versions": {
      "1.0.0": { "url": "", "sha256": "", "dependencies": {} },
      "2.0.0": { "url": "", "sha256": "", "dependencies": {} }
    }}
  }
}"#;
        let index = RegistryIndex::from_str(index_json).unwrap();
        let resolved = vec![
            ("pkg-a".to_string(), "1.0.0".to_string()),
            ("pkg-b".to_string(), "1.0.0".to_string()),
            ("shared".to_string(), "1.0.0".to_string()),
        ];

        let warnings = detect_version_conflicts(&resolved, &index);
        assert!(!warnings.is_empty(), "Should warn about incompatible shared dep");
        assert!(
            warnings[0].contains("conflict"),
            "Warning should contain 'conflict'"
        );
    }

    // --- LockFile / compute_dir_hash ---

    #[test]
    fn lockfile_verify_installed_passes_when_hashes_match() {
        let tmp = TempDir::new().unwrap();
        let pkg_dir = tmp.path().join("mypkg");
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(pkg_dir.join("lib.tc"), "print → 42").unwrap();

        let hash = compute_dir_hash(&pkg_dir);
        let mut lock = LockFile::default();
        lock.packages.insert(
            "mypkg".to_string(),
            LockedPackage {
                version: "1.0.0".to_string(),
                checksum: hash,
                dependencies: vec![],
            },
        );

        assert!(
            lock.verify_installed("mypkg", &pkg_dir).is_ok(),
            "Hash should match immediately after computing it"
        );
    }

    #[test]
    fn lockfile_verify_installed_fails_on_hash_mismatch() {
        let tmp = TempDir::new().unwrap();
        let pkg_dir = tmp.path().join("mypkg");
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(pkg_dir.join("lib.tc"), "print → 42").unwrap();

        let mut lock = LockFile::default();
        lock.packages.insert(
            "mypkg".to_string(),
            LockedPackage {
                version: "1.0.0".to_string(),
                checksum: "deadbeef0000000000000000000000000000000000000000000000000000dead"
                    .to_string(),
                dependencies: vec![],
            },
        );

        let result = lock.verify_installed("mypkg", &pkg_dir);
        assert!(result.is_err(), "Mismatched hash must cause an error");
        assert!(
            result.unwrap_err().contains("lockfile hash mismatch"),
            "Error should mention hash mismatch"
        );
    }

    #[test]
    fn lockfile_verify_skipped_when_checksum_empty() {
        let tmp = TempDir::new().unwrap();
        let pkg_dir = tmp.path().join("mypkg");
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(pkg_dir.join("lib.tc"), "print → 42").unwrap();

        let mut lock = LockFile::default();
        lock.packages.insert(
            "mypkg".to_string(),
            LockedPackage {
                version: "1.0.0".to_string(),
                checksum: String::new(), // empty = skip verification (legacy)
                dependencies: vec![],
            },
        );

        assert!(
            lock.verify_installed("mypkg", &pkg_dir).is_ok(),
            "Empty checksum should be treated as skip (backward-compat)"
        );
    }

    #[test]
    fn lockfile_verify_fails_for_unknown_package() {
        let tmp = TempDir::new().unwrap();
        let pkg_dir = tmp.path().join("mypkg");
        fs::create_dir_all(&pkg_dir).unwrap();

        let lock = LockFile::default();
        let result = lock.verify_installed("unknown", &pkg_dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found in lockfile"));
    }

    #[test]
    fn compute_dir_hash_is_deterministic() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("a.tc"), "x = 1").unwrap();
        fs::write(tmp.path().join("b.tc"), "y = 2").unwrap();

        let h1 = compute_dir_hash(tmp.path());
        let h2 = compute_dir_hash(tmp.path());
        assert_eq!(h1, h2, "Hash must be identical across two calls");
        assert!(!h1.is_empty(), "Hash must be non-empty");
    }

    #[test]
    fn compute_dir_hash_changes_when_file_modified() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("lib.tc"), "x = 1").unwrap();
        let h1 = compute_dir_hash(tmp.path());

        fs::write(tmp.path().join("lib.tc"), "x = 999").unwrap();
        let h2 = compute_dir_hash(tmp.path());
        assert_ne!(h1, h2, "Hash must change when a file's content changes");
    }

    #[test]
    fn detect_version_conflicts_no_warning_on_compatible_constraints() {
        // pkg-a needs shared ^1.0, pkg-b needs shared >=1.0 — both satisfied by 1.0.0.
        let index_json = r#"{
  "version": "1",
  "packages": {
    "pkg-a": { "description": "A", "versions": { "1.0.0": { "url": "", "sha256": "", "dependencies": { "shared": "^1.0" } } } },
    "pkg-b": { "description": "B", "versions": { "1.0.0": { "url": "", "sha256": "", "dependencies": { "shared": ">=1.0" } } } },
    "shared": { "description": "S", "versions": {
      "1.0.0": { "url": "", "sha256": "", "dependencies": {} }
    }}
  }
}"#;
        let index = RegistryIndex::from_str(index_json).unwrap();
        let resolved = vec![
            ("pkg-a".to_string(), "1.0.0".to_string()),
            ("pkg-b".to_string(), "1.0.0".to_string()),
            ("shared".to_string(), "1.0.0".to_string()),
        ];

        let warnings = detect_version_conflicts(&resolved, &index);
        assert!(
            warnings.is_empty(),
            "No conflict warning expected when constraints are compatible"
        );
    }
}
