use crate::runtime::{RuntimeError, Value};
use base64::{engine::general_purpose, Engine as _};
use hex;
use ring::rand::{SecureRandom, SystemRandom};
use sha2::{Digest, Sha256, Sha512};

/// Cryptography library
pub struct CryptoLib;

impl CryptoLib {
    /// Call a crypto library function.
    /// `seed_override`: when `Some(s)`, seeded PRNG is used instead of OS entropy (deterministic mode).
    pub fn call_function(name: &str, args: &[Value], seed_override: Option<u64>) -> Result<Value, RuntimeError> {
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
            // crypto_random_bytes: cryptographically secure random bytes via OS entropy (ring).
            // Use this when security matters. For non-secure use, use math_random_int / math_random_float.
            "crypto_random_bytes" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "crypto_random_bytes requires 1 argument (size: int)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::Integer(n) => {
                        if *n < 0 || *n > 1024 {
                            return Err(RuntimeError::new(
                                "crypto_random_bytes size must be between 0 and 1024".to_string(),
                            ));
                        }
                        let mut bytes = vec![0u8; *n as usize];
                        let rng = SystemRandom::new();
                        rng.fill(&mut bytes).map_err(|e| {
                            RuntimeError::new(format!("Failed to generate random bytes: {}", e))
                        })?;
                        Ok(Value::String(hex::encode(bytes)))
                    }
                    _ => Err(RuntimeError::new(
                        "crypto_random_bytes requires an integer".to_string(),
                    )),
                }
            }
            // crypto_random_int: cryptographically seeded integer in [min, max].
            // For non-secure random integers, use math_random_int instead.
            "crypto_random_int" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "crypto_random_int requires 2 arguments (min, max)".to_string(),
                    ));
                }
                match (&args[0], &args[1]) {
                    (Value::Integer(min), Value::Integer(max)) => {
                        use rand::Rng;
                        let result = if let Some(s) = seed_override {
                            use rand::{rngs::StdRng, SeedableRng};
                            let mut rng = StdRng::seed_from_u64(s);
                            rng.gen_range(*min..=*max)
                        } else {
                            rand::thread_rng().gen_range(*min..=*max)
                        };
                        Ok(Value::Integer(result))
                    }
                    _ => Err(RuntimeError::new(
                        "crypto_random_int requires integers".to_string(),
                    )),
                }
            }
            "encrypt" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "encrypt requires 2 arguments (data, key)".to_string(),
                    ));
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
                    return Err(RuntimeError::new(
                        "decrypt requires 2 arguments (data, key)".to_string(),
                    ));
                }
                match (&args[0], &args[1]) {
                    (Value::String(data_hex), Value::String(key)) => {
                        let data = hex::decode(data_hex)
                            .map_err(|_| RuntimeError::new("Invalid hex string".to_string()))?;
                        let key_bytes = key.as_bytes();
                        let mut decrypted = Vec::new();
                        for (i, byte) in data.iter().enumerate() {
                            decrypted.push(byte ^ key_bytes[i % key_bytes.len()]);
                        }
                        Ok(Value::String(String::from_utf8(decrypted).map_err(
                            |_| RuntimeError::new("Invalid UTF-8 in decrypted data".to_string()),
                        )?))
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
                    return Err(RuntimeError::new(
                        "base64_encode requires 1 argument".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(s) => Ok(Value::String(
                        general_purpose::STANDARD.encode(s.as_bytes()),
                    )),
                    _ => Err(RuntimeError::new(
                        "base64_encode requires a string".to_string(),
                    )),
                }
            }
            "base64_decode" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "base64_decode requires 1 argument".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(s) => {
                        let decoded = general_purpose::STANDARD
                            .decode(s)
                            .map_err(|e| RuntimeError::new(format!("Invalid base64: {}", e)))?;
                        Ok(Value::String(String::from_utf8(decoded).map_err(|_| {
                            RuntimeError::new("Invalid UTF-8 in decoded data".to_string())
                        })?))
                    }
                    _ => Err(RuntimeError::new(
                        "base64_decode requires a string".to_string(),
                    )),
                }
            }
            "hmac_sha256" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "hmac_sha256 requires 2 arguments (key, data)".to_string(),
                    ));
                }
                match (&args[0], &args[1]) {
                    (Value::String(key), Value::String(data)) => {
                        use ring::hmac;
                        let key = hmac::Key::new(hmac::HMAC_SHA256, key.as_bytes());
                        let tag = hmac::sign(&key, data.as_bytes());
                        Ok(Value::String(hex::encode(tag.as_ref())))
                    }
                    _ => Err(RuntimeError::new(
                        "hmac_sha256 requires string arguments".to_string(),
                    )),
                }
            }
            "uuid_v4" => {
                if !args.is_empty() {
                    return Err(RuntimeError::new("uuid_v4 takes no arguments".to_string()));
                }
                let mut bytes = [0u8; 16];
                let rng = SystemRandom::new();
                rng.fill(&mut bytes).map_err(|_| {
                    RuntimeError::new("Failed to generate random bytes for UUID".to_string())
                })?;
                // Set version 4 bits
                bytes[6] = (bytes[6] & 0x0f) | 0x40;
                // Set variant bits (10xx)
                bytes[8] = (bytes[8] & 0x3f) | 0x80;
                let uuid = format!(
                    "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                    bytes[0], bytes[1], bytes[2], bytes[3],
                    bytes[4], bytes[5],
                    bytes[6], bytes[7],
                    bytes[8], bytes[9],
                    bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
                );
                Ok(Value::String(uuid))
            }
            "secure_compare" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "secure_compare requires 2 arguments (a, b)".to_string(),
                    ));
                }
                match (&args[0], &args[1]) {
                    (Value::String(a), Value::String(b)) => {
                        let a_bytes = a.as_bytes();
                        let b_bytes = b.as_bytes();
                        if a_bytes.len() != b_bytes.len() {
                            return Ok(Value::Boolean(false));
                        }
                        let mut diff: u8 = 0;
                        for (x, y) in a_bytes.iter().zip(b_bytes.iter()) {
                            diff |= x ^ y;
                        }
                        Ok(Value::Boolean(diff == 0))
                    }
                    _ => Err(RuntimeError::new(
                        "secure_compare requires string arguments".to_string(),
                    )),
                }
            }
            "pbkdf2" => {
                if args.len() < 3 || args.len() > 4 {
                    return Err(RuntimeError::new(
                        "pbkdf2 requires 3-4 arguments (password, salt, iterations, key_len?)"
                            .to_string(),
                    ));
                }
                match (&args[0], &args[1], &args[2]) {
                    (Value::String(password), Value::String(salt), Value::Integer(iterations)) => {
                        use ring::pbkdf2;
                        use std::num::NonZeroU32;
                        let key_len = if args.len() == 4 {
                            match &args[3] {
                                Value::Integer(n) => *n as usize,
                                _ => 32,
                            }
                        } else {
                            32
                        };
                        if key_len == 0 || key_len > 512 {
                            return Err(RuntimeError::new(
                                "pbkdf2 key_len must be 1-512".to_string(),
                            ));
                        }
                        if *iterations < 1 {
                            return Err(RuntimeError::new(
                                "pbkdf2 iterations must be >= 1".to_string(),
                            ));
                        }
                        let iter = NonZeroU32::new(*iterations as u32).ok_or_else(|| {
                            RuntimeError::new("pbkdf2 iterations must be > 0".to_string())
                        })?;
                        let mut derived = vec![0u8; key_len];
                        pbkdf2::derive(
                            pbkdf2::PBKDF2_HMAC_SHA256,
                            iter,
                            salt.as_bytes(),
                            password.as_bytes(),
                            &mut derived,
                        );
                        Ok(Value::String(hex::encode(derived)))
                    }
                    _ => Err(RuntimeError::new(
                        "pbkdf2 requires (string, string, int) arguments".to_string(),
                    )),
                }
            }
            "bcrypt_hash" => {
                #[cfg(not(feature = "crypto-advanced"))]
                return Err(RuntimeError::new(
                    "bcrypt_hash requires the 'crypto-advanced' feature. \
                     Rebuild with: cargo build --features crypto-advanced"
                        .to_string(),
                ));
                #[cfg(feature = "crypto-advanced")]
                {
                    if args.is_empty() || args.len() > 2 {
                        return Err(RuntimeError::new(
                            "bcrypt_hash requires 1-2 arguments (password, cost?)".to_string(),
                        ));
                    }
                    let password = match &args[0] {
                        Value::String(s) => s.clone(),
                        _ => {
                            return Err(RuntimeError::new(
                                "bcrypt_hash: password must be a string".to_string(),
                            ))
                        }
                    };
                    let cost = match args.get(1) {
                        Some(Value::Integer(n)) => *n as u32,
                        None => 12,
                        _ => {
                            return Err(RuntimeError::new(
                                "bcrypt_hash: cost must be an integer".to_string(),
                            ))
                        }
                    };
                    if !(4..=31).contains(&cost) {
                        return Err(RuntimeError::new(
                            "bcrypt_hash: cost must be between 4 and 31".to_string(),
                        ));
                    }
                    bcrypt::hash(&password, cost)
                        .map(Value::String)
                        .map_err(|e| RuntimeError::new(format!("bcrypt_hash failed: {}", e)))
                }
            }
            "bcrypt_verify" => {
                #[cfg(not(feature = "crypto-advanced"))]
                return Err(RuntimeError::new(
                    "bcrypt_verify requires the 'crypto-advanced' feature. \
                     Rebuild with: cargo build --features crypto-advanced"
                        .to_string(),
                ));
                #[cfg(feature = "crypto-advanced")]
                {
                    if args.len() != 2 {
                        return Err(RuntimeError::new(
                            "bcrypt_verify requires 2 arguments (password, hash)".to_string(),
                        ));
                    }
                    match (&args[0], &args[1]) {
                        (Value::String(password), Value::String(hash)) => {
                            bcrypt::verify(password, hash)
                                .map(Value::Boolean)
                                .map_err(|e| RuntimeError::new(format!("bcrypt_verify failed: {}", e)))
                        }
                        _ => Err(RuntimeError::new(
                            "bcrypt_verify requires string arguments".to_string(),
                        )),
                    }
                }
            }
            "ed25519_sign" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "ed25519_sign requires 2 arguments (seed_hex, message)".to_string(),
                    ));
                }
                match (&args[0], &args[1]) {
                    (Value::String(seed_hex), Value::String(message)) => {
                        use ring::signature;
                        let seed = hex::decode(seed_hex).map_err(|_| {
                            RuntimeError::new(
                                "ed25519_sign: seed_hex must be valid hex".to_string(),
                            )
                        })?;
                        let key_pair = signature::Ed25519KeyPair::from_seed_unchecked(&seed)
                            .map_err(|_| {
                                RuntimeError::new(
                                    "ed25519_sign: invalid seed (must be 32 bytes)".to_string(),
                                )
                            })?;
                        let sig = key_pair.sign(message.as_bytes());
                        Ok(Value::String(hex::encode(sig.as_ref())))
                    }
                    _ => Err(RuntimeError::new(
                        "ed25519_sign requires string arguments".to_string(),
                    )),
                }
            }
            "ed25519_verify" => {
                if args.len() != 3 {
                    return Err(RuntimeError::new(
                        "ed25519_verify requires 3 arguments (pubkey_hex, message, signature_hex)"
                            .to_string(),
                    ));
                }
                match (&args[0], &args[1], &args[2]) {
                    (Value::String(pubkey_hex), Value::String(message), Value::String(sig_hex)) => {
                        use ring::signature;
                        let pubkey_bytes = hex::decode(pubkey_hex).map_err(|_| {
                            RuntimeError::new(
                                "ed25519_verify: pubkey_hex must be valid hex".to_string(),
                            )
                        })?;
                        let sig_bytes = hex::decode(sig_hex).map_err(|_| {
                            RuntimeError::new(
                                "ed25519_verify: signature_hex must be valid hex".to_string(),
                            )
                        })?;
                        let public_key =
                            signature::UnparsedPublicKey::new(&signature::ED25519, pubkey_bytes);
                        Ok(Value::Boolean(
                            public_key.verify(message.as_bytes(), &sig_bytes).is_ok(),
                        ))
                    }
                    _ => Err(RuntimeError::new(
                        "ed25519_verify requires string arguments".to_string(),
                    )),
                }
            }
            "rsa_generate" => {
                #[cfg(not(feature = "crypto-advanced"))]
                return Err(RuntimeError::new(
                    "rsa_generate requires the 'crypto-advanced' feature. \
                     Rebuild with: cargo build --features crypto-advanced"
                        .to_string(),
                ));
                #[cfg(feature = "crypto-advanced")]
                {
                    use rsa::pkcs1::{EncodeRsaPrivateKey, EncodeRsaPublicKey, LineEnding};
                    use rsa::RsaPrivateKey;
                    let bits = match args.first() {
                        Some(Value::Integer(b)) => *b as usize,
                        None => 2048,
                        _ => {
                            return Err(RuntimeError::new(
                                "rsa_generate: optional arg is key size in bits (default 2048)"
                                    .to_string(),
                            ))
                        }
                    };
                    if !(1024..=4096).contains(&bits) {
                        return Err(RuntimeError::new(
                            "rsa_generate: key size must be 1024–4096 bits".to_string(),
                        ));
                    }
                    let mut rng = rand::rngs::OsRng;
                    let private_key = RsaPrivateKey::new(&mut rng, bits)
                        .map_err(|e| RuntimeError::new(format!("rsa_generate failed: {}", e)))?;
                    let public_key = rsa::RsaPublicKey::from(&private_key);
                    let priv_pem = private_key.to_pkcs1_pem(LineEnding::LF).map_err(|e| {
                        RuntimeError::new(format!("rsa_generate: key serialization failed: {}", e))
                    })?;
                    let pub_pem = public_key.to_pkcs1_pem(LineEnding::LF).map_err(|e| {
                        RuntimeError::new(format!(
                            "rsa_generate: pubkey serialization failed: {}",
                            e
                        ))
                    })?;
                    let mut map = std::collections::HashMap::new();
                    map.insert(
                        "private_key".to_string(),
                        Value::String(priv_pem.to_string()),
                    );
                    map.insert("public_key".to_string(), Value::String(pub_pem.to_string()));
                    Ok(Value::Map(map))
                }
            }
            "rsa_sign" => {
                #[cfg(not(feature = "crypto-advanced"))]
                return Err(RuntimeError::new(
                    "rsa_sign requires the 'crypto-advanced' feature. \
                     Rebuild with: cargo build --features crypto-advanced"
                        .to_string(),
                ));
                #[cfg(feature = "crypto-advanced")]
                {
                    if args.len() != 2 {
                        return Err(RuntimeError::new(
                            "rsa_sign requires 2 arguments (private_key_pem, message)".to_string(),
                        ));
                    }
                    match (&args[0], &args[1]) {
                        (Value::String(priv_pem), Value::String(message)) => {
                            use rsa::pkcs1::DecodeRsaPrivateKey;
                            use rsa::pkcs1v15::SigningKey;
                            use rsa::signature::Signer;
                            use rsa::RsaPrivateKey;
                            use sha2::Sha256;
                            let private_key =
                                RsaPrivateKey::from_pkcs1_pem(priv_pem).map_err(|e| {
                                    RuntimeError::new(format!(
                                        "rsa_sign: invalid private key: {}",
                                        e
                                    ))
                                })?;
                            let signing_key = SigningKey::<Sha256>::new(private_key);
                            let sig = signing_key.sign(message.as_bytes());
                            use rsa::signature::SignatureEncoding;
                            Ok(Value::String(hex::encode(sig.to_bytes())))
                        }
                        _ => Err(RuntimeError::new(
                            "rsa_sign requires string arguments".to_string(),
                        )),
                    }
                }
            }
            "rsa_verify" => {
                #[cfg(not(feature = "crypto-advanced"))]
                return Err(RuntimeError::new(
                    "rsa_verify requires the 'crypto-advanced' feature. \
                     Rebuild with: cargo build --features crypto-advanced"
                        .to_string(),
                ));
                #[cfg(feature = "crypto-advanced")]
                {
                    if args.len() != 3 {
                        return Err(RuntimeError::new(
                            "rsa_verify requires 3 arguments (public_key_pem, message, signature_hex)"
                                .to_string(),
                        ));
                    }
                    match (&args[0], &args[1], &args[2]) {
                        (Value::String(pub_pem), Value::String(message), Value::String(sig_hex)) => {
                            use rsa::pkcs1::DecodeRsaPublicKey;
                            use rsa::pkcs1v15::{Signature, VerifyingKey};
                            use rsa::signature::Verifier;
                            use rsa::RsaPublicKey;
                            use sha2::Sha256;
                            let public_key =
                                RsaPublicKey::from_pkcs1_pem(pub_pem).map_err(|e| {
                                    RuntimeError::new(format!(
                                        "rsa_verify: invalid public key: {}",
                                        e
                                    ))
                                })?;
                            let verifying_key = VerifyingKey::<Sha256>::new(public_key);
                            let sig_bytes = hex::decode(sig_hex).map_err(|_| {
                                RuntimeError::new(
                                    "rsa_verify: signature_hex must be valid hex".to_string(),
                                )
                            })?;
                            let sig =
                                Signature::try_from(sig_bytes.as_slice()).map_err(|e| {
                                    RuntimeError::new(format!(
                                        "rsa_verify: invalid signature: {}",
                                        e
                                    ))
                                })?;
                            Ok(Value::Boolean(
                                verifying_key.verify(message.as_bytes(), &sig).is_ok(),
                            ))
                        }
                        _ => Err(RuntimeError::new(
                            "rsa_verify requires string arguments".to_string(),
                        )),
                    }
                }
            }
            _ => Err(RuntimeError::new(format!(
                "Unknown crypto function: {}",
                name
            ))),
        }
    }
}
