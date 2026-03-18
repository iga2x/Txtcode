use crate::runtime::{RuntimeError, Value};
use std::collections::HashMap;
use std::time::Duration;

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
                            checker.check_permission(&PermissionResource::Network("connect".to_string()), Some(hostname.as_str()))?;
                        }
                        Self::http_get_sync(url)
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
                            checker.check_permission(&PermissionResource::Network("connect".to_string()), Some(hostname.as_str()))?;
                        }
                        let headers = if args.len() == 3 {
                            match &args[2] {
                                Value::Map(h) => Some(h),
                                _ => return Err(RuntimeError::new("http_post headers must be a map".to_string())),
                            }
                        } else { None };
                        Self::http_post_sync(url, body, headers)
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
                            checker.check_permission(&PermissionResource::Network("connect".to_string()), Some(hostname.as_str()))?;
                        }
                        let headers = if args.len() == 3 {
                            match &args[2] { Value::Map(h) => Some(h), _ => return Err(RuntimeError::new("http_put headers must be a map".to_string())) }
                        } else { None };
                        Self::http_put_sync(url, body, headers)
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
                            checker.check_permission(&PermissionResource::Network("connect".to_string()), Some(hostname.as_str()))?;
                        }
                        let headers = if args.len() == 2 {
                            match &args[1] { Value::Map(h) => Some(h), _ => return Err(RuntimeError::new("http_delete headers must be a map".to_string())) }
                        } else { None };
                        Self::http_delete_sync(url, headers)
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
                            checker.check_permission(&PermissionResource::Network("connect".to_string()), Some(hostname.as_str()))?;
                        }
                        let headers = if args.len() == 3 {
                            match &args[2] { Value::Map(h) => Some(h), _ => return Err(RuntimeError::new("http_patch headers must be a map".to_string())) }
                        } else { None };
                        Self::http_patch_sync(url, body, headers)
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
                            checker.check_permission(&PermissionResource::Network("connect".to_string()), Some(hostname.as_str()))?;
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
                            checker.check_permission(&PermissionResource::Network("connect".to_string()), Some(hostname.as_str()))?;
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
                    Value::String(s) => Some(s.clone()),
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
                    checker.check_permission(&PermissionResource::Network("connect".to_string()), Some(hostname.as_str()))?;
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
                            checker.check_permission(&PermissionResource::Network("connect".to_string()), Some(hostname.as_str()))?;
                        }
                        // Synchronous context: no real streaming, returns full body like http_get
                        Self::http_get_sync(url)
                    }
                    _ => Err(RuntimeError::new("http_stream requires a string URL".to_string())),
                }
            }
            "websocket_connect" => {
                Err(RuntimeError::new(
                    "websocket_connect: WebSocket support is not built in by default. \
                     It is planned for v0.5. Track progress at: https://github.com/iga2x/txtcode/issues".to_string()
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
                            checker.check_permission(&PermissionResource::Network("connect".to_string()), Some(host.as_str()))?;
                        }
                        if *port < 1 || *port > 65535 {
                            return Err(RuntimeError::new("Port must be between 1 and 65535".to_string()));
                        }
                        Self::tcp_connect_sync(host, *port as u16)
                    }
                    (Value::String(host), Value::Float(port)) => {
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            checker.check_permission(&PermissionResource::Network("connect".to_string()), Some(host.as_str()))?;
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
                            checker.check_permission(&PermissionResource::Network("connect".to_string()), Some(host.as_str()))?;
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
                            checker.check_permission(&PermissionResource::Network("connect".to_string()), Some(domain.as_str()))?;
                        }
                        Self::resolve_dns_sync(domain)
                    }
                    _ => Err(RuntimeError::new("resolve requires a string domain".to_string())),
                }
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

    fn collect_headers(response_headers: &reqwest::header::HeaderMap) -> HashMap<String, Value> {
        let mut map = HashMap::new();
        for (name, value) in response_headers {
            if let Ok(val_str) = value.to_str() {
                map.insert(
                    name.as_str().to_string(),
                    Value::String(val_str.to_string()),
                );
            }
        }
        map
    }

    pub async fn http_get_async(url: &str) -> Result<Value, RuntimeError> {
        let client = reqwest::Client::builder()
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
        let mut result = HashMap::new();
        result.insert("status".to_string(), Value::Integer(status as i64));
        result.insert("body".to_string(), Value::String(body));
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
        headers: Option<&HashMap<String, Value>>,
    ) -> Result<Value, RuntimeError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| RuntimeError::new(format!("Failed to create HTTP client: {}", e)))?;
        let mut request = client.post(url).body(body.to_string());
        if let Some(headers_map) = headers {
            for (key, value) in headers_map {
                if let Value::String(val_str) = value {
                    request = request.header(key, val_str);
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
        let mut result = HashMap::new();
        result.insert("status".to_string(), Value::Integer(status as i64));
        result.insert("body".to_string(), Value::String(response_body));
        result.insert("headers".to_string(), Value::Map(headers_map));
        Ok(Value::Map(result))
    }

    fn http_post_sync(
        url: &str,
        body: &str,
        headers: Option<&HashMap<String, Value>>,
    ) -> Result<Value, RuntimeError> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| RuntimeError::new(format!("Failed to create async runtime: {}", e)))?;
        rt.block_on(Self::http_post_async(url, body, headers))
    }

    async fn http_put_async(
        url: &str,
        body: &str,
        headers: Option<&HashMap<String, Value>>,
    ) -> Result<Value, RuntimeError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| RuntimeError::new(format!("Failed to create HTTP client: {}", e)))?;
        let mut request = client.put(url).body(body.to_string());
        if let Some(headers_map) = headers {
            for (key, value) in headers_map {
                if let Value::String(val_str) = value {
                    request = request.header(key, val_str);
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
        let mut result = HashMap::new();
        result.insert("status".to_string(), Value::Integer(status as i64));
        result.insert("body".to_string(), Value::String(response_body));
        result.insert("headers".to_string(), Value::Map(headers_map));
        Ok(Value::Map(result))
    }

    fn http_put_sync(
        url: &str,
        body: &str,
        headers: Option<&HashMap<String, Value>>,
    ) -> Result<Value, RuntimeError> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| RuntimeError::new(format!("Failed to create async runtime: {}", e)))?;
        rt.block_on(Self::http_put_async(url, body, headers))
    }

    async fn http_delete_async(
        url: &str,
        headers: Option<&HashMap<String, Value>>,
    ) -> Result<Value, RuntimeError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| RuntimeError::new(format!("Failed to create HTTP client: {}", e)))?;
        let mut request = client.delete(url);
        if let Some(headers_map) = headers {
            for (key, value) in headers_map {
                if let Value::String(val_str) = value {
                    request = request.header(key, val_str);
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
        let mut result = HashMap::new();
        result.insert("status".to_string(), Value::Integer(status as i64));
        result.insert("body".to_string(), Value::String(response_body));
        result.insert("headers".to_string(), Value::Map(headers_map));
        Ok(Value::Map(result))
    }

    fn http_delete_sync(
        url: &str,
        headers: Option<&HashMap<String, Value>>,
    ) -> Result<Value, RuntimeError> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| RuntimeError::new(format!("Failed to create async runtime: {}", e)))?;
        rt.block_on(Self::http_delete_async(url, headers))
    }

    async fn http_patch_async(
        url: &str,
        body: &str,
        headers: Option<&HashMap<String, Value>>,
    ) -> Result<Value, RuntimeError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| RuntimeError::new(format!("Failed to create HTTP client: {}", e)))?;
        let mut request = client.patch(url).body(body.to_string());
        if let Some(headers_map) = headers {
            for (key, value) in headers_map {
                if let Value::String(val_str) = value {
                    request = request.header(key, val_str);
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
        let mut result = HashMap::new();
        result.insert("status".to_string(), Value::Integer(status as i64));
        result.insert("body".to_string(), Value::String(response_body));
        result.insert("headers".to_string(), Value::Map(headers_map));
        Ok(Value::Map(result))
    }

    fn http_patch_sync(
        url: &str,
        body: &str,
        headers: Option<&HashMap<String, Value>>,
    ) -> Result<Value, RuntimeError> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| RuntimeError::new(format!("Failed to create async runtime: {}", e)))?;
        rt.block_on(Self::http_patch_async(url, body, headers))
    }

    async fn http_headers_async(url: &str) -> Result<Value, RuntimeError> {
        let client = reqwest::Client::builder()
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
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .map_err(|e| RuntimeError::new(format!("Failed to create HTTP client: {}", e)))?;
        let response = match method.to_uppercase().as_str() {
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
        let mut result = HashMap::new();
        result.insert("status".to_string(), Value::Integer(status as i64));
        result.insert("body".to_string(), Value::String(response_body));
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
        let mut result = HashMap::new();
        result.insert("host".to_string(), Value::String(host.to_string()));
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
            .map(|addr| Value::String(addr.ip().to_string()))
            .collect();
        Ok(Value::Array(addresses))
    }

    fn resolve_dns_sync(domain: &str) -> Result<Value, RuntimeError> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| RuntimeError::new(format!("Failed to create async runtime: {}", e)))?;
        rt.block_on(Self::resolve_dns_async(domain))
    }
}
