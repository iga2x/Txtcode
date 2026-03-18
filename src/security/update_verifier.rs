// Binary update verification using Ed25519 signatures.
//
// The release public key is baked into the binary at compile time.
// `self_update` must download the `.sig` sidecar file alongside the binary
// and call `verify_update_binary` before replacing the running executable.
//
// KEY ROTATION: The public key can only be rotated by shipping a new binary
// release. There is no dynamic fetch of the public key — this prevents MITM
// even if the release hosting is compromised.
//
// SIGNING WORKFLOW (CI):
//   1. Maintainer generates keypair once:
//        cargo run --bin gen_release_key
//      Store the PKCS8 hex as GitHub Actions secret SIGNING_KEY_HEX.
//      Embed the public key bytes below as RELEASE_PUBLIC_KEY.
//   2. scripts/sign_release.sh uses SIGNING_KEY_HEX to produce a .sig sidecar.
//   3. self_update downloads both binary + .sig and calls verify_update_binary.

use ring::signature::{UnparsedPublicKey, ED25519};
use sha2::{Digest, Sha256};

/// Pinned Ed25519 release public key (32 raw bytes).
///
/// Generated with: `cargo run --bin gen_release_key`
/// Fingerprint: f3a621cf6ad61b8f (first 8 bytes of SHA-256 of the key)
///
/// Corresponding private key is stored in GitHub Actions secret SIGNING_KEY_HEX
/// (PKCS8 v2 hex).  It is never committed to the repository.
pub const RELEASE_PUBLIC_KEY: &[u8; 32] = &[
    0xec, 0xad, 0x1e, 0x10, 0x1e, 0xa6, 0x9c, 0x60,
    0x6a, 0x50, 0xf4, 0xf5, 0xf3, 0x9e, 0x04, 0x5d,
    0x6e, 0x28, 0x82, 0xe4, 0xaa, 0x64, 0x57, 0xf0,
    0x8c, 0xc7, 0x09, 0x01, 0x5d, 0x7c, 0x28, 0x70,
];

// ── Public API ────────────────────────────────────────────────────────────────

/// Verify a downloaded binary against its Ed25519 signature using the pinned
/// release public key baked into this binary.
///
/// `binary_bytes`: raw bytes of the downloaded binary.
/// `sig_bytes`:    raw bytes of the `.sig` sidecar file (base64-encoded
///                 `ScriptSignature` produced by `scripts/sign_release.sh`).
///
/// Returns `Ok(())` if the signature is valid, `Err` with explanation otherwise.
/// Never silently skips verification — missing or malformed keys are hard errors.
pub fn verify_update_binary(
    binary_bytes: &[u8],
    sig_bytes: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    verify_update_binary_with_key(binary_bytes, sig_bytes, RELEASE_PUBLIC_KEY)
}

/// Same as `verify_update_binary` but accepts an explicit trusted public key.
///
/// Useful for testing and for supporting custom/private registries with their
/// own signing keys.
pub fn verify_update_binary_with_key(
    binary_bytes: &[u8],
    sig_bytes: &[u8],
    trusted_public_key: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::security::auth::{ScriptAuth, ScriptSignature};

    // Parse the ScriptSignature from the .sig file.
    let sig_str = std::str::from_utf8(sig_bytes)
        .map_err(|_| "Signature file is not valid UTF-8")?;
    let sig = ScriptSignature::from_base64(sig_str.trim())
        .map_err(|e| format!("Failed to parse signature file: {}", e))?;

    // Verify the Ed25519 signature against the trusted pinned key.
    let ok = ScriptAuth::verify_with_key(binary_bytes, &sig, trusted_public_key)
        .map_err(|e| format!("Signature verification error: {}", e))?;

    if !ok {
        return Err(
            "Binary signature verification FAILED. \
             The downloaded binary may have been tampered with. \
             Aborting update."
                .into(),
        );
    }

    Ok(())
}

// ── SHA-256 checksum verification ─────────────────────────────────────────────

