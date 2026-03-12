// Capability AST nodes - structured capability expressions for validation and enforcement

use super::common::Span;

/// Capability expression - describes what actions/resources a function can access
#[derive(Debug, Clone, PartialEq)]
pub enum CapabilityExpr {
    /// Simple capability: "fs.read", "net.connect", etc.
    Simple {
        resource: String, // e.g., "fs", "net", "sys"
        action: String,   // e.g., "read", "write", "connect"
        span: Span,
    },
    /// Wildcard resource: "*.read" (any resource, specific action)
    ResourceWildcard {
        action: String, // e.g., "read", "write"
        span: Span,
    },
    /// Wildcard action: "fs.*" (specific resource, any action)
    ActionWildcard {
        resource: String, // e.g., "fs", "net"
        span: Span,
    },
    /// Full wildcard: "*.*" (any resource, any action)
    FullWildcard { span: Span },
    /// Scoped capability: "fs.read:/tmp/*" (resource.action:scope)
    Scoped {
        resource: String,
        action: String,
        scope: String, // e.g., "/tmp/*", "example.com"
        span: Span,
    },
    /// Tool capability: "tool:nmap" (specific tool execution)
    Tool { tool_name: String, span: Span },
    /// All tools: "tool:*" (any tool)
    AllTools { span: Span },
}

impl CapabilityExpr {
    /// Create a simple capability
    pub fn simple(resource: String, action: String, span: Span) -> Self {
        Self::Simple {
            resource,
            action,
            span,
        }
    }

    /// Parse capability from string like "fs.read", "net.*", etc.
    pub fn from_string(s: &str, span: Span) -> Result<Self, String> {
        // Handle tool capabilities
        if let Some(tool_name) = s.strip_prefix("tool:") {
            if tool_name == "*" {
                return Ok(Self::AllTools { span });
            }
            return Ok(Self::Tool {
                tool_name: tool_name.to_string(),
                span,
            });
        }

        // Handle scoped capabilities: "resource.action:scope"
        if let Some(colon_pos) = s.find(':') {
            let (cap_part, scope) = s.split_at(colon_pos);
            let scope = &scope[1..]; // Skip ':'

            // Parse capability part
            if let Some(dot_pos) = cap_part.find('.') {
                let resource = &cap_part[..dot_pos];
                let action = &cap_part[dot_pos + 1..];

                return Ok(Self::Scoped {
                    resource: resource.to_string(),
                    action: action.to_string(),
                    scope: scope.to_string(),
                    span,
                });
            }
            return Err(format!("Invalid scoped capability format: {}", s));
        }

        // Parse regular capabilities: "resource.action", "resource.*", "*.action", "*.*"
        if let Some(dot_pos) = s.find('.') {
            let resource = &s[..dot_pos];
            let action = &s[dot_pos + 1..];

            if resource == "*" && action == "*" {
                return Ok(Self::FullWildcard { span });
            } else if resource == "*" {
                return Ok(Self::ResourceWildcard {
                    action: action.to_string(),
                    span,
                });
            } else if action == "*" {
                return Ok(Self::ActionWildcard {
                    resource: resource.to_string(),
                    span,
                });
            } else {
                return Ok(Self::Simple {
                    resource: resource.to_string(),
                    action: action.to_string(),
                    span,
                });
            }
        }

        Err(format!("Invalid capability format: {}. Expected 'resource.action', 'resource.*', '*.action', or '*.*'", s))
    }

    /// Check if capability matches a given resource and action
    pub fn matches(&self, resource: &str, action: &str) -> bool {
        match self {
            CapabilityExpr::Simple {
                resource: r,
                action: a,
                ..
            } => r == resource && a == action,
            CapabilityExpr::ResourceWildcard { action: a, .. } => a == action,
            CapabilityExpr::ActionWildcard { resource: r, .. } => r == resource,
            CapabilityExpr::FullWildcard { .. } => true,
            CapabilityExpr::Scoped {
                resource: r,
                action: a,
                scope: _scope,
                ..
            } => {
                // For scoped capabilities, we'd need scope matching logic here
                // For now, just check resource and action
                r == resource && a == action
            }
            CapabilityExpr::Tool { .. } | CapabilityExpr::AllTools { .. } => {
                // Tool capabilities are handled separately
                false
            }
        }
    }

