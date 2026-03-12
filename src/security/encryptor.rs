use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

/// Bytecode encryptor
/// Encrypts bytecode instructions and provides decryption at runtime
pub struct BytecodeEncryptor {
    key: Key<Aes256Gcm>,
}

impl BytecodeEncryptor {
    /// Create a new encryptor with a generated key
    pub fn new() -> Self {
        let key = Aes256Gcm::generate_key(&mut OsRng);
        Self { key }
    }

    /// Create an encryptor with a specific key
    pub fn with_key(key: &[u8]) -> Result<Self, String> {
        if key.len() != 32 {
            return Err("Key must be 32 bytes for AES-256".to_string());
        }
        let key = *Key::<Aes256Gcm>::from_slice(key);
        Ok(Self { key })
    }

    /// Derive key from runtime environment
    pub fn from_runtime() -> Self {
        // Derive key from system properties
        let mut hasher = Sha256::new();

        // Use system time as part of key derivation
        let timestamp = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => duration.as_secs(),
            Err(e) => {
                // Fallback: use a default timestamp if system time fails
                // This is a rare error (system clock set backwards), but we need to handle it
                eprintln!(
                    "Warning: System time error in key derivation: {}. Using fallback timestamp.",
                    e
                );
                // Use a fixed timestamp as fallback (Jan 1, 2020)
                1577836800
            }
        };
        hasher.update(timestamp.to_le_bytes());

        // Use environment variables if available
        if let Ok(hostname) = std::env::var("HOSTNAME") {
            hasher.update(hostname.as_bytes());
        }

        // Use process ID
        hasher.update(std::process::id().to_le_bytes());

        let key_bytes = hasher.finalize();
        let key = *Key::<Aes256Gcm>::from_slice(&key_bytes);

        Self { key }
    }

    /// Encrypt bytecode data
    pub fn encrypt(&self, data: &[u8]) -> Result<EncryptedBytecode, String> {
        let cipher = Aes256Gcm::new(&self.key);
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        let ciphertext = cipher
            .encrypt(&nonce, data)
            .map_err(|e| format!("Encryption failed: {}", e))?;

        Ok(EncryptedBytecode {
            ciphertext,
            nonce: nonce.to_vec(),
            version: 1,
        })
    }

    /// Decrypt bytecode data
    pub fn decrypt(&self, encrypted: &EncryptedBytecode) -> Result<Vec<u8>, String> {
        let cipher = Aes256Gcm::new(&self.key);
        let nonce = Nonce::from_slice(&encrypted.nonce);

        let plaintext = cipher
            .decrypt(nonce, encrypted.ciphertext.as_ref())
            .map_err(|e| format!("Decryption failed: {}", e))?;

        Ok(plaintext)
    }

    /// Get the encryption key (for key management)
    pub fn get_key(&self) -> &Key<Aes256Gcm> {
        &self.key
    }
}

impl Default for BytecodeEncryptor {
    fn default() -> Self {
        Self::new()
    }
}

/// Encrypted bytecode structure
#[derive(Debug, Clone)]
pub struct EncryptedBytecode {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
    pub version: u8,
}

impl EncryptedBytecode {
    /// Serialize encrypted bytecode
    pub fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();
        result.push(self.version);
        result.push(self.nonce.len() as u8);
        result.extend_from_slice(&self.nonce);
        result.extend_from_slice(&(self.ciphertext.len() as u32).to_le_bytes());
        result.extend_from_slice(&self.ciphertext);
        result
    }

    /// Deserialize encrypted bytecode
    pub fn deserialize(data: &[u8]) -> Result<Self, String> {
        if data.len() < 2 {
            return Err("Invalid encrypted bytecode format".to_string());
        }

        let version = data[0];
        let nonce_len = data[1] as usize;

        if data.len() < 2 + nonce_len + 4 {
            return Err("Invalid encrypted bytecode format".to_string());
        }

        let nonce = data[2..2 + nonce_len].to_vec();

        let ciphertext_len = u32::from_le_bytes([
            data[2 + nonce_len],
            data[2 + nonce_len + 1],
            data[2 + nonce_len + 2],
            data[2 + nonce_len + 3],
        ]) as usize;

        if data.len() < 2 + nonce_len + 4 + ciphertext_len {
            return Err("Invalid encrypted bytecode format".to_string());
        }

        let ciphertext = data[2 + nonce_len + 4..2 + nonce_len + 4 + ciphertext_len].to_vec();

        Ok(EncryptedBytecode {
            ciphertext,
            nonce,
            version,
        })
    }
}
