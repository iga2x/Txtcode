//! Txtcode Community Package Registry Server
//!
//! A minimal HTTP registry that serves the package index and accepts package uploads.
//!
//! Endpoints:
//!   GET  /api/v1/packages               → list all packages (JSON)
//!   GET  /api/v1/packages/:name         → package info (JSON)
//!   GET  /api/v1/packages/:name/:version/download → tarball download
//!   POST /api/v1/packages               → publish a package (bearer token auth)
//!   GET  /api/v1/search?q=<query>       → search packages
//!   GET  /health                        → health check
//!
//! Usage:
//!   txtcode-registry --port 8080 --data /var/lib/txtcode-registry
//!
//! Environment variables:
//!   REGISTRY_PORT     — TCP port (default: 8080)
//!   REGISTRY_DATA_DIR — directory for index + tarballs (default: ./registry-data)
//!   REGISTRY_TOKEN    — API token required for publishing (set to enable auth)

use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// Registry state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct PackageEntry {
    name: String,
    description: String,
    author: String,
    license: String,
    keywords: Vec<String>,
    versions: Vec<VersionEntry>,
}

#[derive(Debug, Clone)]
struct VersionEntry {
    version: String,
    sha256: String,
    published_at: String,
    tarball_path: String,
}

struct Registry {
    packages: HashMap<String, PackageEntry>,
    data_dir: PathBuf,
    api_token: Option<String>,
}

impl Registry {
    fn new(data_dir: PathBuf, api_token: Option<String>) -> Self {
        let mut r = Self {
            packages: HashMap::new(),
            data_dir: data_dir.clone(),
            api_token,
        };
        let _ = r.load_index();
        r
    }

    fn load_index(&mut self) -> Result<(), String> {
        let index_path = self.data_dir.join("index.json");
        if !index_path.exists() {
            return Ok(());
        }
        let content = fs::read_to_string(&index_path)
            .map_err(|e| format!("Failed to read index: {}", e))?;
        self.packages = parse_index_json(&content);
        Ok(())
    }

    fn save_index(&self) -> Result<(), String> {
        let index_path = self.data_dir.join("index.json");
        let mut obj = String::from("{\n  \"version\": \"1\",\n  \"packages\": {\n");
        let entries: Vec<String> = self.packages.values().map(|pkg| {
            let keywords = pkg.keywords.iter()
                .map(|k| format!("\"{}\"", k))
                .collect::<Vec<_>>()
                .join(", ");
            let versions: Vec<String> = pkg.versions.iter().map(|v| {
                format!(
                    "      \"{}\": {{ \"sha256\": \"{}\", \"published_at\": \"{}\", \"tarball\": \"/api/v1/packages/{}/{}/download\" }}",
                    v.version, v.sha256, v.published_at, pkg.name, v.version
                )
            }).collect();
            format!(
                "    \"{}\": {{\n      \"description\": \"{}\",\n      \"author\": \"{}\",\n      \"license\": \"{}\",\n      \"keywords\": [{}],\n      \"versions\": {{\n{}\n      }}\n    }}",
                pkg.name, pkg.description, pkg.author, pkg.license, keywords, versions.join(",\n")
            )
        }).collect();
        obj.push_str(&entries.join(",\n"));
        obj.push_str("\n  }\n}\n");
        fs::write(&index_path, obj).map_err(|e| format!("Failed to save index: {}", e))
    }

    fn search(&self, query: &str) -> Vec<&PackageEntry> {
        let q = query.to_lowercase();
        let mut results: Vec<&PackageEntry> = self.packages.values()
            .filter(|p| {
                p.name.to_lowercase().contains(&q)
                || p.description.to_lowercase().contains(&q)
                || p.keywords.iter().any(|k| k.to_lowercase().contains(&q))
            })
            .collect();
        results.sort_by(|a, b| a.name.cmp(&b.name));
        results
    }