    /// Check if capability matches a tool
    pub fn matches_tool(&self, tool_name: &str) -> bool {
        match self {
            CapabilityExpr::Tool { tool_name: t, .. } => t == tool_name,
            CapabilityExpr::AllTools { .. } => true,
            _ => false,
        }
    }
}

impl std::fmt::Display for CapabilityExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CapabilityExpr::Simple {
                resource, action, ..
            } => write!(f, "{}.{}", resource, action),
            CapabilityExpr::ResourceWildcard { action, .. } => write!(f, "*.{}", action),
            CapabilityExpr::ActionWildcard { resource, .. } => write!(f, "{}.*", resource),
            CapabilityExpr::FullWildcard { .. } => write!(f, "*.*"),
            CapabilityExpr::Scoped {
                resource,
                action,
                scope,
                ..
            } => write!(f, "{}.{}:{}", resource, action, scope),
            CapabilityExpr::Tool { tool_name, .. } => write!(f, "tool:{}", tool_name),
            CapabilityExpr::AllTools { .. } => write!(f, "tool:*"),
        }
    }
}

/// Rate limit expression - for capability rate limiting
#[derive(Debug, Clone, PartialEq)]
pub struct RateLimitExpr {
    pub count: u64,          // Number of actions allowed
    pub window_seconds: u64, // Time window in seconds
    pub span: Span,
}

impl RateLimitExpr {
    pub fn new(count: u64, window_seconds: u64, span: Span) -> Self {
        Self {
            count,
            window_seconds,
            span,
        }
    }

    /// Parse rate limit from string like "100/hour", "10/minute"
    pub fn from_string(s: &str, span: Span) -> Result<Self, String> {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 2 {
            return Err(format!(
                "Invalid rate limit format: {}. Expected 'count/period'",
                s
            ));
        }

        let count: u64 = parts[0]
            .parse()
            .map_err(|_| format!("Invalid count in rate limit: {}", parts[0]))?;

        let window_seconds = match parts[1].to_lowercase().as_str() {
            "second" | "sec" | "s" => 1,
            "minute" | "min" | "m" => 60,
            "hour" | "hr" | "h" => 3600,
            "day" | "d" => 86400,
            _ => {
                return Err(format!(
                "Invalid period in rate limit: {}. Expected 'second', 'minute', 'hour', or 'day'",
                parts[1]
            ))
            }
        };

        Ok(Self::new(count, window_seconds, span))
    }
}

/// Duration expression - for timeouts and durations
#[derive(Debug, Clone, PartialEq)]
pub enum DurationExpr {
    Seconds(u64),
    Minutes(u64),
    Hours(u64),
    Days(u64),
}

impl DurationExpr {
    /// Parse duration from string like "30s", "5m", "1h", "2d"
    pub fn from_string(s: &str) -> Result<Self, String> {
        let s = s.trim();
        if s.is_empty() {
            return Err("Empty duration string".to_string());
        }

        let (num_str, unit) = if s.len() > 1 {
            let last_char = s.chars().last().unwrap();
            if last_char.is_alphabetic() {
                (&s[..s.len() - 1], last_char)
            } else {
                return Err(format!(
                    "Invalid duration format: {}. Expected number followed by unit (s, m, h, d)",
                    s
                ));
            }
        } else {
            return Err(format!(
                "Invalid duration format: {}. Expected number followed by unit",
                s
            ));
        };

        let value: u64 = num_str
            .parse()
            .map_err(|_| format!("Invalid number in duration: {}", num_str))?;

        match unit.to_lowercase().next() {
            Some('s') => Ok(Self::Seconds(value)),
            Some('m') => Ok(Self::Minutes(value)),
            Some('h') => Ok(Self::Hours(value)),
            Some('d') => Ok(Self::Days(value)),
            _ => Err(format!(
                "Invalid duration unit: {}. Expected s, m, h, or d",
                unit
            )),
        }
    }

    /// Convert to seconds
    pub fn to_seconds(&self) -> u64 {
        match self {
            DurationExpr::Seconds(s) => *s,
            DurationExpr::Minutes(m) => *m * 60,
            DurationExpr::Hours(h) => *h * 3600,
            DurationExpr::Days(d) => *d * 86400,
        }
    }
}
