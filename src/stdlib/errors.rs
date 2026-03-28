/// Task E.3 — Standard Error Types
///
/// Provides 8 typed error constructors that return structured
/// `Value::Struct("ErrorKind", fields)` values, enabling pattern matching
/// in catch blocks:
///
/// ```text
/// catch e
///   match → e
///     is → FileNotFoundError → println("File missing")
///     is → NetworkError      → println("Network issue")
///     _                      → println("Unknown")
///   end
/// end
/// ```
///
/// Each constructor returns a Map value with an `_error_type` key so
/// scripts can inspect the type without runtime support changes.
use crate::runtime::core::Value;
use std::sync::Arc;
use crate::runtime::errors::RuntimeError;

pub struct ErrorLib;

impl ErrorLib {
    pub fn call_function(name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
        match name {
            // FileNotFoundError(path: string) → error
            "FileNotFoundError" => {
                let path = str_arg(args, 0, name).unwrap_or_default();
                Ok(make_error("FileNotFoundError", [
                    ("_error_type", Value::String(Arc::from("FileNotFoundError"))),
                    ("path",        Value::String(Arc::from(path))),
                    ("message",     Value::String(Arc::from(format!("File not found: {}", str_arg(args, 0, name).unwrap_or_default())))),
                ]))
            }

            // PermissionError(action: string, resource: string) → error
            "PermissionError" => {
                let action   = str_arg(args, 0, name).unwrap_or_default();
                let resource = str_arg(args, 1, name).unwrap_or_default();
                Ok(make_error("PermissionError", [
                    ("_error_type", Value::String(Arc::from("PermissionError"))),
                    ("action",      Value::String(Arc::from(action.clone()))),
                    ("resource",    Value::String(Arc::from(resource.clone()))),
                    ("message",     Value::String(Arc::from(format!("Permission denied: {} on {}", action, resource)))),
                ]))
            }

            // NetworkError(url: string, reason: string) → error
            "NetworkError" => {
                let url    = str_arg(args, 0, name).unwrap_or_default();
                let reason = str_arg(args, 1, name).unwrap_or_default();
                Ok(make_error("NetworkError", [
                    ("_error_type", Value::String(Arc::from("NetworkError"))),
                    ("url",         Value::String(Arc::from(url.clone()))),
                    ("reason",      Value::String(Arc::from(reason.clone()))),
                    ("message",     Value::String(Arc::from(format!("Network error: {} ({})", url, reason)))),
                ]))
            }

            // ParseError(input: string, position: int) → error
            "ParseError" => {
                let input    = str_arg(args, 0, name).unwrap_or_default();
                let position = int_arg(args, 1).unwrap_or(0);
                Ok(make_error("ParseError", [
                    ("_error_type", Value::String(Arc::from("ParseError"))),
                    ("input",       Value::String(Arc::from(input.clone()))),
                    ("position",    Value::Integer(position)),
                    ("message",     Value::String(Arc::from(format!("Parse error at position {}: {}", position, input)))),
                ]))
            }

            // TypeError(expected: string, got: string) → error
            "TypeError" => {
                let expected = str_arg(args, 0, name).unwrap_or_default();
                let got      = str_arg(args, 1, name).unwrap_or_default();
                Ok(make_error("TypeError", [
                    ("_error_type", Value::String(Arc::from("TypeError"))),
                    ("expected",    Value::String(Arc::from(expected.clone()))),
                    ("got",         Value::String(Arc::from(got.clone()))),
                    ("message",     Value::String(Arc::from(format!("Type error: expected {} got {}", expected, got)))),
                ]))
            }

            // ValueError(message: string) → error
            "ValueError" => {
                let msg = str_arg(args, 0, name).unwrap_or_default();
                Ok(make_error("ValueError", [
                    ("_error_type", Value::String(Arc::from("ValueError"))),
                    ("message",     Value::String(Arc::from(msg))),
                ]))
            }

            // IndexError(index: int, length: int) → error
            "IndexError" => {
                let index  = int_arg(args, 0).unwrap_or(0);
                let length = int_arg(args, 1).unwrap_or(0);
                Ok(make_error("IndexError", [
                    ("_error_type", Value::String(Arc::from("IndexError"))),
                    ("index",       Value::Integer(index)),
                    ("length",      Value::Integer(length)),
                    ("message",     Value::String(Arc::from(format!("Index {} out of range (length {})", index, length)))),
                ]))
            }

            // TimeoutError(limit_ms: int) → error
            "TimeoutError" => {
                let limit = int_arg(args, 0).unwrap_or(0);
                Ok(make_error("TimeoutError", [
                    ("_error_type", Value::String(Arc::from("TimeoutError"))),
                    ("limit_ms",    Value::Integer(limit)),
                    ("message",     Value::String(Arc::from(format!("Operation timed out after {}ms", limit)))),
                ]))
            }

            _ => Err(RuntimeError::new(format!("Unknown error constructor: {}", name))),
        }
    }

    /// Check if a value is a typed error struct of a specific kind.
    /// Used by stdlib functions that want to return typed errors.
    pub fn is_error_type(value: &Value, kind: &str) -> bool {
        if let Value::Map(fields) = value {
            if let Some(Value::String(t)) = fields.get("_error_type") {
                return t.as_ref() == kind;
            }
        }
        false
    }
}

/// Build a typed error `Value::Map` with the given fields.
fn make_error<const N: usize>(
    _kind: &str,
    fields: [(&str, Value); N],
) -> Value {
    let mut map = indexmap::IndexMap::new();
    for (k, v) in fields {
        map.insert(k.to_string(), v);
    }
    Value::Map(map)
}

/// Extract a string from args\[index\], returning None on missing/wrong type.
pub fn str_arg(args: &[Value], index: usize, _fn_name: &str) -> Option<String> {
    match args.get(index) {
        Some(Value::String(s)) => Some(s.to_string()),
        Some(v) => Some(format!("{}", v)),
        None => None,
    }
}

/// Extract an integer from args\[index\], returning None on missing/wrong type.
pub fn int_arg(args: &[Value], index: usize) -> Option<i64> {
    match args.get(index) {
        Some(Value::Integer(n)) => Some(*n),
        Some(Value::Float(f)) => Some(*f as i64),
        _ => None,
    }
}

/// Convenience: produce a `FileNotFoundError` Value directly (used by read_file).
pub fn file_not_found(path: &str) -> Value {
    make_error("FileNotFoundError", [
        ("_error_type", Value::String(Arc::from("FileNotFoundError"))),
        ("path",        Value::String(Arc::from(path.to_string()))),
        ("message",     Value::String(Arc::from(format!("File not found: {}", path)))),
    ])
}

/// Convenience: produce a `ParseError` Value directly (used by json_parse).
pub fn parse_error(input: &str, reason: &str) -> Value {
    make_error("ParseError", [
        ("_error_type", Value::String(Arc::from("ParseError"))),
        ("input",       Value::String(Arc::from(input.chars().take(80).collect::<String>()))),
        ("position",    Value::Integer(0)),
        ("message",     Value::String(Arc::from(format!("Parse error: {}", reason)))),
    ])
}

/// Convenience: produce a `NetworkError` Value directly (used by http_get).
pub fn network_error(url: &str, reason: &str) -> Value {
    make_error("NetworkError", [
        ("_error_type", Value::String(Arc::from("NetworkError"))),
        ("url",         Value::String(Arc::from(url.to_string()))),
        ("reason",      Value::String(Arc::from(reason.to_string()))),
        ("message",     Value::String(Arc::from(format!("Network error connecting to {}: {}", url, reason)))),
    ])
}
