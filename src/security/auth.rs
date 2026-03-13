// Script authentication — Ed25519 signing and verification.
//
// PURPOSE: Protect pentest scripts from tampering between author and execution.
// A signed script can be verified before running, ensuring the content has not
// been modified since it was signed by a trusted author.
//
// WORKFLOW:
//   1. Author generates a keypair:
//      let (private_pkcs8, public_key) = ScriptAuth::generate_keypair()?;
//      // Store private_pkcs8 securely; distribute public_key to users.
//
//   2. Author signs a script:
//      let sig = ScriptAuth::sign(source.as_bytes(), "author@example.com", &private_pkcs8)?;
//      let sig_file = sig.to_base64();
//      // Write sig_file to "script.tc.sig" alongside "script.tc".
//
//   3. User verifies before execution:
//      let sig = ScriptSignature::from_base64(&sig_file)?;
//      let ok = ScriptAuth::verify(source.as_bytes(), &sig)?;
//      assert!(ok, "Script signature invalid — file may have been tampered with!");
//
// CRYPTO: Ed25519 via ring crate (ring::signature::Ed25519KeyPair).
// STORAGE: Signatures serialized as base64-encoded JSON for .tc.sig sidecar files.

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use ring::rand::SystemRandom;
use ring::signature::{Ed25519KeyPair, KeyPair, UnparsedPublicKey, ED25519};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

// ── ScriptSignature ───────────────────────────────────────────────────────────

/// A signed attestation for a script file.
#[derive(Debug, Clone)]
pub struct ScriptSignature {
    /// Who signed this (free-form label, e.g. "author@example.com" or a key fingerprint).
    pub signer_id: String,
    /// Unix timestamp (seconds) when the signature was created.
    pub signed_at: u64,
    /// SHA-256 hash of the signed content (for quick fingerprinting without re-verifying).
    pub content_hash: Vec<u8>,
    /// Raw Ed25519 signature bytes over `content_hash || signer_id || signed_at`.
    pub signature: Vec<u8>,
    /// Public key bytes (DER-encoded SubjectPublicKeyInfo) for self-contained verification.
    pub public_key: Vec<u8>,
}

impl ScriptSignature {
    /// Hex fingerprint of the content hash (first 8 bytes). Quick human-readable ID.
    pub fn fingerprint(&self) -> String {
        hex::encode(&self.content_hash[..self.content_hash.len().min(8)])
    }

    /// Serialize to a compact base64 string suitable for .tc.sig sidecar files.
    pub fn to_base64(&self) -> String {
        // Format: VERSION|signer_id|signed_at|content_hash_hex|signature_hex|public_key_hex
        let payload = format!(
            "1|{}|{}|{}|{}|{}",
            Self::escape(&self.signer_id),
            self.signed_at,
            hex::encode(&self.content_hash),
            hex::encode(&self.signature),
            hex::encode(&self.public_key),
        );
        B64.encode(payload.as_bytes())
    }

    /// Deserialize from a base64 string produced by `to_base64()`.
    pub fn from_base64(encoded: &str) -> Result<Self, String> {
        let bytes = B64.decode(encoded)
            .map_err(|e| format!("Base64 decode failed: {}", e))?;
        let text = String::from_utf8(bytes)
            .map_err(|e| format!("UTF-8 decode failed: {}", e))?;

        let parts: Vec<&str> = text.splitn(6, '|').collect();
        if parts.len() != 6 {
            return Err(format!(
                "Invalid signature format: expected 6 fields, got {}",
                parts.len()
            ));
        }

        let version = parts[0];
        if version != "1" {
            return Err(format!("Unsupported signature version: {}", version));
        }

        let signer_id = Self::unescape(parts[1]);
        let signed_at = parts[2]
            .parse::<u64>()
            .map_err(|e| format!("Invalid timestamp: {}", e))?;
        let content_hash = hex::decode(parts[3])
            .map_err(|e| format!("Invalid content hash: {}", e))?;
        let signature = hex::decode(parts[4])
            .map_err(|e| format!("Invalid signature bytes: {}", e))?;
        let public_key = hex::decode(parts[5])
            .map_err(|e| format!("Invalid public key: {}", e))?;

        Ok(ScriptSignature { signer_id, signed_at, content_hash, signature, public_key })
    }

    fn escape(s: &str) -> String {
        s.replace('|', "\\|")
    }

    fn unescape(s: &str) -> String {
        s.replace("\\|", "|")
    }
}

// ── ScriptAuth ────────────────────────────────────────────────────────────────

/// Ed25519 script signing and verification.
pub struct ScriptAuth;

