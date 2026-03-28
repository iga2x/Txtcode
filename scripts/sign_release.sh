#!/usr/bin/env bash
# Sign a Txtcode release binary with Ed25519 using the PKCS8 v2 private key.
#
# Usage:
#   sign_release.sh <binary_path>
#
# Environment:
#   SIGNING_KEY_HEX — PKCS8 v2 Ed25519 private key as a lowercase hex string.
#                     Stored as a GitHub Actions secret (never committed).
#
# Output:
#   Writes <binary_path>.sig next to the binary.
#   Exits 0 in all cases (skip gracefully when key is absent).

set -euo pipefail

if [[ $# -lt 1 ]]; then
    echo "Usage: sign_release.sh <binary_path>" >&2
    exit 1
fi

BINARY="$1"

if [[ ! -f "$BINARY" ]]; then
    echo "sign_release.sh: binary not found: $BINARY" >&2
    exit 1
fi

if [[ -z "${SIGNING_KEY_HEX:-}" ]]; then
    echo "sign_release.sh: SIGNING_KEY_HEX not set — skipping signing"
    exit 0
fi

SIG_FILE="${BINARY}.sig"

# Decode hex → raw DER bytes → temp file (cleaned up on exit)
KEY_TMP=$(mktemp)
trap 'rm -f "$KEY_TMP"' EXIT

printf '%s' "$SIGNING_KEY_HEX" | xxd -r -p > "$KEY_TMP"

# Sign with Ed25519 (raw message, no pre-hashing)
openssl pkeyutl -sign \
    -inkey "$KEY_TMP" \
    -keyform DER \
    -rawin \
    -in "$BINARY" \
    -out "$SIG_FILE"

echo "sign_release.sh: signed $BINARY → $SIG_FILE"
