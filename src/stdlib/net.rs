use crate::runtime::{Value, RuntimeError};
use std::collections::HashMap;
use std::time::Duration;

/// Networking library
/// 
/// Implements HTTP and TCP networking functions using async I/O with Tokio.
/// Functions can be used in both sync and async contexts.
pub struct NetLib;

impl NetLib {
    /// Call a networking library function
    /// This is a synchronous interface that uses async internally
    /// permission_checker: optional permission checker for permission enforcement
    pub fn call_function(name: &str, args: &[Value], permission_checker: Option<&dyn crate::stdlib::permission_checker::PermissionChecker>) -> Result<Value, RuntimeError> {
        match name {
            "http_get" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("http_get requires 1 argument (url)".to_string()));
                }
                match &args[0] {
                    Value::String(url) => {
                        // Check permission if checker is available
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            // Extract hostname from URL
                            if let Some(hostname) = Self::extract_hostname(url) {
                                checker.check_permission(
                                    &PermissionResource::Network("connect".to_string()),
                                    Some(hostname.as_str())
                                )?;
                            }
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
                        // Check permission if checker is available
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            if let Some(hostname) = Self::extract_hostname(url) {
                                checker.check_permission(
                                    &PermissionResource::Network("connect".to_string()),
                                    Some(hostname.as_str())
                                )?;
                            }
                        }
                        
                        let headers = if args.len() == 3 {
                            match &args[2] {
                                Value::Map(h) => Some(h),
                                _ => return Err(RuntimeError::new("http_post headers must be a map".to_string())),
                            }
                        } else {
                            None
                        };
                        Self::http_post_sync(url, body, headers)
                    }
                    _ => Err(RuntimeError::new("http_post requires url and body as strings".to_string())),
                }
            }
            "tcp_connect" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new("tcp_connect requires 2 arguments (host, port)".to_string()));
                }
                match (&args[0], &args[1]) {
                    (Value::String(host), Value::Integer(port)) => {
                        // Check permission if checker is available
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            checker.check_permission(
                                &PermissionResource::Network("connect".to_string()),
                                Some(host.as_str())
                            )?;
                        }
                        if *port < 1 || *port > 65535 {
                            return Err(RuntimeError::new("Port must be between 1 and 65535".to_string()));
                        }
                        Self::tcp_connect_sync(host, *port as u16)
                    }
                    (Value::String(host), Value::Float(port)) => {
                        // Check permission if checker is available
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            checker.check_permission(
                                &PermissionResource::Network("connect".to_string()),
                                Some(host.as_str())
                            )?;
                        }
                        let port_int = *port as i64;
                        if port_int < 1 || port_int > 65535 {
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
                        Self::resolve_dns_sync(domain)
                    }
                    _ => Err(RuntimeError::new("resolve requires a string domain".to_string())),
                }
            }
            _ => Err(RuntimeError::new(format!("Unknown networking function: {}", name))),
        }
    }

    /// Extract hostname from URL (simplified - for production, use proper URL parsing)
    fn extract_hostname(url: &str) -> Option<String> {
        // Remove protocol if present
        let url = url.trim_start_matches("http://").trim_start_matches("https://");
        // Take everything before / or ?
        let hostname = url.split(&['/', '?', '#'][..]).next()?;
        // Remove port if present
        let hostname = hostname.split(':').next()?;
        Some(hostname.to_string())
    }

    /// Async HTTP GET - returns a future
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
        let body = response.text()
            .await
            .map_err(|e| RuntimeError::new(format!("Failed to read response body: {}", e)))?;
        
        // Return a map with status and body
        let mut result = HashMap::new();
        result.insert("status".to_string(), Value::Integer(status as i64));
        result.insert("body".to_string(), Value::String(body));
        result.insert("headers".to_string(), Value::Map(HashMap::new())); // Headers not included for simplicity
        
        Ok(Value::Map(result))
    }

    /// Synchronous wrapper for HTTP GET
    fn http_get_sync(url: &str) -> Result<Value, RuntimeError> {
        // Use tokio runtime to execute async code synchronously
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| RuntimeError::new(format!("Failed to create async runtime: {}", e)))?;
        rt.block_on(Self::http_get_async(url))
    }

    /// Async HTTP POST - returns a future
    pub async fn http_post_async(url: &str, body: &str, headers: Option<&HashMap<String, Value>>) -> Result<Value, RuntimeError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| RuntimeError::new(format!("Failed to create HTTP client: {}", e)))?;
        
        let mut request = client.post(url).body(body.to_string());
        
        // Add headers if provided
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
        let response_body = response.text()
            .await
            .map_err(|e| RuntimeError::new(format!("Failed to read response body: {}", e)))?;
        
        // Return a map with status and body
        let mut result = HashMap::new();
        result.insert("status".to_string(), Value::Integer(status as i64));
        result.insert("body".to_string(), Value::String(response_body));
        result.insert("headers".to_string(), Value::Map(HashMap::new()));
        
        Ok(Value::Map(result))
    }

    /// Synchronous wrapper for HTTP POST
    fn http_post_sync(url: &str, body: &str, headers: Option<&HashMap<String, Value>>) -> Result<Value, RuntimeError> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| RuntimeError::new(format!("Failed to create async runtime: {}", e)))?;
        rt.block_on(Self::http_post_async(url, body, headers))
    }

    /// Async TCP connect
    pub async fn tcp_connect_async(host: &str, port: u16) -> Result<Value, RuntimeError> {
        use tokio::net::TcpStream;
        
        let address = format!("{}:{}", host, port);
        let _stream = TcpStream::connect(&address)
            .await
            .map_err(|e| RuntimeError::new(format!("TCP connection failed: {}", e)))?;
        
        // Return connection info
        // Note: The stream is dropped here. For a full implementation, we'd need to store it.
        let mut result = HashMap::new();
        result.insert("host".to_string(), Value::String(host.to_string()));
        result.insert("port".to_string(), Value::Integer(port as i64));
        result.insert("connected".to_string(), Value::Boolean(true));
        
        Ok(Value::Map(result))
    }

    /// Synchronous wrapper for TCP connect
    fn tcp_connect_sync(host: &str, port: u16) -> Result<Value, RuntimeError> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| RuntimeError::new(format!("Failed to create async runtime: {}", e)))?;
        rt.block_on(Self::tcp_connect_async(host, port))
    }

    /// Async UDP send
    pub async fn udp_send_async(host: &str, port: u16, data: &str) -> Result<Value, RuntimeError> {
        use tokio::net::UdpSocket;
        
        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .map_err(|e| RuntimeError::new(format!("Failed to create UDP socket: {}", e)))?;
        
        let address = format!("{}:{}", host, port);
        socket.send_to(data.as_bytes(), &address)
            .await
            .map_err(|e| RuntimeError::new(format!("UDP send failed: {}", e)))?;
        
        Ok(Value::Integer(data.len() as i64)) // Return bytes sent
    }

    /// Synchronous wrapper for UDP send
    fn udp_send_sync(host: &str, port: u16, data: &str) -> Result<Value, RuntimeError> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| RuntimeError::new(format!("Failed to create async runtime: {}", e)))?;
        rt.block_on(Self::udp_send_async(host, port, data))
    }

    /// Async DNS resolution
    pub async fn resolve_dns_async(domain: &str) -> Result<Value, RuntimeError> {
        use tokio::net::lookup_host;
        
        let addresses: Vec<_> = lookup_host((domain, 0))
            .await
            .map_err(|e| RuntimeError::new(format!("DNS resolution failed: {}", e)))?
            .map(|addr| Value::String(addr.ip().to_string()))
            .collect();
        
        Ok(Value::Array(addresses))
    }

    /// Synchronous wrapper for DNS resolution
    fn resolve_dns_sync(domain: &str) -> Result<Value, RuntimeError> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| RuntimeError::new(format!("Failed to create async runtime: {}", e)))?;
        rt.block_on(Self::resolve_dns_async(domain))
    }
}