    fn verify_token(&self, token: &str) -> bool {
        match &self.api_token {
            Some(t) => t == token,
            None => true, // No auth configured — allow all
        }
    }
}

// ---------------------------------------------------------------------------
// HTTP framing (minimal, no external deps)
// ---------------------------------------------------------------------------

fn read_request(stream: &mut TcpStream) -> Option<(String, String, HashMap<String, String>, Vec<u8>)> {
    let mut buf = [0u8; 8192];
    let n = stream.read(&mut buf).ok()?;
    let raw = String::from_utf8_lossy(&buf[..n]);

    let mut lines = raw.lines();
    let request_line = lines.next()?;
    let parts: Vec<&str> = request_line.splitn(3, ' ').collect();
    if parts.len() < 3 { return None; }
    let method = parts[0].to_string();
    let path = parts[1].to_string();

    let mut headers = HashMap::new();
    let mut content_length = 0usize;
    for line in lines.by_ref() {
        if line.is_empty() { break; }
        if let Some((k, v)) = line.split_once(": ") {
            let key = k.to_lowercase();
            if key == "content-length" {
                content_length = v.trim().parse().unwrap_or(0);
            }
            headers.insert(key, v.to_string());
        }
    }

    // Body: extract from raw buffer after headers
    let header_end = raw.find("\r\n\r\n").map(|p| p + 4)
        .or_else(|| raw.find("\n\n").map(|p| p + 2))
        .unwrap_or(n);
    let body_start = header_end.min(n);
    let body = buf[body_start..n.min(body_start + content_length)].to_vec();

    Some((method, path, headers, body))
}

fn write_response(stream: &mut TcpStream, status: u16, status_text: &str, content_type: &str, body: &[u8]) {
    let header = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
        status, status_text, content_type, body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body);
}

fn json_response(stream: &mut TcpStream, status: u16, body: &str) {
    write_response(stream, status, if status == 200 { "OK" } else { "Error" }, "application/json", body.as_bytes());
}

// ---------------------------------------------------------------------------
// Request routing
// ---------------------------------------------------------------------------

