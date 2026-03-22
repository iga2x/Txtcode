//! Bytes stdlib — operations on `Value::Bytes(Vec<u8>)`.

use crate::runtime::{RuntimeError, Value};
use std::sync::Arc;

pub struct BytesLib;

impl BytesLib {
    pub fn call_function(name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
        match name {
            // bytes_new(len) → Bytes of zeros
            "bytes_new" => {
                let len = match args.first() {
                    Some(Value::Integer(n)) if *n >= 0 => *n as usize,
                    _ => return Err(RuntimeError::new("bytes_new(len): expected non-negative integer".to_string())),
                };
                Ok(Value::Bytes(vec![0u8; len]))
            }
            // bytes_from_hex(s) → Bytes
            "bytes_from_hex" => {
                let s = match args.first() {
                    Some(Value::String(s)) => s.as_ref(),
                    _ => return Err(RuntimeError::new("bytes_from_hex(s): expected string".to_string())),
                };
                let s = s.trim_start_matches("0x").trim_start_matches("0X");
                if s.len() % 2 != 0 {
                    return Err(RuntimeError::new("bytes_from_hex: hex string must have even length".to_string()));
                }
                let bytes: Result<Vec<u8>, _> = (0..s.len())
                    .step_by(2)
                    .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
                    .collect();
                bytes.map(Value::Bytes).map_err(|_| RuntimeError::new("bytes_from_hex: invalid hex character".to_string()))
            }
            // bytes_to_hex(b) → String
            "bytes_to_hex" => {
                let b = match args.first() {
                    Some(Value::Bytes(b)) => b,
                    _ => return Err(RuntimeError::new("bytes_to_hex(b): expected bytes".to_string())),
                };
                let hex: String = b.iter().map(|byte| format!("{:02x}", byte)).collect();
                Ok(Value::String(Arc::from(hex)))
            }
            // bytes_get(b, i) → Integer
            "bytes_get" => {
                let (b, idx) = match (args.first(), args.get(1)) {
                    (Some(Value::Bytes(b)), Some(Value::Integer(i))) => (b, *i),
                    _ => return Err(RuntimeError::new("bytes_get(b, i): expected bytes and integer".to_string())),
                };
                if idx < 0 || idx as usize >= b.len() {
                    return Err(RuntimeError::new(format!("bytes_get: index {} out of bounds (len {})", idx, b.len())));
                }
                Ok(Value::Integer(b[idx as usize] as i64))
            }
            // bytes_set(b, i, v) → Bytes
            "bytes_set" => {
                let (b, idx, val) = match (args.first(), args.get(1), args.get(2)) {
                    (Some(Value::Bytes(b)), Some(Value::Integer(i)), Some(Value::Integer(v))) => (b, *i, *v),
                    _ => return Err(RuntimeError::new("bytes_set(b, i, v): expected bytes, integer, integer".to_string())),
                };
                if idx < 0 || idx as usize >= b.len() {
                    return Err(RuntimeError::new(format!("bytes_set: index {} out of bounds (len {})", idx, b.len())));
                }
                if val < 0 || val > 255 {
                    return Err(RuntimeError::new(format!("bytes_set: value {} is not a valid byte (0-255)", val)));
                }
                let mut new_b = b.clone();
                new_b[idx as usize] = val as u8;
                Ok(Value::Bytes(new_b))
            }
            // bytes_len(b) → Integer
            "bytes_len" => {
                match args.first() {
                    Some(Value::Bytes(b)) => Ok(Value::Integer(b.len() as i64)),
                    _ => Err(RuntimeError::new("bytes_len(b): expected bytes".to_string())),
                }
            }
            // bytes_slice(b, start, end) → Bytes
            "bytes_slice" => {
                let (b, start, end) = match (args.first(), args.get(1), args.get(2)) {
                    (Some(Value::Bytes(b)), Some(Value::Integer(s)), Some(Value::Integer(e))) => (b, *s as usize, *e as usize),
                    _ => return Err(RuntimeError::new("bytes_slice(b, start, end): expected bytes and two integers".to_string())),
                };
                if start > end || end > b.len() {
                    return Err(RuntimeError::new(format!("bytes_slice: invalid range {}..{} for len {}", start, end, b.len())));
                }
                Ok(Value::Bytes(b[start..end].to_vec()))
            }
            // bytes_concat(b1, b2) → Bytes
            "bytes_concat" => {
                let (b1, b2) = match (args.first(), args.get(1)) {
                    (Some(Value::Bytes(a)), Some(Value::Bytes(b))) => (a, b),
                    _ => return Err(RuntimeError::new("bytes_concat(b1, b2): expected two bytes arguments".to_string())),
                };
                let mut result = b1.clone();
                result.extend_from_slice(b2);
                Ok(Value::Bytes(result))
            }

            // ── Group 27.3: Gzip compression ──────────────────────────────

            // gzip_compress(bytes_or_string) → Bytes
            "gzip_compress" => {
                use flate2::write::GzEncoder;
                use flate2::Compression;
                use std::io::Write;
                let data: Vec<u8> = match args.first() {
                    Some(Value::Bytes(b)) => b.clone(),
                    Some(Value::String(s)) => s.as_bytes().to_vec(),
                    _ => return Err(RuntimeError::new(
                        "gzip_compress(data): expected bytes or string".to_string(),
                    )),
                };
                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(&data).map_err(|e| {
                    RuntimeError::new(format!("gzip_compress: write error: {}", e))
                })?;
                let compressed = encoder.finish().map_err(|e| {
                    RuntimeError::new(format!("gzip_compress: finish error: {}", e))
                })?;
                Ok(Value::Bytes(compressed))
            }

            // gzip_decompress(bytes) → Bytes
            "gzip_decompress" => {
                use flate2::read::GzDecoder;
                use std::io::Read;
                let data: &[u8] = match args.first() {
                    Some(Value::Bytes(b)) => b,
                    _ => return Err(RuntimeError::new(
                        "gzip_decompress(data): expected bytes".to_string(),
                    )),
                };
                let mut decoder = GzDecoder::new(data);
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed).map_err(|e| {
                    RuntimeError::new(format!("gzip_decompress: decompress error: {}", e))
                })?;
                Ok(Value::Bytes(decompressed))
            }

            // gzip_compress_string(s) → Bytes  (convenience alias)
            "gzip_compress_string" => {
                use flate2::write::GzEncoder;
                use flate2::Compression;
                use std::io::Write;
                let s = match args.first() {
                    Some(Value::String(s)) => s.as_bytes().to_vec(),
                    _ => return Err(RuntimeError::new(
                        "gzip_compress_string(s): expected string".to_string(),
                    )),
                };
                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(&s).map_err(|e| {
                    RuntimeError::new(format!("gzip_compress_string: error: {}", e))
                })?;
                let compressed = encoder.finish().map_err(|e| {
                    RuntimeError::new(format!("gzip_compress_string: finish error: {}", e))
                })?;
                Ok(Value::Bytes(compressed))
            }

            // gzip_decompress_string(bytes) → String  (UTF-8 decompressed content)
            "gzip_decompress_string" => {
                use flate2::read::GzDecoder;
                use std::io::Read;
                let data: &[u8] = match args.first() {
                    Some(Value::Bytes(b)) => b,
                    _ => return Err(RuntimeError::new(
                        "gzip_decompress_string(data): expected bytes".to_string(),
                    )),
                };
                let mut decoder = GzDecoder::new(data);
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed).map_err(|e| {
                    RuntimeError::new(format!("gzip_decompress_string: decompress error: {}", e))
                })?;
                let s = String::from_utf8(decompressed).map_err(|e| {
                    RuntimeError::new(format!("gzip_decompress_string: UTF-8 error: {}", e))
                })?;
                Ok(Value::String(Arc::from(s)))
            }

            _ => Err(RuntimeError::new(format!("Unknown bytes function: {}", name))),
        }
    }
}