/// Verify the SHA-256 checksum of a downloaded binary against a `sha256sums` file.
///
/// `sha256sums_content`: content of the `sha256sums` file from the release.
/// `filename`: base filename of the binary (e.g., `txtcode-1.0.0-linux-x86_64`).
/// `binary_bytes`: raw bytes of the downloaded binary.
///
/// Returns `Ok(())` if the hash matches.  Returns `Err` on mismatch.
/// If the filename is not found in the sums file, the function returns `Err`
/// (not a silent warning) to avoid silent bypass of checksum verification.
pub fn verify_sha256(
    sha256sums_content: &str,
    filename: &str,
    binary_bytes: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let actual_hash = hex::encode(Sha256::digest(binary_bytes));

    for line in sha256sums_content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Standard format: "<hash>  <filename>" (two spaces) or "<hash> <filename>".
        // Some tools prefix the filename with '*' (binary mode flag).
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() < 2 {
            continue;
        }
        let expected_hash = parts[0].trim();
        let file_part = parts[1].trim().trim_start_matches('*');

        if file_part == filename || file_part.ends_with(filename) {
            if actual_hash == expected_hash {
                return Ok(());
            } else {
                return Err(format!(
                    "SHA-256 checksum mismatch for '{}':\n  expected: {}\n  actual:   {}",
                    filename, expected_hash, actual_hash
                )
                .into());
            }
        }
    }

    // Filename not found — treat as an error, not a warning.
    // A legitimate release always includes the filename in sha256sums.
    Err(format!(
        "Filename '{}' not found in sha256sums file. \
         Cannot verify integrity — aborting.",
        filename
    )
    .into())
}

// ── Fingerprint helper ────────────────────────────────────────────────────────

/// Return the short hex fingerprint of the pinned release public key
/// (first 8 bytes of SHA-256).
pub fn release_key_fingerprint() -> String {
    crate::security::auth::ScriptAuth::key_fingerprint(RELEASE_PUBLIC_KEY)
}

