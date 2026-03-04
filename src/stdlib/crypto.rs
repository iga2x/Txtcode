use crate::runtime::{Value, RuntimeError};
use sha2::{Sha256, Sha512, Digest};
use ring::rand::{SecureRandom, SystemRandom};
use hex;
use base64::{Engine as _, engine::general_purpose};

/// Cryptography library
pub struct CryptoLib;

impl CryptoLib {
    /// Call a crypto library function
    pub fn call_function(name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
        match name {
            "sha256" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("sha256 requires 1 argument".to_string()));
                }
                match &args[0] {
                    Value::String(s) => {
                        let mut hasher = Sha256::new();
                        hasher.update(s.as_bytes());
                        let hash = hasher.finalize();
                        Ok(Value::String(hex::encode(hash)))
                    }
                    _ => Err(RuntimeError::new("sha256 requires a string".to_string())),
                }
            }
            "sha512" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("sha512 requires 1 argument".to_string()));
                }
                match &args[0] {
                    Value::String(s) => {
                        let mut hasher = Sha512::new();
                        hasher.update(s.as_bytes());
                        let hash = hasher.finalize();
                        Ok(Value::String(hex::encode(hash)))
                    }
                    _ => Err(RuntimeError::new("sha512 requires a string".to_string())),
                }
            }
            "random_bytes" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("random_bytes requires 1 argument".to_string()));
                }
                match &args[0] {
                    Value::Integer(n) => {
                        if *n < 0 || *n > 1024 {
                            return Err(RuntimeError::new("random_bytes size must be between 0 and 1024".to_string()));
                        }
                        let mut bytes = vec![0u8; *n as usize];
                        let rng = SystemRandom::new();
                        rng.fill(&mut bytes).map_err(|e| RuntimeError::new(format!("Failed to generate random bytes: {}", e)))?;
                        Ok(Value::String(hex::encode(bytes)))
                    }
                    _ => Err(RuntimeError::new("random_bytes requires an integer".to_string())),
                }
            }
            "random_int" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new("random_int requires 2 arguments (min, max)".to_string()));
                }
                match (&args[0], &args[1]) {
                    (Value::Integer(min), Value::Integer(max)) => {
                        use rand::Rng;
                        let mut rng = rand::thread_rng();
                        let result = rng.gen_range(*min..=*max);
                        Ok(Value::Integer(result))
                    }
                    _ => Err(RuntimeError::new("random_int requires integers".to_string())),
                }
            }
            "encrypt" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new("encrypt requires 2 arguments (data, key)".to_string()));
                }
                match (&args[0], &args[1]) {
                    (Value::String(data), Value::String(key)) => {
                        // Simple XOR encryption (for demonstration)
                        // In production, use proper AES encryption
                        let key_bytes = key.as_bytes();
                        let mut encrypted = Vec::new();
                        for (i, byte) in data.bytes().enumerate() {
                            encrypted.push(byte ^ key_bytes[i % key_bytes.len()]);
                        }
                        Ok(Value::String(hex::encode(encrypted)))
                    }
                    _ => Err(RuntimeError::new("encrypt requires strings".to_string())),
                }
            }
            "decrypt" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new("decrypt requires 2 arguments (data, key)".to_string()));
                }
                match (&args[0], &args[1]) {
                    (Value::String(data_hex), Value::String(key)) => {
                        let data = hex::decode(data_hex).map_err(|_| RuntimeError::new("Invalid hex string".to_string()))?;
                        let key_bytes = key.as_bytes();
                        let mut decrypted = Vec::new();
                        for (i, byte) in data.iter().enumerate() {
                            decrypted.push(byte ^ key_bytes[i % key_bytes.len()]);
                        }
                        Ok(Value::String(String::from_utf8(decrypted).map_err(|_| RuntimeError::new("Invalid UTF-8 in decrypted data".to_string()))?))
                    }
                    _ => Err(RuntimeError::new("decrypt requires strings".to_string())),
                }
            }
            "md5" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("md5 requires 1 argument".to_string()));
                }
                match &args[0] {
                    Value::String(s) => {
                        let hash = md5::compute(s.as_bytes());
                        Ok(Value::String(hex::encode(hash.0)))
                    }
                    _ => Err(RuntimeError::new("md5 requires a string".to_string())),
                }
            }
            "base64_encode" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("base64_encode requires 1 argument".to_string()));
                }
                match &args[0] {
                    Value::String(s) => {
                        Ok(Value::String(general_purpose::STANDARD.encode(s.as_bytes())))
                    }
                    _ => Err(RuntimeError::new("base64_encode requires a string".to_string())),
                }
            }
            "base64_decode" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("base64_decode requires 1 argument".to_string()));
                }
                match &args[0] {
                    Value::String(s) => {
                        let decoded = general_purpose::STANDARD.decode(s)
                            .map_err(|e| RuntimeError::new(format!("Invalid base64: {}", e)))?;
                        Ok(Value::String(String::from_utf8(decoded)
                            .map_err(|_| RuntimeError::new("Invalid UTF-8 in decoded data".to_string()))?))
                    }
                    _ => Err(RuntimeError::new("base64_decode requires a string".to_string())),
                }
            }
            _ => Err(RuntimeError::new(format!("Unknown crypto function: {}", name))),
        }
    }
}