impl ScriptAuth {
    /// Generate a new Ed25519 keypair.
    ///
    /// Returns `(private_key_pkcs8, public_key_raw)`.
    ///   - `private_key_pkcs8`: Store securely; needed for `sign()`.
    ///   - `public_key_raw`: Distribute to users; needed for `verify()`.
    pub fn generate_keypair() -> Result<(Vec<u8>, Vec<u8>), String> {
        let rng = SystemRandom::new();
        let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng)
            .map_err(|_| "Ed25519 key generation failed".to_string())?;
        let key_pair = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref())
            .map_err(|e| format!("Key parse failed: {}", e))?;
        let public_key = key_pair.public_key().as_ref().to_vec();
        Ok((pkcs8.as_ref().to_vec(), public_key))
    }

    /// Sign `content` with an Ed25519 private key.
    ///
    /// `signer_id`: Identifies the signer (email, key fingerprint, etc.).
    /// `private_key_pkcs8`: PKCS#8-encoded private key from `generate_keypair()`.
    pub fn sign(
        content: &[u8],
        signer_id: &str,
        private_key_pkcs8: &[u8],
    ) -> Result<ScriptSignature, String> {
        let key_pair = Ed25519KeyPair::from_pkcs8(private_key_pkcs8)
            .map_err(|e| format!("Invalid private key: {}", e))?;

        let signed_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Compute SHA-256 of content.
        let mut hasher = Sha256::new();
        hasher.update(content);
        let content_hash = hasher.finalize().to_vec();

        // Build the signed payload: content_hash || signer_id || timestamp.
        // Including timestamp and signer_id prevents replay and identity confusion.
        let signed_payload = Self::build_payload(&content_hash, signer_id, signed_at);
        let signature = key_pair.sign(&signed_payload).as_ref().to_vec();
        let public_key = key_pair.public_key().as_ref().to_vec();

        Ok(ScriptSignature { signer_id: signer_id.to_string(), signed_at, content_hash, signature, public_key })
    }

    /// Verify that `content` matches the signature.
    ///
    /// Uses the public key embedded in the signature for self-contained verification.
    /// For a stricter workflow, compare `sig.public_key` against a trusted list before calling.
    pub fn verify(content: &[u8], sig: &ScriptSignature) -> Result<bool, String> {
        // Recompute content hash and confirm it matches what was signed.
        let mut hasher = Sha256::new();
        hasher.update(content);
        let computed_hash = hasher.finalize().to_vec();

        if computed_hash != sig.content_hash {
            return Ok(false); // Content hash mismatch — file was modified.
        }

        // Rebuild the signed payload.
        let signed_payload = Self::build_payload(&sig.content_hash, &sig.signer_id, sig.signed_at);

        // Verify the Ed25519 signature.
        let pk = UnparsedPublicKey::new(&ED25519, &sig.public_key);
        Ok(pk.verify(&signed_payload, &sig.signature).is_ok())
    }

    /// Verify using an explicitly trusted public key instead of the key in the signature.
    ///
    /// Use this when you maintain a list of trusted public keys and want to enforce
    /// that the signature was made by a specific known key.
    pub fn verify_with_key(
        content: &[u8],
        sig: &ScriptSignature,
        trusted_public_key: &[u8],
    ) -> Result<bool, String> {
        let mut hasher = Sha256::new();
        hasher.update(content);
        let computed_hash = hasher.finalize().to_vec();

        if computed_hash != sig.content_hash {
            return Ok(false);
        }

        // Also confirm the embedded public key matches the trusted key.
        if sig.public_key != trusted_public_key {
            return Ok(false); // Signed with a different key than expected.
        }

        let signed_payload = Self::build_payload(&sig.content_hash, &sig.signer_id, sig.signed_at);
        let pk = UnparsedPublicKey::new(&ED25519, trusted_public_key);
        Ok(pk.verify(&signed_payload, &sig.signature).is_ok())
    }

    /// Derive the public key fingerprint (first 8 bytes of SHA-256 of the raw key).
    pub fn key_fingerprint(public_key: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(public_key);
        let hash = hasher.finalize();
        hex::encode(&hash[..8])
    }

    fn build_payload(content_hash: &[u8], signer_id: &str, signed_at: u64) -> Vec<u8> {
        let mut payload = Vec::with_capacity(content_hash.len() + signer_id.len() + 8 + 2);
        payload.extend_from_slice(content_hash);
        payload.push(b'|');
        payload.extend_from_slice(signer_id.as_bytes());
        payload.push(b'|');
        payload.extend_from_slice(&signed_at.to_le_bytes());
        payload
    }
}

// ── KeyStore ─────────────────────────────────────────────────────────────────

/// A simple in-memory trusted public key store.
///
/// Load trusted keys from your policy config and use `is_trusted()` before
/// accepting a script signature.
pub struct KeyStore {
    /// Trusted public keys indexed by signer label.
    keys: std::collections::HashMap<String, Vec<u8>>,
}

impl KeyStore {
    pub fn new() -> Self {
        Self { keys: std::collections::HashMap::new() }
    }

    /// Add a trusted public key for a given signer label.
    pub fn add(&mut self, label: String, public_key: Vec<u8>) {
        self.keys.insert(label, public_key);
    }

    /// Load a hex-encoded public key.
    pub fn add_hex(&mut self, label: String, public_key_hex: &str) -> Result<(), String> {
        let key = hex::decode(public_key_hex)
            .map_err(|e| format!("Invalid hex key for '{}': {}", label, e))?;
        self.keys.insert(label, key);
        Ok(())
    }

    /// Check whether `sig.public_key` is in the trust store.
    pub fn is_trusted(&self, sig: &ScriptSignature) -> bool {
        self.keys.values().any(|k| k == &sig.public_key)
    }

    /// Verify a signature against the trust store — both signature and key must be valid.
    pub fn verify_trusted(&self, content: &[u8], sig: &ScriptSignature) -> Result<bool, String> {
        if !self.is_trusted(sig) {
            return Ok(false); // Signer not in trust store.
        }
        ScriptAuth::verify(content, sig)
    }
}

impl Default for KeyStore {
    fn default() -> Self {
        Self::new()
    }
}
