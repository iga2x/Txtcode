use crate::runtime::{RuntimeError, Value};
use std::collections::HashMap;

/// JSON library for parsing and encoding JSON
pub struct JsonLib;

impl JsonLib {
    /// Call a JSON library function
    pub fn call_function(name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
        match name {
            "json_encode" | "json_stringify" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "json_encode requires 1 argument".to_string(),
                    ));
                }
                Self::json_encode(&args[0])
            }
            "json_decode" | "json_parse" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "json_decode requires 1 argument".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(json_str) => Self::json_decode(json_str),
                    _ => Err(RuntimeError::new(
                        "json_decode requires a string argument".to_string(),
                    )),
                }
            }
            _ => Err(RuntimeError::new(format!(
                "Unknown JSON function: {}",
                name
            ))),
        }
    }

    /// Encode a Txtcode value to JSON string
    fn json_encode(value: &Value) -> Result<Value, RuntimeError> {
        let json_str = match value {
            Value::Null => "null".to_string(),
            Value::Boolean(b) => b.to_string(),
            Value::Integer(i) => i.to_string(),
            Value::Float(f) => {
                if f.is_finite() {
                    f.to_string()
                } else if f.is_nan() || f.is_infinite() {
                    "null".to_string()
                } else {
                    f.to_string()
                }
            }
            Value::String(s) => {
                // Escape string for JSON
                format!("\"{}\"", Self::escape_json_string(s))
            }
            Value::Array(arr) => {
                let items: Vec<String> = arr
                    .iter()
                    .map(|v| {
                        let encoded = Self::json_encode(v)?;
                        match encoded {
                            Value::String(s) => Ok(s),
                            _ => Ok(encoded.to_string()),
                        }
                    })
                    .collect::<Result<Vec<String>, RuntimeError>>()?;
                format!("[{}]", items.join(", "))
            }
            Value::Map(map) => {
                let items: Vec<String> = map
                    .iter()
                    .map(|(k, v)| {
                        let key = format!("\"{}\"", Self::escape_json_string(k));
                        let encoded = Self::json_encode(v)?;
                        let val = match encoded {
                            Value::String(s) => s,
                            _ => encoded.to_string(),
                        };
                        Ok(format!("{}: {}", key, val))
                    })
                    .collect::<Result<Vec<String>, RuntimeError>>()?;
                format!("{{{}}}", items.join(", "))
            }
            Value::Set(set) => {
                // Sets are encoded as arrays
                let arr: Vec<Value> = set.to_vec();
                let encoded = Self::json_encode(&Value::Array(arr))?;
                match encoded {
                    Value::String(s) => s,
                    _ => encoded.to_string(),
                }
            }
            Value::Struct(name, fields) => {
                // Structs are encoded as objects
                let mut map = HashMap::new();
                map.insert("_type".to_string(), Value::String(name.clone()));
                for (k, v) in fields {
                    map.insert(k.clone(), v.clone());
                }
                let encoded = Self::json_encode(&Value::Map(map))?;
                match encoded {
                    Value::String(s) => s,
                    _ => encoded.to_string(),
                }
            }
            Value::Enum(name, variant) => {
                // Enums are encoded as objects
                let mut map = HashMap::new();
                map.insert("_type".to_string(), Value::String(name.clone()));
                map.insert("_variant".to_string(), Value::String(variant.clone()));
                let encoded = Self::json_encode(&Value::Map(map))?;
                match encoded {
                    Value::String(s) => s,
                    _ => encoded.to_string(),
                }
            }
            Value::Char(c) => {
                format!("\"{}\"", Self::escape_json_string(&c.to_string()))
            }
            Value::Function(_, _, _, _) => {
                return Err(RuntimeError::new(
                    "Cannot encode functions to JSON".to_string(),
                ));
            }
            Value::Result(ok, inner) => {
                let inner_str = match Self::json_encode(inner)? {
                    Value::String(s) => s,
                    other => other.to_string(),
                };
                if *ok {
                    format!("{{\"ok\":{}}}", inner_str)
                } else {
                    format!("{{\"err\":{}}}", inner_str)
                }
            }
            Value::Future(_) => "null".to_string(),
        };

        // If the result is already a JSON string (starts with "), return it as-is
        // Otherwise wrap it
        Ok(Value::String(json_str))
    }

    /// Decode a JSON string to Txtcode value
    fn json_decode(json_str: &str) -> Result<Value, RuntimeError> {
        // Simple JSON parser (handles basic cases)
        // For production, use a proper JSON library
        let trimmed = json_str.trim();

        if trimmed == "null" {
            return Ok(Value::Null);
        }

        if trimmed == "true" {
            return Ok(Value::Boolean(true));
        }

        if trimmed == "false" {
            return Ok(Value::Boolean(false));
        }

        // Try to parse as number
        if let Ok(i) = trimmed.parse::<i64>() {
            return Ok(Value::Integer(i));
        }

        if let Ok(f) = trimmed.parse::<f64>() {
            return Ok(Value::Float(f));
        }

        // Parse string
        if trimmed.starts_with('"') && trimmed.ends_with('"') {
            let unescaped = Self::unescape_json_string(&trimmed[1..trimmed.len() - 1]);
            return Ok(Value::String(unescaped));
        }

        // Parse array
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            return Self::parse_json_array(&trimmed[1..trimmed.len() - 1]);
        }

        // Parse object
        if trimmed.starts_with('{') && trimmed.ends_with('}') {
            return Self::parse_json_object(&trimmed[1..trimmed.len() - 1]);
        }

        Err(RuntimeError::new(format!("Invalid JSON: {}", json_str)))
    }

    fn parse_json_array(content: &str) -> Result<Value, RuntimeError> {
        let mut items = Vec::new();
        let mut current = String::new();
        let mut depth = 0;
        let mut in_string = false;
        let mut escape = false;

        for ch in content.chars() {
            if escape {
                current.push(ch);
                escape = false;
                continue;
            }

            if ch == '\\' && in_string {
                escape = true;
                current.push(ch);
                continue;
            }

            if ch == '"' {
                in_string = !in_string;
                current.push(ch);
                continue;
            }

            if in_string {
                current.push(ch);
                continue;
            }

            match ch {
                '[' | '{' => {
                    depth += 1;
                    current.push(ch);
                }
                ']' | '}' => {
                    depth -= 1;
                    current.push(ch);
                }
                ',' if depth == 0 => {
                    if !current.trim().is_empty() {
                        items.push(Self::json_decode(current.trim())?);
                    }
                    current.clear();
                }
                _ => {
                    current.push(ch);
                }
            }
        }

        if !current.trim().is_empty() {
            items.push(Self::json_decode(current.trim())?);
        }

        Ok(Value::Array(items))
    }

    fn parse_json_object(content: &str) -> Result<Value, RuntimeError> {
        let mut map = HashMap::new();
        let mut current_key: Option<String> = None;
        let mut current_value = String::new();
        let mut depth = 0;
        let mut in_string = false;
        let mut escape = false;
        let mut expecting_value = false;

        for ch in content.chars() {
            if escape {
                if expecting_value {
                    current_value.push(ch);
                }
                escape = false;
                continue;
            }

            if ch == '\\' && in_string {
                escape = true;
                if expecting_value {
                    current_value.push(ch);
                }
                continue;
            }

            if ch == '"' {
                in_string = !in_string;
                if expecting_value {
                    current_value.push(ch);
                }
                continue;
            }

            if in_string {
                if expecting_value {
                    current_value.push(ch);
                }
                continue;
            }

            match ch {
                '[' | '{' => {
                    depth += 1;
                    if expecting_value {
                        current_value.push(ch);
                    }
                }
                ']' | '}' => {
                    depth -= 1;
                    if expecting_value {
                        current_value.push(ch);
                    }
                }
                ':' if depth == 0 => {
                    expecting_value = true;
                }
                ',' if depth == 0 => {
                    if let Some(key) = current_key.take() {
                        let value = Self::json_decode(current_value.trim())?;
                        map.insert(Self::unescape_json_string(key.trim_matches('"')), value);
                    }
                    current_value.clear();
                    expecting_value = false;
                }
                _ => {
                    if expecting_value {
                        current_value.push(ch);
                    } else if ch.is_whitespace() {
                        // Skip whitespace before key
                    } else {
                        // This is part of the key
                        if current_key.is_none() {
                            current_key = Some(String::new());
                        }
                        if let Some(ref mut k) = current_key {
                            k.push(ch);
                        }
                    }
                }
            }
        }

        if let Some(key) = current_key {
            if !current_value.trim().is_empty() {
                let value = Self::json_decode(current_value.trim())?;
                map.insert(Self::unescape_json_string(key.trim_matches('"')), value);
            }
        }

        Ok(Value::Map(map))
    }

    fn escape_json_string(s: &str) -> String {
        s.chars()
            .map(|c| match c {
                '"' => "\\\"".to_string(),
                '\\' => "\\\\".to_string(),
                '\n' => "\\n".to_string(),
                '\r' => "\\r".to_string(),
                '\t' => "\\t".to_string(),
                '\x08' => "\\b".to_string(),
                '\x0c' => "\\f".to_string(),
                _ => c.to_string(),
            })
            .collect()
    }

    fn unescape_json_string(s: &str) -> String {
        let mut result = String::new();
        let mut chars = s.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '\\' {
                if let Some(next) = chars.next() {
                    match next {
                        '"' => result.push('"'),
                        '\\' => result.push('\\'),
                        'n' => result.push('\n'),
                        'r' => result.push('\r'),
                        't' => result.push('\t'),
                        'b' => result.push('\x08'),
                        'f' => result.push('\x0c'),
                        'u' => {
                            // Unicode escape (simplified - just skip for now)
                            let mut hex = String::new();
                            for _ in 0..4 {
                                if let Some(h) = chars.next() {
                                    hex.push(h);
                                }
                            }
                            if let Ok(code) = u32::from_str_radix(&hex, 16) {
                                if let Some(unicode) = char::from_u32(code) {
                                    result.push(unicode);
                                }
                            }
                        }
                        _ => {
                            result.push('\\');
                            result.push(next);
                        }
                    }
                } else {
                    result.push('\\');
                }
            } else {
                result.push(ch);
            }
        }

        result
    }
}