// ═════════════════════════════════════════════════════════════════════════════
// Tests
// ═════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::auth::ScriptAuth;

    // ── helpers ───────────────────────────────────────────────────────────────

    /// Generate a fresh test keypair and return (pkcs8_bytes, pub_key_bytes).
    fn make_test_keypair() -> (Vec<u8>, Vec<u8>) {
        ScriptAuth::generate_keypair().expect("keygen failed")
    }

    /// Sign `content` with `pkcs8` and serialise the ScriptSignature to its
    /// wire format (base64 string), ready to pass to `verify_update_binary_with_key`.
    fn sign_as_sig_file(content: &[u8], pkcs8: &[u8]) -> Vec<u8> {
        let sig = ScriptAuth::sign(content, "test-release@txtcode", pkcs8)
            .expect("sign failed");
        sig.to_base64().into_bytes()
    }

    // ── core verification ─────────────────────────────────────────────────────

    #[test]
    fn test_valid_signature_accepted() {
        let (pkcs8, pub_key) = make_test_keypair();
        let binary = b"fake binary content v1.0.0";

        let sig_file = sign_as_sig_file(binary, &pkcs8);

        let result = verify_update_binary_with_key(binary, &sig_file, &pub_key);
        assert!(result.is_ok(), "Valid signature should be accepted: {:?}", result);
    }

    #[test]
    fn test_tampered_binary_rejected() {
        let (pkcs8, pub_key) = make_test_keypair();
        let binary = b"legitimate binary bytes";
        let sig_file = sign_as_sig_file(binary, &pkcs8);

        // Simulate MITM replacing the binary after signing.
        let tampered = b"malicious binary bytes!!";

        let result = verify_update_binary_with_key(tampered, &sig_file, &pub_key);
        assert!(
            result.is_err(),
            "Tampered binary must be rejected"
        );
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("FAILED") || msg.contains("mismatch") || msg.contains("false"),
            "Error message should indicate failure, got: {msg}"
        );
    }

    #[test]
    fn test_wrong_key_rejected() {
        let (pkcs8, _pub_key) = make_test_keypair();
        let (_, wrong_pub_key) = make_test_keypair(); // Different keypair.
        let binary = b"binary signed by keypair A";

        let sig_file = sign_as_sig_file(binary, &pkcs8);

        // Verifying with a different public key must fail.
        let result = verify_update_binary_with_key(binary, &sig_file, &wrong_pub_key);
        assert!(
            result.is_err(),
            "Signature from wrong key should be rejected"
        );
    }

    #[test]
    fn test_corrupted_sig_file_rejected() {
        let binary = b"some binary";

        let result = verify_update_binary_with_key(binary, b"not-a-real-sig", &[0u8; 32]);
        assert!(result.is_err(), "Corrupted sig file should be rejected");
    }

    #[test]
    fn test_empty_sig_file_rejected() {
        let binary = b"some binary";
        let result = verify_update_binary_with_key(binary, b"", &[0u8; 32]);
        assert!(result.is_err(), "Empty sig file should be rejected");
    }

    // ── direct ring verification round-trip ───────────────────────────────────

    #[test]
    fn test_ring_ed25519_round_trip() {
        // Low-level test: signs with ring, verifies with ring.
        // Demonstrates correct API usage independent of ScriptAuth.
        use ring::rand::SystemRandom;
        use ring::signature::{Ed25519KeyPair, KeyPair};

        let rng = SystemRandom::new();
        let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng).expect("keygen failed");
        let key_pair = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).expect("parse failed");
        let pub_key_bytes = key_pair.public_key().as_ref();

        let message = b"test binary payload for ring round-trip";
        let sig = key_pair.sign(message);

        let verifier = UnparsedPublicKey::new(&ED25519, pub_key_bytes);
        assert!(
            verifier.verify(message, sig.as_ref()).is_ok(),
            "ring Ed25519 round-trip must succeed"
        );

        // Mutated message must fail.
        let mut mutated = message.to_vec();
        mutated[0] ^= 0xff;
        assert!(
            verifier.verify(&mutated, sig.as_ref()).is_err(),
            "Mutated message must not verify"
        );
    }

    // ── SHA-256 verification ──────────────────────────────────────────────────

    #[test]
    fn test_sha256_correct_hash_accepted() {
        use sha2::{Digest, Sha256};

        let binary = b"release binary payload";
        let hash = hex::encode(Sha256::digest(binary));
        let sums = format!("{}  txtcode-1.0.0-linux-x86_64\n", hash);

        let result = verify_sha256(&sums, "txtcode-1.0.0-linux-x86_64", binary);
        assert!(result.is_ok(), "Correct hash should be accepted: {:?}", result);
    }

    #[test]
    fn test_sha256_wrong_hash_rejected() {
        let binary = b"release binary payload";
        let sums = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef  txtcode-1.0.0-linux-x86_64\n";

        let result = verify_sha256(sums, "txtcode-1.0.0-linux-x86_64", binary);
        assert!(result.is_err(), "Wrong hash must be rejected");
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("mismatch"), "Error should say mismatch, got: {msg}");
    }

    #[test]
    fn test_sha256_filename_not_found_is_error() {
        let binary = b"some content";
        let sums = "abc123  other-file.bin\n";

        let result = verify_sha256(sums, "txtcode-1.0.0-linux-x86_64", binary);
        assert!(
            result.is_err(),
            "Missing filename must be an error, not silently ignored"
        );
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("not found"), "Error should mention not found, got: {msg}");
    }

    #[test]
    fn test_sha256_star_prefix_filename_accepted() {
        use sha2::{Digest, Sha256};

        let binary = b"windows binary";
        let hash = hex::encode(Sha256::digest(binary));
        // Some tools emit: "<hash> *<filename>"
        let sums = format!("{} *txtcode-1.0.0-windows-x86_64.exe\n", hash);

        let result = verify_sha256(&sums, "txtcode-1.0.0-windows-x86_64.exe", binary);
        assert!(result.is_ok(), "Star-prefixed filename should be handled: {:?}", result);
    }

    // ── RELEASE_PUBLIC_KEY sanity ─────────────────────────────────────────────

    #[test]
    fn test_release_key_is_not_zeros() {
        assert!(
            !RELEASE_PUBLIC_KEY.iter().all(|&b| b == 0),
            "RELEASE_PUBLIC_KEY must not be the zero placeholder"
        );
    }

    #[test]
    fn test_release_key_length() {
        assert_eq!(
            RELEASE_PUBLIC_KEY.len(),
            32,
            "Ed25519 raw public key must be exactly 32 bytes"
        );
    }

    #[test]
    fn test_release_key_fingerprint_stable() {
        // Fingerprint of the pinned key — guards against accidental key changes.
        let fp = release_key_fingerprint();
        assert_eq!(
            fp, "f3a621cf6ad61b8f",
            "Pinned key fingerprint changed — was RELEASE_PUBLIC_KEY rotated intentionally?"
        );
    }
}
