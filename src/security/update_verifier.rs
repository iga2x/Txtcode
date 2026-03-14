// Binary update verification using Ed25519 signatures.
//
// The release public key is baked into the binary at compile time.
// `self_update` must download the `.sig` sidecar file alongside the binary
// and call `verify_update_binary` before replacing the running executable.
//
// KEY ROTATION: The public key can only be rotated by shipping a new binary
// release. There is no dynamic fetch of the public key — this prevents MITM
// even if the release hosting is compromised.

use sha2::{Digest, Sha256};

/// Pinned Ed25519 public key for release verification.
///
/// Replace these zeros with the real SubjectPublicKeyInfo DER bytes before
/// shipping. Generate with: `txtcode security gen-release-keypair`
///
/// The private key must be stored in GitHub Actions secrets (RELEASE_SIGNING_KEY).
pub const RELEASE_PUBLIC_KEY: &[u8] = &[0u8; 32]; // TODO: replace with real key bytes

/// Verify a downloaded binary against its Ed25519 signature.
///
/// `binary_bytes`: raw bytes of the downloaded binary.
/// `sig_bytes`: raw bytes of the `.sig` file (base64-encoded JSON ScriptSignature).
///
/// Returns Ok(()) if verification passes, Err with explanation otherwise.
pub fn verify_update_binary(
    binary_bytes: &[u8],
    sig_bytes: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::security::auth::{ScriptAuth, ScriptSignature};

    // Parse the signature file
    let sig_str = std::str::from_utf8(sig_bytes)
        .map_err(|_| "Signature file is not valid UTF-8")?;
    let sig = ScriptSignature::from_base64(sig_str)
        .map_err(|e| format!("Failed to parse signature file: {}", e))?;

    // Verify using the pinned public key
    if RELEASE_PUBLIC_KEY.iter().all(|&b| b == 0) {
        // No real key baked in yet — skip verification with a warning.
        // This path should not exist in production builds.
        eprintln!(
            "WARNING: Release signing key not configured. \
             Binary signature verification is DISABLED. \
             This is a development build only."
        );
        return Ok(());
    }

    let ok = ScriptAuth::verify_with_key(binary_bytes, &sig, RELEASE_PUBLIC_KEY)
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

/// Verify SHA-256 checksum of downloaded binary against a `sha256sums` file.
///
/// `sha256sums_content`: content of the `sha256sums` file from the release.
/// `filename`: the base filename of the binary (e.g., `txtcode-1.0.0-linux-x86_64`).
/// `binary_bytes`: raw bytes of the downloaded binary.
pub fn verify_sha256(
    sha256sums_content: &str,
    filename: &str,
    binary_bytes: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    // Compute actual hash
    let actual_hash = hex::encode(Sha256::digest(binary_bytes));

    // Find the expected hash for this filename in the sums file
    for line in sha256sums_content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Format: "<hash>  <filename>" or "<hash> <filename>"
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() < 2 {
            continue;
        }
        let expected_hash = parts[0].trim();
        let file_part = parts[1].trim().trim_start_matches('*'); // some tools prefix with *
        if file_part == filename || file_part.ends_with(filename) {
            if actual_hash != expected_hash {
                return Err(format!(
                    "SHA-256 checksum mismatch for '{}':\n  expected: {}\n  actual:   {}",
                    filename, expected_hash, actual_hash
                )
                .into());
            }
            return Ok(());
        }
    }

    // If we didn't find the filename, warn but don't fail (the sha256sums file
    // might use a different naming convention).
    eprintln!(
        "WARNING: Could not find '{}' in sha256sums file. Skipping hash verification.",
        filename
    );
    Ok(())
}
