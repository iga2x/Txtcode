use crate::runtime::{RuntimeError, Value};

/// URL encoding/decoding library
pub struct UrlLib;

impl UrlLib {
    /// Call a URL library function
    pub fn call_function(name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
        match name {
            "url_encode" | "encode_uri" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "url_encode requires 1 argument".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(s) => Ok(Value::String(Self::url_encode(s))),
                    _ => Err(RuntimeError::new(
                        "url_encode requires a string argument".to_string(),
                    )),
                }
            }
            "url_decode" | "decode_uri" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "url_decode requires 1 argument".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(s) => Self::url_decode(s)
                        .map(Value::String)
                        .map_err(|e| RuntimeError::new(format!("Invalid URL encoding: {}", e))),
                    _ => Err(RuntimeError::new(
                        "url_decode requires a string argument".to_string(),
                    )),
                }
            }
            "url_encode_component" | "encode_uri_component" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "url_encode_component requires 1 argument".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(s) => Ok(Value::String(Self::url_encode_component(s))),
                    _ => Err(RuntimeError::new(
                        "url_encode_component requires a string argument".to_string(),
                    )),
                }
            }
            "url_decode_component" | "decode_uri_component" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "url_decode_component requires 1 argument".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(s) => Self::url_decode(s)
                        .map(Value::String)
                        .map_err(|e| RuntimeError::new(format!("Invalid URL encoding: {}", e))),
                    _ => Err(RuntimeError::new(
                        "url_decode_component requires a string argument".to_string(),
                    )),
                }
            }
            _ => Err(RuntimeError::new(format!("Unknown URL function: {}", name))),
        }
    }

    fn url_encode(s: &str) -> String {
        let mut encoded = String::new();
        for byte in s.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    encoded.push(byte as char);
                }
                b' ' => {
                    encoded.push('+');
                }
                _ => {
                    encoded.push_str(&format!("%{:02X}", byte));
                }
            }
        }
        encoded
    }

    fn url_encode_component(s: &str) -> String {
        let mut encoded = String::new();
        for byte in s.bytes() {
            match byte {
                b'A'..=b'Z'
                | b'a'..=b'z'
                | b'0'..=b'9'
                | b'-'
                | b'_'
                | b'.'
                | b'!'
                | b'*'
                | b'\''
                | b'('
                | b')' => {
                    encoded.push(byte as char);
                }
                _ => {
                    encoded.push_str(&format!("%{:02X}", byte));
                }
            }
        }
        encoded
    }

    fn url_decode(s: &str) -> Result<String, String> {
        let mut decoded = Vec::new();
        let mut chars = s.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '+' => {
                    decoded.push(b' ');
                }
                '%' => {
                    let hex1 = chars.next().ok_or("Incomplete percent encoding")?;
                    let hex2 = chars.next().ok_or("Incomplete percent encoding")?;
                    let hex_str = format!("{}{}", hex1, hex2);
                    let byte = u8::from_str_radix(&hex_str, 16)
                        .map_err(|_| format!("Invalid hex in percent encoding: {}", hex_str))?;
                    decoded.push(byte);
                }
                _ => {
                    decoded.push(ch as u8);
                }
            }
        }

        String::from_utf8(decoded).map_err(|e| format!("Invalid UTF-8 in decoded string: {}", e))
    }
}
