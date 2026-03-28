use crate::runtime::{RuntimeError, Value};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// ── Task 16.2: WebSocket connection registry ─────────────────────────────────
// Maps integer handle IDs to open WebSocket streams.
// Using MaybeTlsWebSocket to support both ws:// and wss://.
#[cfg(feature = "net")]
type WsStream = tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>;

#[cfg(feature = "net")]
lazy_static::lazy_static! {
    static ref WS_CONNECTIONS: Mutex<HashMap<i64, Arc<Mutex<WsStream>>>> =
        Mutex::new(HashMap::new());
    static ref WS_NEXT_ID: Mutex<i64> = Mutex::new(1);
}

/// Networking library
pub struct NetLib;

impl NetLib {
    /// Call a networking library function.
    ///
    /// `permission_checker`: Must be `Some(checker)` in all VM-dispatched calls.
    /// Pass `None` only in trusted internal Rust contexts (unit tests, tool executors
    /// that perform their own permission checks upstream).
    pub fn call_function(
        name: &str,
        args: &[Value],
        permission_checker: Option<&dyn crate::stdlib::permission_checker::PermissionChecker>,
    ) -> Result<Value, RuntimeError> {
        #[cfg(debug_assertions)]
        if permission_checker.is_none() {
            crate::tools::logger::log_warn(&format!(
                "stdlib internal: '{}' called without permission_checker — trusted path only", name
            ));
        }
        match name {
            "http_get" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("http_get requires 1 argument (url)".to_string()));
                }
                match &args[0] {
                    Value::String(url) => {
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            let hostname = Self::extract_hostname(url).ok_or_else(|| RuntimeError::new(format!("Malformed URL '{}': cannot determine hostname for permission check", url)))?;
                            checker.check_permission(&PermissionResource::Network("connect".to_string()), Some(hostname.as_ref()))?;
                        }
                        Ok(Self::http_get_future(url.to_string()))
                    }
                    _ => Err(RuntimeError::new("http_get requires a string URL".to_string())),
                }
            }
            "http_post" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(RuntimeError::new("http_post requires 2 or 3 arguments (url, body, headers?)".to_string()));
                }
                match (&args[0], &args[1]) {
                    (Value::String(url), Value::String(body)) => {
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            let hostname = Self::extract_hostname(url).ok_or_else(|| RuntimeError::new(format!("Malformed URL '{}': cannot determine hostname for permission check", url)))?;
                            checker.check_permission(&PermissionResource::Network("connect".to_string()), Some(hostname.as_ref()))?;
                        }
                        let headers = if args.len() == 3 {
                            match &args[2] {
                                Value::Map(h) => Some(h.clone()),
                                _ => return Err(RuntimeError::new("http_post headers must be a map".to_string())),
                            }
                        } else { None };
                        Ok(Self::http_post_future(url.to_string(), body.to_string(), headers))
                    }
                    _ => Err(RuntimeError::new("http_post requires url and body as strings".to_string())),
                }
            }
            "http_put" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(RuntimeError::new("http_put requires 2 or 3 arguments (url, body, headers?)".to_string()));
                }
                match (&args[0], &args[1]) {
                    (Value::String(url), Value::String(body)) => {
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            let hostname = Self::extract_hostname(url).ok_or_else(|| RuntimeError::new(format!("Malformed URL '{}': cannot determine hostname for permission check", url)))?;
                            checker.check_permission(&PermissionResource::Network("connect".to_string()), Some(hostname.as_ref()))?;
                        }
                        let headers = if args.len() == 3 {
                            match &args[2] { Value::Map(h) => Some(h.clone()), _ => return Err(RuntimeError::new("http_put headers must be a map".to_string())) }
                        } else { None };
                        Ok(Self::http_put_future(url.to_string(), body.to_string(), headers))
                    }
                    _ => Err(RuntimeError::new("http_put requires url and body as strings".to_string())),
                }
            }
            "http_delete" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(RuntimeError::new("http_delete requires 1 or 2 arguments (url, headers?)".to_string()));
                }
                match &args[0] {
                    Value::String(url) => {
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            let hostname = Self::extract_hostname(url).ok_or_else(|| RuntimeError::new(format!("Malformed URL '{}': cannot determine hostname for permission check", url)))?;
                            checker.check_permission(&PermissionResource::Network("connect".to_string()), Some(hostname.as_ref()))?;
                        }
                        let headers = if args.len() == 2 {
                            match &args[1] { Value::Map(h) => Some(h.clone()), _ => return Err(RuntimeError::new("http_delete headers must be a map".to_string())) }
                        } else { None };
                        Ok(Self::http_delete_future(url.to_string(), headers))
                    }
                    _ => Err(RuntimeError::new("http_delete requires a string URL".to_string())),
                }
            }
            "http_patch" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(RuntimeError::new("http_patch requires 2 or 3 arguments (url, body, headers?)".to_string()));
                }
                match (&args[0], &args[1]) {
                    (Value::String(url), Value::String(body)) => {
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            let hostname = Self::extract_hostname(url).ok_or_else(|| RuntimeError::new(format!("Malformed URL '{}': cannot determine hostname for permission check", url)))?;
                            checker.check_permission(&PermissionResource::Network("connect".to_string()), Some(hostname.as_ref()))?;
                        }
                        let headers = if args.len() == 3 {
                            match &args[2] { Value::Map(h) => Some(h.clone()), _ => return Err(RuntimeError::new("http_patch headers must be a map".to_string())) }
                        } else { None };
                        Ok(Self::http_patch_future(url.to_string(), body.to_string(), headers))
                    }
                    _ => Err(RuntimeError::new("http_patch requires url and body as strings".to_string())),
                }
            }
            "http_headers" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("http_headers requires 1 argument (url)".to_string()));
                }
                match &args[0] {
                    Value::String(url) => {
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            let hostname = Self::extract_hostname(url).ok_or_else(|| RuntimeError::new(format!("Malformed URL '{}': cannot determine hostname for permission check", url)))?;
                            checker.check_permission(&PermissionResource::Network("connect".to_string()), Some(hostname.as_ref()))?;
                        }
                        Self::http_headers_sync(url)
                    }
                    _ => Err(RuntimeError::new("http_headers requires a string URL".to_string())),
                }
            }
            "http_status" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("http_status requires 1 argument (url)".to_string()));
                }
                match &args[0] {
                    Value::String(url) => {
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            let hostname = Self::extract_hostname(url).ok_or_else(|| RuntimeError::new(format!("Malformed URL '{}': cannot determine hostname for permission check", url)))?;
                            checker.check_permission(&PermissionResource::Network("connect".to_string()), Some(hostname.as_ref()))?;
                        }
                        Self::http_status_sync(url)
                    }
                    _ => Err(RuntimeError::new("http_status requires a string URL".to_string())),
                }
            }
            "http_timeout" => {
                if args.len() < 3 || args.len() > 4 {
                    return Err(RuntimeError::new("http_timeout requires 3-4 arguments (url, method, body, timeout_ms?)".to_string()));
                }
                let url = match &args[0] { Value::String(s) => s.clone(), _ => return Err(RuntimeError::new("http_timeout: url must be a string".to_string())) };
                let method = match &args[1] { Value::String(s) => s.clone(), _ => return Err(RuntimeError::new("http_timeout: method must be a string".to_string())) };
                let body: Option<String> = match &args[2] {
                    Value::String(s) => Some(s.to_string()),
                    Value::Null => None,
                    _ => return Err(RuntimeError::new("http_timeout: body must be a string or null".to_string())),
                };
                let timeout_ms: u64 = if args.len() == 4 {
                    match &args[3] {
                        Value::Integer(ms) => *ms as u64,
                        _ => return Err(RuntimeError::new("http_timeout: timeout_ms must be an integer".to_string())),
                    }
                } else { 30000 };
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    let hostname = Self::extract_hostname(&url).ok_or_else(|| RuntimeError::new(format!("Malformed URL '{}': cannot determine hostname for permission check", url)))?;
                    checker.check_permission(&PermissionResource::Network("connect".to_string()), Some(hostname.as_ref()))?;
                }
                Self::http_timeout_sync(&url, &method, body.as_deref(), timeout_ms)
            }
            "http_stream" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("http_stream requires 1 argument (url)".to_string()));
                }
                match &args[0] {
                    Value::String(url) => {
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            let hostname = Self::extract_hostname(url).ok_or_else(|| RuntimeError::new(format!("Malformed URL '{}': cannot determine hostname for permission check", url)))?;
                            checker.check_permission(&PermissionResource::Network("connect".to_string()), Some(hostname.as_ref()))?;
                        }
                        // Synchronous context: no real streaming, returns full body like http_get
                        Self::http_get_sync(url)
                    }
                    _ => Err(RuntimeError::new("http_stream requires a string URL".to_string())),
                }
            }
            // ── HTTP Server helpers ──────────────────────────────────────────
            "http_response" => {
                // http_response(status, body, headers?) → {status, body, headers}
                if args.len() < 2 || args.len() > 3 {
                    return Err(RuntimeError::new("http_response requires 2 or 3 arguments (status, body, headers?)".to_string()));
                }
                let status = match &args[0] {
                    Value::Integer(n) => *n,
                    Value::Float(f) => *f as i64,
                    _ => return Err(RuntimeError::new("http_response: status must be an integer".to_string())),
                };
                let body: std::sync::Arc<str> = match &args[1] {
                    Value::String(s) => s.clone(),
                    other => std::sync::Arc::from(other.to_string()),
                };
                let headers = if args.len() == 3 {
                    match &args[2] {
                        Value::Map(m) => m.clone(),
                        _ => return Err(RuntimeError::new("http_response: headers must be a map".to_string())),
                    }
                } else {
                    IndexMap::new()
                };
                let mut result = IndexMap::new();
                result.insert("status".to_string(), Value::Integer(status));
                result.insert("body".to_string(), Value::String(body));
                result.insert("headers".to_string(), Value::Map(headers));
                Ok(Value::Map(result))
            }
            "http_request_method" => {
                if args.len() != 1 { return Err(RuntimeError::new("http_request_method requires 1 argument (request)".to_string())); }
                match &args[0] {
                    Value::Map(m) => Ok(m.get("method").cloned().unwrap_or(Value::String(std::sync::Arc::from("GET")))),
                    _ => Err(RuntimeError::new("http_request_method: argument must be a request map".to_string())),
                }
            }
            "http_request_path" => {
                if args.len() != 1 { return Err(RuntimeError::new("http_request_path requires 1 argument (request)".to_string())); }
                match &args[0] {
                    Value::Map(m) => Ok(m.get("path").cloned().unwrap_or(Value::String(std::sync::Arc::from("/")))),
                    _ => Err(RuntimeError::new("http_request_path: argument must be a request map".to_string())),
                }
            }
            "http_request_body" => {
                if args.len() != 1 { return Err(RuntimeError::new("http_request_body requires 1 argument (request)".to_string())); }
                match &args[0] {
                    Value::Map(m) => Ok(m.get("body").cloned().unwrap_or(Value::String(std::sync::Arc::from("")))),
                    _ => Err(RuntimeError::new("http_request_body: argument must be a request map".to_string())),
                }
            }
            "http_serve" => {
                // http_serve requires an executor to call the handler; without one it errors.
                Err(RuntimeError::new(
                    "http_serve: handler callback requires VM context. \
                     Call http_serve via the standard VM dispatch path.".to_string()
                ))
            }
            // ── Task 16.2: WebSocket client ──────────────────────────────────
            "ws_connect" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("ws_connect requires 1 argument (url)".to_string()));
                }
                let url = match &args[0] {
                    Value::String(s) => s.to_string(),
                    _ => return Err(RuntimeError::new("ws_connect: url must be a string".to_string())),
                };
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    let host = Self::extract_hostname(&url).ok_or_else(||
                        RuntimeError::new(format!("ws_connect: cannot extract hostname from '{}'", url)))?;
                    checker.check_permission(&PermissionResource::Network("connect".to_string()), Some(host.as_ref()))?;
                }
                Self::ws_connect_impl(&url)
            }
            "ws_send" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new("ws_send requires 2 arguments (id, message)".to_string()));
                }
                let id = match &args[0] {
                    Value::Integer(n) => *n,
                    _ => return Err(RuntimeError::new("ws_send: id must be an integer".to_string())),
                };
                let msg: String = match &args[1] {
                    Value::String(s) => s.to_string(),
                    other => other.to_string(),
                };
                Self::ws_send_impl(id, &msg)
            }
            "ws_recv" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("ws_recv requires 1 argument (id)".to_string()));
                }
                let id = match &args[0] {
                    Value::Integer(n) => *n,
                    _ => return Err(RuntimeError::new("ws_recv: id must be an integer".to_string())),
                };
                Self::ws_recv_impl(id)
            }
            "ws_close" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("ws_close requires 1 argument (id)".to_string()));
                }
                let id = match &args[0] {
                    Value::Integer(n) => *n,
                    _ => return Err(RuntimeError::new("ws_close: id must be an integer".to_string())),
                };
                Self::ws_close_impl(id)
            }
            "websocket_connect" => {
                // Legacy alias for ws_connect — kept for backward compatibility.
                if args.is_empty() {
                    return Err(RuntimeError::new("ws_connect requires 1 argument (url)".to_string()));
                }
                let url = match &args[0] {
                    Value::String(s) => s.to_string(),
                    _ => return Err(RuntimeError::new("ws_connect: url must be a string".to_string())),
                };
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    let host = Self::extract_hostname(&url).ok_or_else(||
                        RuntimeError::new(format!("ws_connect: cannot extract hostname from '{}'", url)))?;
                    checker.check_permission(&PermissionResource::Network("connect".to_string()), Some(host.as_ref()))?;
                }
                Self::ws_connect_impl(&url)
            }
            "ws_serve" => {
                // ws_serve requires an executor to call the handler; routed via serve_ws_with_executor.
                Err(RuntimeError::new(
                    "ws_serve: handler callback requires VM context. \
                     Call ws_serve via the standard VM dispatch path.".to_string()
                ))
            }
            "tcp_connect" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new("tcp_connect requires 2 arguments (host, port)".to_string()));
                }
                match (&args[0], &args[1]) {
                    (Value::String(host), Value::Integer(port)) => {
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            checker.check_permission(&PermissionResource::Network("connect".to_string()), Some(host.as_ref()))?;
                        }
                        if *port < 1 || *port > 65535 {
                            return Err(RuntimeError::new("Port must be between 1 and 65535".to_string()));
                        }
                        Self::tcp_connect_sync(host, *port as u16)
                    }
                    (Value::String(host), Value::Float(port)) => {
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            checker.check_permission(&PermissionResource::Network("connect".to_string()), Some(host.as_ref()))?;
                        }
                        let port_int = *port as i64;
                        if !(1..=65535).contains(&port_int) {
                            return Err(RuntimeError::new("Port must be between 1 and 65535".to_string()));
                        }
                        Self::tcp_connect_sync(host, port_int as u16)
                    }
                    _ => Err(RuntimeError::new("tcp_connect requires string host and integer port".to_string())),
                }
            }
            "udp_send" => {
                if args.len() != 3 {
                    return Err(RuntimeError::new("udp_send requires 3 arguments (host, port, data)".to_string()));
                }
                match (&args[0], &args[1], &args[2]) {
                    (Value::String(host), Value::Integer(port), Value::String(data)) => {
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            checker.check_permission(&PermissionResource::Network("connect".to_string()), Some(host.as_ref()))?;
                        }
                        if *port < 1 || *port > 65535 {
                            return Err(RuntimeError::new("Port must be between 1 and 65535".to_string()));
                        }
                        Self::udp_send_sync(host, *port as u16, data)
                    }
                    _ => Err(RuntimeError::new("udp_send requires string host, integer port, and string data".to_string())),
                }
            }
            "resolve" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("resolve requires 1 argument (domain)".to_string()));
                }
                match &args[0] {
                    Value::String(domain) => {
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            checker.check_permission(&PermissionResource::Network("connect".to_string()), Some(domain.as_ref()))?;
                        }
                        Self::resolve_dns_sync(domain)
                    }
                    _ => Err(RuntimeError::new("resolve requires a string domain".to_string())),
                }
            }
            // ── Task 16.5: DNS and network utilities ─────────────────────────

            // dns_resolve(hostname) → Array<String> of IP addresses
            "dns_resolve" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("dns_resolve requires 1 argument (hostname)".to_string()));
                }
                let host = match &args[0] {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::new("dns_resolve: hostname must be a string".to_string())),
                };
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(&PermissionResource::Network("connect".to_string()), Some(host.as_ref()))?;
                }
                Self::resolve_dns_sync(&host)
            }

            // net_port_open(host, port, timeout_ms) → bool
            "net_port_open" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(RuntimeError::new("net_port_open requires 2-3 arguments (host, port, timeout_ms?)".to_string()));
                }
                let host = match &args[0] {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::new("net_port_open: host must be a string".to_string())),
                };
                let port = match &args[1] {
                    Value::Integer(n) => *n,
                    Value::Float(f) => *f as i64,
                    _ => return Err(RuntimeError::new("net_port_open: port must be an integer".to_string())),
                };
                if !(1..=65535).contains(&port) {
                    return Err(RuntimeError::new("net_port_open: port must be between 1 and 65535".to_string()));
                }
                let timeout_ms: u64 = if args.len() == 3 {
                    match &args[2] {
                        Value::Integer(ms) => *ms as u64,
                        Value::Float(f) => *f as u64,
                        _ => 3000,
                    }
                } else { 3000 };
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(&PermissionResource::Network("connect".to_string()), Some(host.as_ref()))?;
                }
                Ok(Value::Boolean(Self::tcp_probe(&host, port as u16, timeout_ms)))
            }

            // net_ping(host, timeout_ms) → bool
            // Implemented as TCP probe to port 80 (ICMP requires root; TCP probe is portable).
            "net_ping" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(RuntimeError::new("net_ping requires 1-2 arguments (host, timeout_ms?)".to_string()));
                }
                let host = match &args[0] {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::new("net_ping: host must be a string".to_string())),
                };
                let timeout_ms: u64 = if args.len() == 2 {
                    match &args[1] {
                        Value::Integer(ms) => *ms as u64,
                        Value::Float(f) => *f as u64,
                        _ => 3000,
                    }
                } else { 3000 };
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(&PermissionResource::Network("connect".to_string()), Some(host.as_ref()))?;
                }
                // Probe ports 80 and 443; reachable if either succeeds.
                let reachable = Self::tcp_probe(&host, 80, timeout_ms)
                    || Self::tcp_probe(&host, 443, timeout_ms);
                Ok(Value::Boolean(reachable))
            }

            // ── Task 16.1: Raw TLS socket connection ─────────────────────────
            "tls_connect" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new("tls_connect requires 2 arguments (host, port)".to_string()));
                }
                let host = match &args[0] {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::new("tls_connect: host must be a string".to_string())),
                };
                let port = match &args[1] {
                    Value::Integer(n) => *n,
                    Value::Float(f) => *f as i64,
                    _ => return Err(RuntimeError::new("tls_connect: port must be an integer".to_string())),
                };
                if !(1..=65535).contains(&port) {
                    return Err(RuntimeError::new("tls_connect: port must be between 1 and 65535".to_string()));
                }
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(&PermissionResource::Network("connect".to_string()), Some(host.as_ref()))?;
                }
                Self::tls_connect_sync(&host, port as u16)
            }
            _ => Err(RuntimeError::new(format!("Unknown networking function: {}", name))),
        }
    }

    fn extract_hostname(url: &str) -> Option<String> {
        let url = url
            .trim_start_matches("http://")
            .trim_start_matches("https://");
        let hostname = url.split(&['/', '?', '#'][..]).next()?;
        let hostname = hostname.split(':').next()?;
        Some(hostname.to_string())
    }

    fn collect_headers(response_headers: &reqwest::header::HeaderMap) -> IndexMap<String, Value> {
        let mut map = IndexMap::new();
        for (name, value) in response_headers {
            if let Ok(val_str) = value.to_str() {
                map.insert(
                    name.as_str().to_string(),
                    Value::String(Arc::from(val_str.to_string())),
                );
            }
        }
        map
    }

    pub async fn http_get_async(url: &str) -> Result<Value, RuntimeError> {
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| RuntimeError::new(format!("Failed to create HTTP client: {}", e)))?;
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| RuntimeError::new(format!("HTTP GET failed: {}", e)))?;
        let status = response.status().as_u16();
        let headers_map = Self::collect_headers(response.headers());
        let body = response
            .text()
            .await
            .map_err(|e| RuntimeError::new(format!("Failed to read response body: {}", e)))?;
        let mut result = IndexMap::new();
        result.insert("status".to_string(), Value::Integer(status as i64));
        result.insert("body".to_string(), Value::String(Arc::from(body)));
        result.insert("headers".to_string(), Value::Map(headers_map));
        Ok(Value::Map(result))
    }

    fn http_get_sync(url: &str) -> Result<Value, RuntimeError> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| RuntimeError::new(format!("Failed to create async runtime: {}", e)))?;
        rt.block_on(Self::http_get_async(url))
    }

    pub async fn http_post_async(
        url: &str,
        body: &str,
        headers: Option<&IndexMap<String, Value>>,
    ) -> Result<Value, RuntimeError> {
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| RuntimeError::new(format!("Failed to create HTTP client: {}", e)))?;
        let mut request = client.post(url).body(body.to_string());
        if let Some(headers_map) = headers {
            for (key, value) in headers_map {
                if let Value::String(val_str) = value {
                    request = request.header(key.as_str(), val_str.as_ref());
                }
            }
        }
        let response = request
            .send()
            .await
            .map_err(|e| RuntimeError::new(format!("HTTP POST failed: {}", e)))?;
        let status = response.status().as_u16();
        let headers_map = Self::collect_headers(response.headers());
        let response_body = response
            .text()
            .await
            .map_err(|e| RuntimeError::new(format!("Failed to read response body: {}", e)))?;
        let mut result = IndexMap::new();
        result.insert("status".to_string(), Value::Integer(status as i64));
        result.insert("body".to_string(), Value::String(Arc::from(response_body)));
        result.insert("headers".to_string(), Value::Map(headers_map));
        Ok(Value::Map(result))
    }

    fn http_post_sync(
        url: &str,
        body: &str,
        headers: Option<&IndexMap<String, Value>>,
    ) -> Result<Value, RuntimeError> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| RuntimeError::new(format!("Failed to create async runtime: {}", e)))?;
        rt.block_on(Self::http_post_async(url, body, headers))
    }

    async fn http_put_async(
        url: &str,
        body: &str,
        headers: Option<&IndexMap<String, Value>>,
    ) -> Result<Value, RuntimeError> {
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| RuntimeError::new(format!("Failed to create HTTP client: {}", e)))?;
        let mut request = client.put(url).body(body.to_string());
        if let Some(headers_map) = headers {
            for (key, value) in headers_map {
                if let Value::String(val_str) = value {
                    request = request.header(key.as_str(), val_str.as_ref());
                }
            }
        }
        let response = request
            .send()
            .await
            .map_err(|e| RuntimeError::new(format!("HTTP PUT failed: {}", e)))?;
        let status = response.status().as_u16();
        let headers_map = Self::collect_headers(response.headers());
        let response_body = response
            .text()
            .await
            .map_err(|e| RuntimeError::new(format!("Failed to read response body: {}", e)))?;
        let mut result = IndexMap::new();
        result.insert("status".to_string(), Value::Integer(status as i64));
        result.insert("body".to_string(), Value::String(Arc::from(response_body)));
        result.insert("headers".to_string(), Value::Map(headers_map));
        Ok(Value::Map(result))
    }

    fn http_put_sync(
        url: &str,
        body: &str,
        headers: Option<&IndexMap<String, Value>>,
    ) -> Result<Value, RuntimeError> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| RuntimeError::new(format!("Failed to create async runtime: {}", e)))?;
        rt.block_on(Self::http_put_async(url, body, headers))
    }

    async fn http_delete_async(
        url: &str,
        headers: Option<&IndexMap<String, Value>>,
    ) -> Result<Value, RuntimeError> {
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| RuntimeError::new(format!("Failed to create HTTP client: {}", e)))?;
        let mut request = client.delete(url);
        if let Some(headers_map) = headers {
            for (key, value) in headers_map {
                if let Value::String(val_str) = value {
                    request = request.header(key.as_str(), val_str.as_ref());
                }
            }
        }
        let response = request
            .send()
            .await
            .map_err(|e| RuntimeError::new(format!("HTTP DELETE failed: {}", e)))?;
        let status = response.status().as_u16();
        let headers_map = Self::collect_headers(response.headers());
        let response_body = response
            .text()
            .await
            .map_err(|e| RuntimeError::new(format!("Failed to read response body: {}", e)))?;
        let mut result = IndexMap::new();
        result.insert("status".to_string(), Value::Integer(status as i64));
        result.insert("body".to_string(), Value::String(Arc::from(response_body)));
        result.insert("headers".to_string(), Value::Map(headers_map));
        Ok(Value::Map(result))
    }

    fn http_delete_sync(
        url: &str,
        headers: Option<&IndexMap<String, Value>>,
    ) -> Result<Value, RuntimeError> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| RuntimeError::new(format!("Failed to create async runtime: {}", e)))?;
        rt.block_on(Self::http_delete_async(url, headers))
    }

    async fn http_patch_async(
        url: &str,
        body: &str,
        headers: Option<&IndexMap<String, Value>>,
    ) -> Result<Value, RuntimeError> {
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| RuntimeError::new(format!("Failed to create HTTP client: {}", e)))?;
        let mut request = client.patch(url).body(body.to_string());
        if let Some(headers_map) = headers {
            for (key, value) in headers_map {
                if let Value::String(val_str) = value {
                    request = request.header(key.as_str(), val_str.as_ref());
                }
            }
        }
        let response = request
            .send()
            .await
            .map_err(|e| RuntimeError::new(format!("HTTP PATCH failed: {}", e)))?;
        let status = response.status().as_u16();
        let headers_map = Self::collect_headers(response.headers());
        let response_body = response
            .text()
            .await
            .map_err(|e| RuntimeError::new(format!("Failed to read response body: {}", e)))?;
        let mut result = IndexMap::new();
        result.insert("status".to_string(), Value::Integer(status as i64));
        result.insert("body".to_string(), Value::String(Arc::from(response_body)));
        result.insert("headers".to_string(), Value::Map(headers_map));
        Ok(Value::Map(result))
    }

    fn http_patch_sync(
        url: &str,
        body: &str,
        headers: Option<&IndexMap<String, Value>>,
    ) -> Result<Value, RuntimeError> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| RuntimeError::new(format!("Failed to create async runtime: {}", e)))?;
        rt.block_on(Self::http_patch_async(url, body, headers))
    }

    // ── Non-blocking Future helpers ─────────────────────────────────────────

    fn http_get_future(url: String) -> Value {
        let (handle, sender) = crate::runtime::core::value::FutureHandle::pending();
        std::thread::spawn(move || {
            let result = Self::http_get_sync(&url);
            sender.send(result.map_err(|e| e.to_string()));
        });
        Value::Future(handle)
    }

    fn http_post_future(url: String, body: String, headers: Option<IndexMap<String, Value>>) -> Value {
        let (handle, sender) = crate::runtime::core::value::FutureHandle::pending();
        std::thread::spawn(move || {
            let result = Self::http_post_sync(&url, &body, headers.as_ref());
            sender.send(result.map_err(|e| e.to_string()));
        });
        Value::Future(handle)
    }

    fn http_put_future(url: String, body: String, headers: Option<IndexMap<String, Value>>) -> Value {
        let (handle, sender) = crate::runtime::core::value::FutureHandle::pending();
        std::thread::spawn(move || {
            let result = Self::http_put_sync(&url, &body, headers.as_ref());
            sender.send(result.map_err(|e| e.to_string()));
        });
        Value::Future(handle)
    }

    fn http_delete_future(url: String, headers: Option<IndexMap<String, Value>>) -> Value {
        let (handle, sender) = crate::runtime::core::value::FutureHandle::pending();
        std::thread::spawn(move || {
            let result = Self::http_delete_sync(&url, headers.as_ref());
            sender.send(result.map_err(|e| e.to_string()));
        });
        Value::Future(handle)
    }

    fn http_patch_future(url: String, body: String, headers: Option<IndexMap<String, Value>>) -> Value {
        let (handle, sender) = crate::runtime::core::value::FutureHandle::pending();
        std::thread::spawn(move || {
            let result = Self::http_patch_sync(&url, &body, headers.as_ref());
            sender.send(result.map_err(|e| e.to_string()));
        });
        Value::Future(handle)
    }

    async fn http_headers_async(url: &str) -> Result<Value, RuntimeError> {
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| RuntimeError::new(format!("Failed to create HTTP client: {}", e)))?;
        let response = client
            .head(url)
            .send()
            .await
            .map_err(|e| RuntimeError::new(format!("HTTP HEAD failed: {}", e)))?;
        Ok(Value::Map(Self::collect_headers(response.headers())))
    }

    fn http_headers_sync(url: &str) -> Result<Value, RuntimeError> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| RuntimeError::new(format!("Failed to create async runtime: {}", e)))?;
        rt.block_on(Self::http_headers_async(url))
    }

    async fn http_status_async(url: &str) -> Result<Value, RuntimeError> {
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| RuntimeError::new(format!("Failed to create HTTP client: {}", e)))?;
        let response = client
            .head(url)
            .send()
            .await
            .map_err(|e| RuntimeError::new(format!("HTTP HEAD failed: {}", e)))?;
        Ok(Value::Integer(response.status().as_u16() as i64))
    }

    fn http_status_sync(url: &str) -> Result<Value, RuntimeError> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| RuntimeError::new(format!("Failed to create async runtime: {}", e)))?;
        rt.block_on(Self::http_status_async(url))
    }

    async fn http_timeout_async(
        url: &str,
        method: &str,
        body: Option<&str>,
        timeout_ms: u64,
    ) -> Result<Value, RuntimeError> {
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .map_err(|e| RuntimeError::new(format!("Failed to create HTTP client: {}", e)))?;
        let response = match method.to_uppercase().as_ref() {
            "GET" => client.get(url).send().await,
            "POST" => {
                let rb = client.post(url);
                if let Some(b) = body {
                    rb.body(b.to_string()).send().await
                } else {
                    rb.send().await
                }
            }
            "PUT" => {
                let rb = client.put(url);
                if let Some(b) = body {
                    rb.body(b.to_string()).send().await
                } else {
                    rb.send().await
                }
            }
            "DELETE" => client.delete(url).send().await,
            "PATCH" => {
                let rb = client.patch(url);
                if let Some(b) = body {
                    rb.body(b.to_string()).send().await
                } else {
                    rb.send().await
                }
            }
            "HEAD" => client.head(url).send().await,
            other => {
                return Err(RuntimeError::new(format!(
                    "http_timeout: unsupported method '{}'",
                    other
                )))
            }
        }
        .map_err(|e| RuntimeError::new(format!("HTTP {} failed: {}", method, e)))?;
        let status = response.status().as_u16();
        let headers_map = Self::collect_headers(response.headers());
        let response_body = response
            .text()
            .await
            .map_err(|e| RuntimeError::new(format!("Failed to read response body: {}", e)))?;
        let mut result = IndexMap::new();
        result.insert("status".to_string(), Value::Integer(status as i64));
        result.insert("body".to_string(), Value::String(Arc::from(response_body)));
        result.insert("headers".to_string(), Value::Map(headers_map));
        Ok(Value::Map(result))
    }

    fn http_timeout_sync(
        url: &str,
        method: &str,
        body: Option<&str>,
        timeout_ms: u64,
    ) -> Result<Value, RuntimeError> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| RuntimeError::new(format!("Failed to create async runtime: {}", e)))?;
        rt.block_on(Self::http_timeout_async(url, method, body, timeout_ms))
    }

    pub async fn tcp_connect_async(host: &str, port: u16) -> Result<Value, RuntimeError> {
        use tokio::net::TcpStream;
        let address = format!("{}:{}", host, port);
        let _stream = TcpStream::connect(&address)
            .await
            .map_err(|e| RuntimeError::new(format!("TCP connection failed: {}", e)))?;
        let mut result = IndexMap::new();
        result.insert("host".to_string(), Value::String(Arc::from(host.to_string())));
        result.insert("port".to_string(), Value::Integer(port as i64));
        result.insert("connected".to_string(), Value::Boolean(true));
        Ok(Value::Map(result))
    }

    fn tcp_connect_sync(host: &str, port: u16) -> Result<Value, RuntimeError> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| RuntimeError::new(format!("Failed to create async runtime: {}", e)))?;
        rt.block_on(Self::tcp_connect_async(host, port))
    }

    pub async fn udp_send_async(host: &str, port: u16, data: &str) -> Result<Value, RuntimeError> {
        use tokio::net::UdpSocket;
        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .map_err(|e| RuntimeError::new(format!("Failed to create UDP socket: {}", e)))?;
        let address = format!("{}:{}", host, port);
        socket
            .send_to(data.as_bytes(), &address)
            .await
            .map_err(|e| RuntimeError::new(format!("UDP send failed: {}", e)))?;
        Ok(Value::Integer(data.len() as i64))
    }

    fn udp_send_sync(host: &str, port: u16, data: &str) -> Result<Value, RuntimeError> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| RuntimeError::new(format!("Failed to create async runtime: {}", e)))?;
        rt.block_on(Self::udp_send_async(host, port, data))
    }

    pub async fn resolve_dns_async(domain: &str) -> Result<Value, RuntimeError> {
        use tokio::net::lookup_host;
        let addresses: Vec<_> = lookup_host((domain, 0))
            .await
            .map_err(|e| RuntimeError::new(format!("DNS resolution failed: {}", e)))?
            .map(|addr| Value::String(Arc::from(addr.ip().to_string())))
            .collect();
        Ok(Value::Array(addresses))
    }

    fn resolve_dns_sync(domain: &str) -> Result<Value, RuntimeError> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| RuntimeError::new(format!("Failed to create async runtime: {}", e)))?;
        rt.block_on(Self::resolve_dns_async(domain))
    }

    // ── Task 16.1: Raw TLS connection ────────────────────────────────────────

    /// Establish a TLS connection to `host:port` and return a connection-info map.
    /// Uses the platform native TLS stack via the `native-tls` crate.
    fn tls_connect_sync(host: &str, port: u16) -> Result<Value, RuntimeError> {
        use native_tls::TlsConnector;
        use std::net::TcpStream;

        let connector = TlsConnector::new()
            .map_err(|e| RuntimeError::new(format!("tls_connect: failed to create TLS context: {}", e)))?;

        let addr = format!("{}:{}", host, port);
        let tcp = TcpStream::connect(&addr)
            .map_err(|e| RuntimeError::new(format!("tls_connect: TCP connection failed: {}", e)))?;

        // Perform TLS handshake; validates the server certificate.
        let _tls_stream = connector
            .connect(host, tcp)
            .map_err(|e| RuntimeError::new(format!("tls_connect: TLS handshake failed: {}", e)))?;

        let mut result = IndexMap::new();
        result.insert("host".to_string(), Value::String(Arc::from(host.to_string())));
        result.insert("port".to_string(), Value::Integer(port as i64));
        result.insert("connected".to_string(), Value::Boolean(true));
        result.insert("tls".to_string(), Value::Boolean(true));
        Ok(Value::Map(result))
    }

    // ── Task 16.5: TCP reachability probe ────────────────────────────────────

    /// Attempt a TCP connect to `host:port` within `timeout_ms` milliseconds.
    /// Returns `true` if the port is open (connection succeeded), `false` otherwise.
    fn tcp_probe(host: &str, port: u16, timeout_ms: u64) -> bool {
        use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
        let addr_str = format!("{}:{}", host, port);
        let timeout = std::time::Duration::from_millis(timeout_ms);
        let addrs: Vec<SocketAddr> = match addr_str.to_socket_addrs() {
            Ok(a) => a.collect(),
            Err(_) => return false,
        };
        for addr in addrs {
            if TcpStream::connect_timeout(&addr, timeout).is_ok() {
                return true;
            }
        }
        false
    }

    // ── Task 16.2: WebSocket implementation ─────────────────────────────────

    #[cfg(feature = "net")]
    fn ws_connect_impl(url: &str) -> Result<Value, RuntimeError> {
        use tungstenite::connect;
        let (ws, _response) = connect(url)
            .map_err(|e| RuntimeError::new(format!("ws_connect: failed to connect to '{}': {}", url, e)))?;
        let id = {
            let mut next = WS_NEXT_ID.lock().unwrap();
            let id = *next;
            *next += 1;
            id
        };
        WS_CONNECTIONS.lock().unwrap().insert(id, Arc::new(Mutex::new(ws)));
        Ok(Value::Integer(id))
    }

    #[cfg(not(feature = "net"))]
    fn ws_connect_impl(_url: &str) -> Result<Value, RuntimeError> {
        Err(RuntimeError::new("ws_connect requires the 'net' feature.".to_string()))
    }

    #[cfg(feature = "net")]
    fn ws_send_impl(id: i64, message: &str) -> Result<Value, RuntimeError> {
        use tungstenite::Message;
        let conns = WS_CONNECTIONS.lock().unwrap();
        let ws = conns.get(&id).ok_or_else(|| RuntimeError::new(format!("ws_send: no open WebSocket with id {}", id)))?;
        ws.lock().unwrap()
            .send(Message::Text(message.to_string().into()))
            .map_err(|e| RuntimeError::new(format!("ws_send: failed to send: {}", e)))?;
        Ok(Value::Null)
    }

    #[cfg(not(feature = "net"))]
    fn ws_send_impl(_id: i64, _message: &str) -> Result<Value, RuntimeError> {
        Err(RuntimeError::new("ws_send requires the 'net' feature.".to_string()))
    }

    #[cfg(feature = "net")]
    fn ws_recv_impl(id: i64) -> Result<Value, RuntimeError> {
        use tungstenite::Message;
        let conns = WS_CONNECTIONS.lock().unwrap();
        let ws = conns.get(&id).ok_or_else(|| RuntimeError::new(format!("ws_recv: no open WebSocket with id {}", id)))?;
        let msg = ws.lock().unwrap()
            .read()
            .map_err(|e| RuntimeError::new(format!("ws_recv: failed to receive: {}", e)))?;
        match msg {
            Message::Text(t) => Ok(Value::String(Arc::from(t.to_string()))),
            Message::Binary(b) => Ok(Value::String(Arc::from(String::from_utf8_lossy(&b).to_string()))),
            Message::Close(_) => Ok(Value::Null),
            Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => Ok(Value::String(std::sync::Arc::from(""))),
        }
    }

    #[cfg(not(feature = "net"))]
    fn ws_recv_impl(_id: i64) -> Result<Value, RuntimeError> {
        Err(RuntimeError::new("ws_recv requires the 'net' feature.".to_string()))
    }

    #[cfg(feature = "net")]
    fn ws_close_impl(id: i64) -> Result<Value, RuntimeError> {
        use tungstenite::protocol::CloseFrame;
        use tungstenite::protocol::frame::coding::CloseCode;
        if let Some(ws) = WS_CONNECTIONS.lock().unwrap().remove(&id) {
            let _ = ws.lock().unwrap().close(Some(CloseFrame {
                code: CloseCode::Normal,
                reason: "closed by client".into(),
            }));
        }
        Ok(Value::Null)
    }

    #[cfg(not(feature = "net"))]
    fn ws_close_impl(_id: i64) -> Result<Value, RuntimeError> {
        Err(RuntimeError::new("ws_close requires the 'net' feature.".to_string()))
    }

    /// WebSocket server: accepts connections on `port`, calls `handler_fn` for each message.
    ///
    /// Handler receives `{id, message}` and its return value is sent back as the reply.
    pub fn serve_ws_with_executor<E: crate::stdlib::function_executor::FunctionExecutor>(
        args: &[Value],
        executor: &mut E,
        permission_checker: Option<&dyn crate::stdlib::permission_checker::PermissionChecker>,
    ) -> Result<Value, RuntimeError> {
        #[cfg(feature = "net")]
        {
            use tungstenite::{accept, Message};

            if args.len() != 2 {
                return Err(RuntimeError::new("ws_serve requires 2 arguments (port, handler)".to_string()));
            }
            let port = match &args[0] {
                Value::Integer(p) => *p as u16,
                Value::Float(f) => *f as u16,
                _ => return Err(RuntimeError::new("ws_serve: port must be an integer".to_string())),
            };
            let handler_fn = args[1].clone();

            if let Some(checker) = permission_checker {
                checker.check_permission(
                    &crate::runtime::permissions::PermissionResource::Network("bind".to_string()),
                    Some(&format!("0.0.0.0:{}", port)),
                )?;
            }

            let listener = std::net::TcpListener::bind(format!("0.0.0.0:{}", port))
                .map_err(|e| RuntimeError::new(format!("ws_serve: cannot bind to port {}: {}", port, e)))?;

            let mut conn_id: i64 = 1;
            for stream_result in listener.incoming() {
                let stream = match stream_result {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let mut ws = match accept(stream) {
                    Ok(w) => w,
                    Err(_) => continue,
                };
                loop {
                    let msg = match ws.read() {
                        Ok(m) => m,
                        Err(_) => break,
                    };
                    let text = match &msg {
                        Message::Text(t) => t.to_string(),
                        Message::Close(_) => break,
                        _ => continue,
                    };
                    let mut req = IndexMap::new();
                    req.insert("id".to_string(), Value::Integer(conn_id));
                    req.insert("message".to_string(), Value::String(Arc::from(text)));
                    let reply = executor.call_function_value(&handler_fn, &[Value::Map(req)])?;
                    let reply_str: String = match reply {
                        Value::String(s) => s.to_string(),
                        Value::Null => break,
                        other => other.to_string(),
                    };
                    if ws.send(Message::Text(reply_str.into())).is_err() {
                        break;
                    }
                }
                conn_id += 1;
            }
            Ok(Value::Null)
        }
        #[cfg(not(feature = "net"))]
        {
            let _ = (args, executor, permission_checker);
            Err(RuntimeError::new("ws_serve requires the 'net' feature.".to_string()))
        }
    }

    /// Start a blocking HTTP/1.1 server on `port`.
    ///
    /// For each request, calls `handler_fn` via `executor` with a request map:
    /// `{method, path, body, headers}`.
    /// The handler should return a map `{status, body, headers?}` or a string.
    pub fn serve_with_executor<E: crate::stdlib::function_executor::FunctionExecutor>(
        args: &[Value],
        executor: &mut E,
        permission_checker: Option<&dyn crate::stdlib::permission_checker::PermissionChecker>,
    ) -> Result<Value, RuntimeError> {
        if args.len() != 2 {
            return Err(RuntimeError::new("http_serve requires 2 arguments (port, handler)".to_string()));
        }
        let port = match &args[0] {
            Value::Integer(p) => *p as u16,
            Value::Float(f) => *f as u16,
            _ => return Err(RuntimeError::new("http_serve: port must be an integer".to_string())),
        };
        let handler_fn = args[1].clone();

        if let Some(checker) = permission_checker {
            checker.check_permission(
                &crate::runtime::permissions::PermissionResource::Network("listen".to_string()),
                Some(&format!("0.0.0.0:{}", port)),
            )?;
        }

        let listener = std::net::TcpListener::bind(format!("0.0.0.0:{}", port))
            .map_err(|e| RuntimeError::new(format!("http_serve: cannot bind to port {}: {}", port, e)))?;

        for stream_result in listener.incoming() {
            let mut stream = match stream_result {
                Ok(s) => s,
                Err(_) => continue,
            };
            // Parse HTTP/1.1 request
            let request_map = match Self::parse_http_request(&mut stream) {
                Ok(req) => req,
                Err(_) => continue,
            };
            // Call the handler; on error return HTTP 500 instead of propagating.
            let response_val = match executor.call_function_value(&handler_fn, &[Value::Map(request_map)]) {
                Ok(v) => v,
                Err(e) => {
                    let mut m = IndexMap::new();
                    m.insert("status".to_string(), Value::Integer(500));
                    m.insert("body".to_string(), Value::String(Arc::from(format!("Internal Server Error: {}", e))));
                    Value::Map(m)
                }
            };
            // Write the HTTP response
            let _ = Self::write_http_response(&mut stream, response_val);
        }
        Ok(Value::Null)
    }

    pub fn parse_http_request(stream: &mut std::net::TcpStream) -> Result<IndexMap<String, Value>, RuntimeError> {
        use std::io::{BufRead, BufReader, Read};
        let mut reader = BufReader::new(stream.try_clone().map_err(|e| RuntimeError::new(e.to_string()))?);

        // Read request line
        let mut request_line = String::new();
        reader.read_line(&mut request_line).map_err(|e| RuntimeError::new(e.to_string()))?;
        let parts: Vec<&str> = request_line.trim().splitn(3, ' ').collect();
        let method = parts.first().copied().unwrap_or("GET").to_string();
        let path = parts.get(1).copied().unwrap_or("/").to_string();

        // Read headers
        let mut headers: IndexMap<String, Value> = IndexMap::new();
        let mut content_length: usize = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).map_err(|e| RuntimeError::new(e.to_string()))?;
            let trimmed = line.trim();
            if trimmed.is_empty() { break; }
            if let Some(colon_pos) = trimmed.find(':') {
                let key = trimmed[..colon_pos].trim().to_lowercase();
                let val = trimmed[colon_pos + 1..].trim().to_string();
                if key == "content-length" {
                    content_length = val.parse().unwrap_or(0);
                }
                headers.insert(key, Value::String(Arc::from(val)));
            }
        }

        // Read body
        let body = if content_length > 0 {
            let mut buf = vec![0u8; content_length];
            reader.read_exact(&mut buf).map_err(|e| RuntimeError::new(e.to_string()))?;
            String::from_utf8_lossy(&buf).to_string()
        } else {
            String::new()
        };

        let mut req = IndexMap::new();
        req.insert("method".to_string(), Value::String(Arc::from(method)));
        req.insert("path".to_string(), Value::String(Arc::from(path)));
        req.insert("body".to_string(), Value::String(Arc::from(body)));
        req.insert("headers".to_string(), Value::Map(headers));
        Ok(req)
    }

    pub fn write_http_response(stream: &mut std::net::TcpStream, response: Value) -> std::io::Result<()> {
        use std::io::Write;
        let (status, body, extra_headers) = match response {
            Value::Map(ref m) => {
                let status = m.get("status").and_then(|v| if let Value::Integer(n) = v { Some(*n as u16) } else { None }).unwrap_or(200);
                let body: String = m.get("body").map(|v| match v { Value::String(s) => s.to_string(), other => other.to_string() }).unwrap_or_default();
                let headers: Vec<String> = m.get("headers").and_then(|v| if let Value::Map(h) = v { Some(h) } else { None })
                    .map(|h| h.iter().map(|(k, v)| format!("{}: {}", k, v)).collect())
                    .unwrap_or_default();
                (status, body, headers)
            }
            Value::String(s) => (200, s.to_string(), vec![]),
            _ => (200, response.to_string(), vec![]),
        };
        let status_text = match status {
            200 => "OK", 201 => "Created", 204 => "No Content",
            400 => "Bad Request", 401 => "Unauthorized", 403 => "Forbidden",
            404 => "Not Found", 500 => "Internal Server Error",
            _ => "OK",
        };
        write!(stream, "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nContent-Type: text/plain\r\nConnection: close\r\n",
            status, status_text, body.len())?;
        for h in &extra_headers {
            write!(stream, "{}\r\n", h)?;
        }
        write!(stream, "\r\n{}", body)
    }
}