fn handle_request(
    mut stream: TcpStream,
    registry: Arc<Mutex<Registry>>,
) {
    let Some((method, path, headers, body)) = read_request(&mut stream) else { return };

    // Strip query string for routing
    let (route, query) = path.split_once('?').unwrap_or((&path, ""));
    let segments: Vec<&str> = route.trim_start_matches('/').split('/').collect();

    match (method.as_str(), segments.as_slice()) {
        ("GET", ["health"]) => {
            json_response(&mut stream, 200, r#"{"status":"ok","service":"txtcode-registry"}"#);
        }

        ("GET", ["api", "v1", "packages"]) => {
            let reg = registry.lock().unwrap();
            let mut arr = Vec::new();
            let mut pkgs: Vec<&PackageEntry> = reg.packages.values().collect();
            pkgs.sort_by(|a, b| a.name.cmp(&b.name));
            for pkg in pkgs {
                arr.push(format!(
                    "{{\"name\":\"{}\",\"description\":\"{}\",\"latest\":\"{}\"}}",
                    pkg.name, pkg.description,
                    pkg.versions.last().map(|v| v.version.as_str()).unwrap_or("0.1.0")
                ));
            }
            json_response(&mut stream, 200, &format!("[{}]", arr.join(",")));
        }

        ("GET", ["api", "v1", "packages", name]) => {
            let reg = registry.lock().unwrap();
            if let Some(pkg) = reg.packages.get(*name) {
                let versions: Vec<String> = pkg.versions.iter().map(|v| {
                    format!("\"{}\":{{\"sha256\":\"{}\",\"published_at\":\"{}\"}}", v.version, v.sha256, v.published_at)
                }).collect();
                let body = format!(
                    "{{\"name\":\"{}\",\"description\":\"{}\",\"author\":\"{}\",\"license\":\"{}\",\"versions\":{{{}}}}}",
                    pkg.name, pkg.description, pkg.author, pkg.license, versions.join(",")
                );
                json_response(&mut stream, 200, &body);
            } else {
                json_response(&mut stream, 404, &format!("{{\"error\":\"Package '{}' not found\"}}", name));
            }
        }

        ("GET", ["api", "v1", "packages", name, version, "download"]) => {
            let reg = registry.lock().unwrap();
            if let Some(pkg) = reg.packages.get(*name) {
                if let Some(ver) = pkg.versions.iter().find(|v| v.version == *version) {
                    let tarball_path = reg.data_dir.join(&ver.tarball_path);
                    drop(reg);
                    if let Ok(data) = fs::read(&tarball_path) {
                        write_response(&mut stream, 200, "OK", "application/gzip", &data);
                    } else {
                        json_response(&mut stream, 404, "{\"error\":\"Tarball not found\"}");
                    }
                } else {
                    json_response(&mut stream, 404, &format!("{{\"error\":\"Version {} not found\"}}", version));
                }
            } else {
                json_response(&mut stream, 404, &format!("{{\"error\":\"Package '{}' not found\"}}", name));
            }
        }

        ("POST", ["api", "v1", "packages"]) => {
            // Auth check
            let token = headers.get("authorization")
                .and_then(|v| v.strip_prefix("Bearer "))
                .unwrap_or("");
            let reg = registry.lock().unwrap();
            if !reg.verify_token(token) {
                drop(reg);
                json_response(&mut stream, 401, "{\"error\":\"Unauthorized\"}");
                return;
            }
            drop(reg);

            // Parse multipart or JSON metadata (simplified: expect JSON body)
            let body_str = String::from_utf8_lossy(&body);
            // Extract name and version from JSON body (naive parse)
            let get_str = |key: &str| -> String {
                let needle = format!("\"{}\":", key);
                body_str.find(&needle)
                    .and_then(|p| {
                        let rest = body_str[p + needle.len()..].trim_start();
                        if let Some(inner) = rest.strip_prefix('"') {
                            inner.find('"').map(|e| inner[..e].to_string())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default()
            };

            let name = get_str("name");
            let version = get_str("version");
            let sha256 = get_str("sha256");

            if name.is_empty() || version.is_empty() {
                json_response(&mut stream, 400, "{\"error\":\"name and version are required\"}");
                return;
            }

            let now = chrono_now();
            let tarball_rel = format!("tarballs/{}-{}.tar.gz", name, version);

            let mut reg = registry.lock().unwrap();
            let pkg = reg.packages.entry(name.clone()).or_insert_with(|| PackageEntry {
                name: name.clone(),
                description: get_str("description"),
                author: get_str("author"),
                license: get_str("license").or_else_if_empty("MIT"),
                keywords: Vec::new(),
                versions: Vec::new(),
            });
            if pkg.versions.iter().any(|v| v.version == version) {
                json_response(&mut stream, 409, &format!("{{\"error\":\"Version {} already published\"}}", version));
                return;
            }
            pkg.versions.push(VersionEntry {
                version: version.clone(),
                sha256,
                published_at: now.clone(),
                tarball_path: tarball_rel,
            });
            let _ = reg.save_index();
            json_response(&mut stream, 201, &format!(
                "{{\"message\":\"Published {}/{} at {}\"}}", name, version, now
            ));
        }

        ("GET", ["api", "v1", "search"]) => {
            let q = query.strip_prefix("q=").unwrap_or(query);
            let q = urldecode(q);
            let reg = registry.lock().unwrap();
            let results = reg.search(&q);
            let arr: Vec<String> = results.iter().map(|p| {
                format!("{{\"name\":\"{}\",\"description\":\"{}\"}}", p.name, p.description)
            }).collect();
            json_response(&mut stream, 200, &format!("[{}]", arr.join(",")));
        }

        _ => {
            json_response(&mut stream, 404, "{\"error\":\"Not found\"}");
        }
    }
}

// ---------------------------------------------------------------------------
// Index JSON parser (parses the format produced by save_index())
// ---------------------------------------------------------------------------

/// Extract the first quoted string from a JSON fragment.
fn idx_first_quoted(s: &str) -> String {
    if let Some(start) = s.find('"') {
        let rest = &s[start + 1..];
        if let Some(end) = rest.find('"') {
            return rest[..end].to_string();
        }
    }
    String::new()
}

/// Extract the string value for `"key": "value"` in a JSON fragment.
fn idx_str_val(s: &str, key: &str) -> String {
    let needle = format!("\"{}\"", key);
    let pos = match s.find(&needle) {
        Some(p) => p,
        None => return String::new(),
    };
    let after = s[pos + needle.len()..].trim_start();
    let after = match after.strip_prefix(':') {
        Some(a) => a.trim_start(),
        None => return String::new(),
    };
    if let Some(inner) = after.strip_prefix('"') {
        if let Some(end) = inner.find('"') {
            return inner[..end].to_string();
        }
    }
    String::new()
}

/// Extract an array of strings for `"key": ["a", "b"]` in a JSON fragment.
fn idx_str_array(s: &str, key: &str) -> Vec<String> {
    let needle = format!("\"{}\"", key);
    let pos = match s.find(&needle) {
        Some(p) => p,
        None => return Vec::new(),
    };
    let after = &s[pos + needle.len()..];
    let start = match after.find('[') {
        Some(p) => p + 1,
        None => return Vec::new(),
    };
    let arr = &after[start..];
    let end = match arr.find(']') {
        Some(p) => p,
        None => return Vec::new(),
    };
    arr[..end]
        .split(',')
        .filter_map(|item| {
            let t = item.trim().trim_matches('"');
            if t.is_empty() { None } else { Some(t.to_string()) }
        })
        .collect()
}

/// Parse a single-line version entry:
/// `"1.0.0": { "sha256": "...", "published_at": "...", "tarball": "..." }`
fn idx_parse_version(line: &str, pkg_name: &str) -> Option<VersionEntry> {
    let version = idx_first_quoted(line);
    if version.is_empty() { return None; }
    let sha256 = idx_str_val(line, "sha256");
    let published_at = idx_str_val(line, "published_at");
    let tarball_path = {
        let t = idx_str_val(line, "tarball");
        if t.is_empty() {
            format!("tarballs/{}-{}.tar.gz", pkg_name, version)
        } else {
            t
        }
    };
    Some(VersionEntry { version, sha256, published_at, tarball_path })
}

/// Parse the JSON index produced by `save_index()`.
///
/// Format (2-space outer indent, 4 for packages, 6 for fields, 8 for versions):
/// ```json
/// { "version": "1", "packages": {
///     "name": { "description": "...", "author": "...", "license": "...",
///               "keywords": [...], "versions": {
///         "1.0.0": { "sha256": "...", "published_at": "...", "tarball": "..." }
///     }}
/// }}
/// ```
fn parse_index_json(content: &str) -> HashMap<String, PackageEntry> {
    let mut packages = HashMap::new();
    let mut current_pkg: Option<PackageEntry> = None;
    let mut in_packages = false;
    let mut in_versions = false;

    for line in content.lines() {
        // Detect start of "packages" block.
        if !in_packages {
            if line.contains("\"packages\"") && line.contains('{') {
                in_packages = true;
            }
            continue;
        }

        let leading = line.len() - line.trim_start_matches(' ').len();
        let trimmed = line.trim();

        if trimmed.is_empty() { continue; }

        // save_index() indentation:
        //   4 sp → package name:   `    "name": {`
        //   6 sp → package fields: `      "description": "..."` and
        //          versions header: `      "versions": {`   and
        //          version entries: `      "1.0.0": { ... }` (when in_versions)
        //          close versions:  `      }`
        //   4 sp → close package:  `    }`
        //   2 sp → close packages: `  }`
        match leading {
            4 => {
                if trimmed.starts_with('"') {
                    // New package entry: save the previous one first.
                    if let Some(pkg) = current_pkg.take() {
                        packages.insert(pkg.name.clone(), pkg);
                    }
                    in_versions = false;
                    let current_name = idx_first_quoted(trimmed);
                    if !current_name.is_empty() {
                        current_pkg = Some(PackageEntry {
                            name: current_name,
                            description: String::new(),
                            author: String::new(),
                            license: String::new(),
                            keywords: Vec::new(),
                            versions: Vec::new(),
                        });
                    }
                } else if trimmed.starts_with('}') {
                    // Closing brace of the last package before packages-block closes.
                    if let Some(pkg) = current_pkg.take() {
                        packages.insert(pkg.name.clone(), pkg);
                    }
                }
            }
            6 => {
                if in_versions {
                    // Inside the "versions" object.
                    if trimmed.starts_with('}') {
                        // Closing brace of versions object.
                        in_versions = false;
                    } else if trimmed.starts_with('"') {
                        // Version entry: `"1.0.0": { "sha256": "...", ... }`
                        if let Some(pkg) = current_pkg.as_mut() {
                            if let Some(ver) = idx_parse_version(trimmed, &pkg.name) {
                                pkg.versions.push(ver);
                            }
                        }
                    }
                } else if let Some(pkg) = current_pkg.as_mut() {
                    if trimmed.starts_with("\"description\"") {
                        pkg.description = idx_str_val(trimmed, "description");
                    } else if trimmed.starts_with("\"author\"") {
                        pkg.author = idx_str_val(trimmed, "author");
                    } else if trimmed.starts_with("\"license\"") {
                        pkg.license = idx_str_val(trimmed, "license");
                    } else if trimmed.starts_with("\"keywords\"") {
                        pkg.keywords = idx_str_array(trimmed, "keywords");
                    } else if trimmed.starts_with("\"versions\"") {
                        in_versions = true;
                    }
                }
            }
            // End of packages block.
            0 | 2 if trimmed.starts_with('}') => {
                break;
            }
            _ => {}
        }
    }

    if let Some(pkg) = current_pkg {
        packages.insert(pkg.name.clone(), pkg);
    }
    packages
}

// ---------------------------------------------------------------------------

fn chrono_now() -> String {
    // Minimal timestamp without chrono dep
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    format!("{}", secs)
}

fn urldecode(s: &str) -> String {
    s.replace('+', " ")
        .split('%')
        .enumerate()
        .map(|(i, chunk)| {
            if i == 0 { chunk.to_string() }
            else if chunk.len() >= 2 {
                let hex = &chunk[..2];
                let rest = &chunk[2..];
                if let Ok(b) = u8::from_str_radix(hex, 16) {
                    format!("{}{}", b as char, rest)
                } else {
                    format!("%{}", chunk)
                }
            } else {
                format!("%{}", chunk)
            }
        })
        .collect()
}

trait StrExt {
    fn or_else_if_empty(self, default: &str) -> String;
}

impl StrExt for String {
    fn or_else_if_empty(self, default: &str) -> String {
        if self.is_empty() { default.to_string() } else { self }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    let port: u16 = std::env::var("REGISTRY_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8080);

    let data_dir = PathBuf::from(
        std::env::var("REGISTRY_DATA_DIR").unwrap_or_else(|_| "./registry-data".to_string())
    );
    fs::create_dir_all(&data_dir).expect("Failed to create data directory");
    fs::create_dir_all(data_dir.join("tarballs")).expect("Failed to create tarballs directory");

    let api_token = std::env::var("REGISTRY_TOKEN").ok();
    if api_token.is_none() {
        eprintln!("WARNING: REGISTRY_TOKEN not set — publishing is open to anyone");
    }

    let registry = Arc::new(Mutex::new(Registry::new(data_dir, api_token)));

    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).expect("Failed to bind");
    eprintln!("txtcode registry listening on http://{}", addr);

    for stream in listener.incoming().flatten() {
        let reg = Arc::clone(&registry);
        std::thread::spawn(move || handle_request(stream, reg));
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_SEQ: AtomicU64 = AtomicU64::new(0);

    fn test_temp_dir() -> std::path::PathBuf {
        let n = TEST_SEQ.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!("txtcode_reg_{}_{}",
            std::process::id(), n))
    }

    fn make_registry_with_pkg() -> Registry {
        let tmp = test_temp_dir();
        let _ = fs::create_dir_all(&tmp);
        let mut r = Registry {
            packages: HashMap::new(),
            data_dir: tmp.clone(),
            api_token: None,
        };
        r.packages.insert("mypkg".to_string(), PackageEntry {
            name: "mypkg".to_string(),
            description: "A test package".to_string(),
            author: "tester".to_string(),
            license: "MIT".to_string(),
            keywords: vec!["test".to_string(), "demo".to_string()],
            versions: vec![VersionEntry {
                version: "1.0.0".to_string(),
                sha256: "deadbeef".to_string(),
                published_at: "1711490000".to_string(),
                tarball_path: "tarballs/mypkg-1.0.0.tar.gz".to_string(),
            }],
        });
        r
    }

    #[test]
    fn test_load_index_roundtrip() {
        let r = make_registry_with_pkg();
        r.save_index().expect("save_index failed");

        // Reload into a fresh registry
        let mut r2 = Registry {
            packages: HashMap::new(),
            data_dir: r.data_dir.clone(),
            api_token: None,
        };
        r2.load_index().expect("load_index failed");

        assert!(r2.packages.contains_key("mypkg"), "mypkg should be loaded");
        let pkg = &r2.packages["mypkg"];
        assert_eq!(pkg.description, "A test package");
        assert_eq!(pkg.author, "tester");
        assert_eq!(pkg.license, "MIT");
        assert_eq!(pkg.keywords, vec!["test", "demo"]);
        assert_eq!(pkg.versions.len(), 1);
        assert_eq!(pkg.versions[0].version, "1.0.0");
        assert_eq!(pkg.versions[0].sha256, "deadbeef");
    }

    #[test]
    fn test_load_index_multiple_packages() {
        let mut r = make_registry_with_pkg();
        r.packages.insert("otherpkg".to_string(), PackageEntry {
            name: "otherpkg".to_string(),
            description: "Another package".to_string(),
            author: "dev".to_string(),
            license: "Apache-2.0".to_string(),
            keywords: vec![],
            versions: vec![VersionEntry {
                version: "2.1.0".to_string(),
                sha256: "cafebabe".to_string(),
                published_at: "1711500000".to_string(),
                tarball_path: "tarballs/otherpkg-2.1.0.tar.gz".to_string(),
            }],
        });
        r.save_index().expect("save failed");

        let mut r2 = Registry {
            packages: HashMap::new(),
            data_dir: r.data_dir.clone(),
            api_token: None,
        };
        r2.load_index().expect("load failed");
        assert_eq!(r2.packages.len(), 2);
        assert!(r2.packages.contains_key("mypkg"));
        assert!(r2.packages.contains_key("otherpkg"));
        assert_eq!(r2.packages["otherpkg"].license, "Apache-2.0");
        assert_eq!(r2.packages["otherpkg"].versions[0].version, "2.1.0");
    }

    #[test]
    fn test_load_index_empty_file_absent() {
        let tmp = test_temp_dir();
        // Don't create the dir at all — simulates missing data directory
        let mut r = Registry { packages: HashMap::new(), data_dir: tmp, api_token: None };
        // No index.json → load_index should return Ok and leave packages empty
        assert!(r.load_index().is_ok());
        assert!(r.packages.is_empty());
    }
}
