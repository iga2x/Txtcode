#!/usr/bin/env bash
# scripts/sign_release.sh — Sign a release binary with an Ed25519 private key.
#
# Usage:
#   SIGNING_KEY_HEX=<64-char-hex> ./scripts/sign_release.sh <binary_path>
#
# The script produces:
#   <binary_path>.sig   — base64-encoded Ed25519 signature (ScriptSignature JSON)
#   sha256sums          — SHA-256 checksums file for all binaries in the same dir
#
# Requirements:
#   - openssl >= 3.0 (for Ed25519 support)
#   - SIGNING_KEY_HEX env var set to the 64-char hex private key seed
#
# In CI this is called from .github/workflows/release.yml with
# SIGNING_KEY_HEX set from a GitHub Actions secret.
set -euo pipefail

BINARY="${1:-}"
if [[ -z "$BINARY" || ! -f "$BINARY" ]]; then
    echo "Usage: $0 <binary_path>" >&2
    exit 1
fi

if [[ -z "${SIGNING_KEY_HEX:-}" ]]; then
    echo "Error: SIGNING_KEY_HEX environment variable not set." >&2
    echo "Set it to the 64-char hex private key seed before running." >&2
    exit 1
fi

BINARY_DIR="$(dirname "$BINARY")"
BINARY_NAME="$(basename "$BINARY")"
SIG_FILE="${BINARY}.sig"
SHA_FILE="${BINARY_DIR}/sha256sums"

# ── Write private key to a temp file ────────────────────────────────────────
TMPDIR_KEY="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_KEY"' EXIT

KEY_FILE="${TMPDIR_KEY}/signing.key"
printf '%s' "$SIGNING_KEY_HEX" | xxd -r -p > "${TMPDIR_KEY}/seed.bin"
openssl genpkey -algorithm ed25519 -out "$KEY_FILE" 2>/dev/null || {
    # Fall back: write raw seed directly (OpenSSL 3+)
    openssl pkey -outform PEM -out "$KEY_FILE" \
        -engine "" 2>/dev/null || true
}
# If we have the raw seed, use it via a DER wrapper
python3 - <<'PYEOF'
import os, sys, base64
seed_hex = os.environ.get('SIGNING_KEY_HEX', '')
if not seed_hex:
    sys.exit(0)
seed = bytes.fromhex(seed_hex)
# Ed25519 private key: RFC 8032 seed is the first 32 bytes
# Write PKCS#8 DER for OpenSSL
der_prefix = bytes.fromhex('302e020100300506032b657004220420')
pkcs8_der = der_prefix + seed
key_path = os.environ['KEY_FILE']
with open(key_path, 'wb') as f:
    f.write(pkcs8_der)
PYEOF
export KEY_FILE

PUB_FILE="${TMPDIR_KEY}/signing.pub"
openssl pkey -in "$KEY_FILE" -inform DER -pubout -out "$PUB_FILE" 2>/dev/null || \
    openssl pkey -in "$KEY_FILE" -pubout -out "$PUB_FILE"

# ── Sign the binary ──────────────────────────────────────────────────────────
RAW_SIG_FILE="${TMPDIR_KEY}/signature.bin"
openssl pkeyutl -sign -inkey "$KEY_FILE" -keyform DER \
    -in "$BINARY" -rawin -out "$RAW_SIG_FILE" 2>/dev/null || \
openssl pkeyutl -sign -inkey "$KEY_FILE" \
    -in "$BINARY" -rawin -out "$RAW_SIG_FILE"

SIG_B64="$(base64 < "$RAW_SIG_FILE" | tr -d '\n')"

# Extract public key bytes for the ScriptSignature JSON
PUB_B64="$(openssl pkey -in "$PUB_FILE" -pubin -pubout -outform DER 2>/dev/null | \
    tail -c 32 | base64 | tr -d '\n')"

# ── Write .sig file (ScriptSignature JSON format) ───────────────────────────
cat > "$SIG_FILE" <<JSON
{
  "version": "1",
  "algorithm": "ed25519",
  "public_key": "${PUB_B64}",
  "signature": "${SIG_B64}",
  "binary": "${BINARY_NAME}"
}
JSON
echo "Signature written to: ${SIG_FILE}"

# ── Write / append sha256sums ────────────────────────────────────────────────
SHA="$(sha256sum "$BINARY" | awk '{print $1}')"
echo "${SHA}  ${BINARY_NAME}" >> "$SHA_FILE"
echo "SHA-256: ${SHA}  (appended to ${SHA_FILE})"
