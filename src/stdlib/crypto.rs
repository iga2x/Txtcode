use crate::runtime::{RuntimeError, Value};
use std::sync::Arc;
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
                        Ok(Value::String(Arc::from(hex::encode(hash))))
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
                        Ok(Value::String(Arc::from(hex::encode(hash))))
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
                        Ok(Value::String(Arc::from(hex::encode(bytes))))
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
                        Ok(Value::String(Arc::from(hex::encode(encrypted))))
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
                        let data = hex::decode(data_hex.as_ref())
                            .map_err(|_| RuntimeError::new("Invalid hex string".to_string()))?;
                        let key_bytes = key.as_bytes();
                        let mut decrypted = Vec::new();
                        for (i, byte) in data.iter().enumerate() {
                            decrypted.push(byte ^ key_bytes[i % key_bytes.len()]);
                        }
                        Ok(Value::String(Arc::from(String::from_utf8(decrypted).map_err(
                            |_| RuntimeError::new("Invalid UTF-8 in decrypted data".to_string()),
                        )?)))
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
                        Ok(Value::String(Arc::from(hex::encode(hash.0))))
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
                    Value::String(s) => Ok(Value::String(Arc::from(
                        general_purpose::STANDARD.encode(s.as_bytes()),
                    ))),
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
                            .decode(s.as_ref())
                            .map_err(|e| RuntimeError::new(format!("Invalid base64: {}", e)))?;
                        Ok(Value::String(Arc::from(String::from_utf8(decoded).map_err(|_| {
                            RuntimeError::new("Invalid UTF-8 in decoded data".to_string())
                        })?)))
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
                        Ok(Value::String(Arc::from(hex::encode(tag.as_ref()))))
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
                Ok(Value::String(Arc::from(uuid)))
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
                        Ok(Value::String(Arc::from(hex::encode(derived))))
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
                        let seed = hex::decode(seed_hex.as_ref()).map_err(|_| {
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
                        Ok(Value::String(Arc::from(hex::encode(sig.as_ref()))))
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
                        let pubkey_bytes = hex::decode(pubkey_hex.as_ref()).map_err(|_| {
                            RuntimeError::new(
                                "ed25519_verify: pubkey_hex must be valid hex".to_string(),
                            )
                        })?;
                        let sig_bytes = hex::decode(sig_hex.as_ref()).map_err(|_| {
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
                        Value::String(Arc::from(priv_pem.to_string())),
                    );
                    map.insert("public_key".to_string(), Value::String(Arc::from(pub_pem.to_string())));
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
                            Ok(Value::String(Arc::from(hex::encode(sig.to_bytes()))))
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
            // ── Task 16.3: Named crypto aliases & AES-256-GCM ────────────────

            // crypto_sha256(data) — explicit-namespace alias for sha256
            "crypto_sha256" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("crypto_sha256 requires 1 argument".to_string()));
                }
                match &args[0] {
                    Value::String(s) => {
                        let mut hasher = Sha256::new();
                        hasher.update(s.as_bytes());
                        Ok(Value::String(Arc::from(hex::encode(hasher.finalize()))))
                    }
                    _ => Err(RuntimeError::new("crypto_sha256 requires a string".to_string())),
                }
            }

            // crypto_hmac_sha256(key, data) — explicit-namespace alias for hmac_sha256
            "crypto_hmac_sha256" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "crypto_hmac_sha256 requires 2 arguments (key, data)".to_string(),
                    ));
                }
                match (&args[0], &args[1]) {
                    (Value::String(key), Value::String(data)) => {
                        use ring::hmac;
                        let hmac_key = hmac::Key::new(hmac::HMAC_SHA256, key.as_bytes());
                        let tag = hmac::sign(&hmac_key, data.as_bytes());
                        Ok(Value::String(Arc::from(hex::encode(tag.as_ref()))))
                    }
                    _ => Err(RuntimeError::new(
                        "crypto_hmac_sha256 requires string arguments".to_string(),
                    )),
                }
            }

            // crypto_aes_encrypt(key, plaintext) → base64-encoded ciphertext (nonce prepended)
            // key: 32-byte hex string (64 hex chars) or 32-char UTF-8 passphrase
            "crypto_aes_encrypt" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "crypto_aes_encrypt requires 2 arguments (key, plaintext)".to_string(),
                    ));
                }
                match (&args[0], &args[1]) {
                    (Value::String(key_str), Value::String(plaintext)) => {
                        use aes_gcm::{
                            aead::{Aead, AeadCore, KeyInit, OsRng},
                            Aes256Gcm, Key,
                        };
                        // Derive 32-byte key: try hex-decode first, otherwise SHA-256 the string.
                        let key_bytes = Self::derive_aes_key(key_str);
                        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
                        let cipher = Aes256Gcm::new(key);
                        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
                        let ciphertext = cipher
                            .encrypt(&nonce, plaintext.as_bytes())
                            .map_err(|e| RuntimeError::new(format!("AES encrypt failed: {}", e)))?;
                        // Encode as nonce (12 bytes) || ciphertext, base64-encoded.
                        let mut combined = nonce.to_vec();
                        combined.extend_from_slice(&ciphertext);
                        Ok(Value::String(Arc::from(general_purpose::STANDARD.encode(&combined))))
                    }
                    _ => Err(RuntimeError::new(
                        "crypto_aes_encrypt requires string arguments".to_string(),
                    )),
                }
            }

            // crypto_aes_decrypt(key, ciphertext_b64) → plaintext string
            "crypto_aes_decrypt" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "crypto_aes_decrypt requires 2 arguments (key, ciphertext)".to_string(),
                    ));
                }
                match (&args[0], &args[1]) {
                    (Value::String(key_str), Value::String(ciphertext_b64)) => {
                        use aes_gcm::{
                            aead::{Aead, KeyInit},
                            Aes256Gcm, Key, Nonce,
                        };
                        let key_bytes = Self::derive_aes_key(key_str);
                        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
                        let cipher = Aes256Gcm::new(key);
                        let combined = general_purpose::STANDARD
                            .decode(ciphertext_b64.as_bytes())
                            .map_err(|e| RuntimeError::new(format!("Base64 decode failed: {}", e)))?;
                        if combined.len() < 12 {
                            return Err(RuntimeError::new(
                                "crypto_aes_decrypt: ciphertext too short".to_string(),
                            ));
                        }
                        let nonce = Nonce::from_slice(&combined[..12]);
                        let plaintext = cipher
                            .decrypt(nonce, &combined[12..])
                            .map_err(|e| RuntimeError::new(format!("AES decrypt failed: {}", e)))?;
                        Ok(Value::String(Arc::from(
                            String::from_utf8(plaintext).map_err(|_| {
                                RuntimeError::new("Decrypted data is not valid UTF-8".to_string())
                            })?,
                        )))
                    }
                    _ => Err(RuntimeError::new(
                        "crypto_aes_decrypt requires string arguments".to_string(),
                    )),
                }
            }

            // ── Task 16.4: JWT helpers ────────────────────────────────────────

            // jwt_sign(payload_map, secret, algorithm) → token string
            // algorithm: "HS256" (default) or "RS256"
            "jwt_sign" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(RuntimeError::new(
                        "jwt_sign requires 2-3 arguments (payload, secret, algorithm?)".to_string(),
                    ));
                }
                let algorithm: String = if args.len() == 3 {
                    match &args[2] {
                        Value::String(s) => s.to_string(),
                        _ => return Err(RuntimeError::new("jwt_sign: algorithm must be a string".to_string())),
                    }
                } else {
                    "HS256".to_string()
                };
                let secret: String = match &args[1] {
                    Value::String(s) => s.to_string(),
                    _ => return Err(RuntimeError::new("jwt_sign: secret must be a string".to_string())),
                };
                let payload_json = Self::value_to_json(&args[0]);
                Self::jwt_sign_impl(&payload_json, &secret, &algorithm)
            }

            // jwt_verify(token, secret) → ok(payload_map) | err(reason)
            "jwt_verify" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "jwt_verify requires 2 arguments (token, secret)".to_string(),
                    ));
                }
                match (&args[0], &args[1]) {
                    (Value::String(token), Value::String(secret)) => {
                        Self::jwt_verify_impl(token, secret)
                    }
                    _ => Err(RuntimeError::new("jwt_verify requires string arguments".to_string())),
                }
            }

            // jwt_decode(token) → payload map (no verification — inspection only)
            "jwt_decode" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("jwt_decode requires 1 argument (token)".to_string()));
                }
                match &args[0] {
                    Value::String(token) => Self::jwt_decode_impl(token),
                    _ => Err(RuntimeError::new("jwt_decode requires a string token".to_string())),
                }
            }

            _ => Err(RuntimeError::new(format!(
                "Unknown crypto function: {}",
                name
            ))),
        }
    }

    // ── JWT implementation helpers ────────────────────────────────────────────

    fn jwt_sign_impl(payload_json: &serde_json::Value, secret: &str, algorithm: &str) -> Result<Value, RuntimeError> {
        use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
        let alg = match algorithm.to_uppercase().as_str() {
            "HS256" => Algorithm::HS256,
            "HS384" => Algorithm::HS384,
            "HS512" => Algorithm::HS512,
            other => return Err(RuntimeError::new(format!("jwt_sign: unsupported algorithm '{}'", other))),
        };
        let header = Header::new(alg);
        let key = EncodingKey::from_secret(secret.as_bytes());
        let token = encode(&header, payload_json, &key)
            .map_err(|e| RuntimeError::new(format!("jwt_sign: {}", e)))?;
        Ok(Value::String(Arc::from(token)))
    }

    fn jwt_verify_impl(token: &str, secret: &str) -> Result<Value, RuntimeError> {
        use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
        use std::collections::HashSet;
        let key = DecodingKey::from_secret(secret.as_bytes());
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = false;
        validation.required_spec_claims = HashSet::new(); // no required claims
        match decode::<serde_json::Value>(token, &key, &validation) {
            Ok(token_data) => {
                let payload = Self::json_to_value(&token_data.claims);
                Ok(Value::Result(true, Box::new(payload)))
            }
            Err(e) => Ok(Value::Result(
                false,
                Box::new(Value::String(Arc::from(e.to_string()))),
            )),
        }
    }

    #[allow(deprecated)]
    fn jwt_decode_impl(token: &str) -> Result<Value, RuntimeError> {
        use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
        use std::collections::HashSet;
        let key = DecodingKey::from_secret(b"");
        let mut validation = Validation::new(Algorithm::HS256);
        validation.insecure_disable_signature_validation();
        validation.validate_exp = false;
        validation.required_spec_claims = HashSet::new();
        match decode::<serde_json::Value>(token, &key, &validation) {
            Ok(token_data) => Ok(Self::json_to_value(&token_data.claims)),
            Err(e) => Err(RuntimeError::new(format!("jwt_decode: {}", e))),
        }
    }

    /// Convert a `Value` to a `serde_json::Value` for JWT payload encoding.
    fn value_to_json(value: &Value) -> serde_json::Value {
        match value {
            Value::Null => serde_json::Value::Null,
            Value::Boolean(b) => serde_json::Value::Bool(*b),
            Value::Integer(n) => serde_json::Value::Number((*n).into()),
            Value::Float(f) => serde_json::json!(*f),
            Value::String(s) => serde_json::Value::String(s.to_string()),
            Value::Array(arr) => serde_json::Value::Array(arr.iter().map(Self::value_to_json).collect()),
            Value::Map(m) => {
                let obj: serde_json::Map<String, serde_json::Value> =
                    m.iter().map(|(k, v)| (k.clone(), Self::value_to_json(v))).collect();
                serde_json::Value::Object(obj)
            }
            other => serde_json::Value::String(other.to_string()),
        }
    }

    /// Convert a `serde_json::Value` back to a `Value`.
    fn json_to_value(json: &serde_json::Value) -> Value {
        match json {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Boolean(*b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Integer(i)
                } else {
                    Value::Float(n.as_f64().unwrap_or(0.0))
                }
            }
            serde_json::Value::String(s) => Value::String(Arc::from(s.clone())),
            serde_json::Value::Array(arr) => Value::Array(arr.iter().map(Self::json_to_value).collect()),
            serde_json::Value::Object(obj) => {
                let mut map = indexmap::IndexMap::new();
                for (k, v) in obj {
                    map.insert(k.clone(), Self::json_to_value(v));
                }
                Value::Map(map)
            }
        }
    }

    /// Derive a 32-byte AES key from a string.
    /// If the string is 64 hex characters, decode it directly.
    /// Otherwise, SHA-256 hash the string to produce 32 bytes.
    fn derive_aes_key(key_str: &str) -> [u8; 32] {
        if key_str.len() == 64 {
            if let Ok(bytes) = hex::decode(key_str) {
                if bytes.len() == 32 {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    return arr;
                }
            }
        }
        let mut hasher = Sha256::new();
        hasher.update(key_str.as_bytes());
        let result = hasher.finalize();
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&result);
        arr
    }
}
