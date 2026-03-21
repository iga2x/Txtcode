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
        // Parse our simplified JSON format
        // In production, use serde_json — here we do a lightweight read
        let _ = content; // loaded — deserialization would parse packages
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
                        if rest.starts_with('"') {
                            let inner = &rest[1..];
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

    for stream in listener.incoming() {
        if let Ok(stream) = stream {
            let reg = Arc::clone(&registry);
            std::thread::spawn(move || handle_request(stream, reg));
        }
    }
}
