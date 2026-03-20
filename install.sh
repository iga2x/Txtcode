#!/usr/bin/env sh
# ─────────────────────────────────────────────────────────────────────────────
# Txt-code — Installer
# Tries to download a pre-built binary; falls back to `cargo build` if none.
#
# Usage:
#   sh install.sh                            # → /usr/local/bin  (may need sudo)
#   INSTALL_DIR=~/.local/bin sh install.sh   # → user dir, no sudo needed
# ─────────────────────────────────────────────────────────────────────────────
set -e

INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
BIN_NAME="txtcode"
REPO="iga2x/txtcode"
RELEASE_BASE="https://github.com/${REPO}/releases/latest/download"

GREEN='\033[0;32m'; YELLOW='\033[0;33m'; RED='\033[0;31m'; BOLD='\033[1m'; RESET='\033[0m'
ok()   { printf "${GREEN}ok${RESET}  %s\n" "$*"; }
warn() { printf "${YELLOW}warn${RESET}  %s\n" "$*"; }
err()  { printf "${RED}error${RESET}  %s\n" "$*" >&2; exit 1; }

printf "\n${BOLD}Txt-code — Installer${RESET}\n\n"

# ── Detect OS and architecture ────────────────────────────────────────────────
detect_platform() {
    OS="$(uname -s 2>/dev/null || echo unknown)"
    ARCH="$(uname -m 2>/dev/null || echo unknown)"

    case "$OS" in
        Linux*)
            case "$ARCH" in
                x86_64)  PLATFORM="linux-x86_64" ;;
                aarch64) PLATFORM="linux-arm64"  ;;
                *)       PLATFORM="" ;;
            esac
            ;;
        Darwin*)
            case "$ARCH" in
                x86_64)        PLATFORM="macos-x86_64" ;;
                arm64|aarch64) PLATFORM="macos-arm64"  ;;
                *)             PLATFORM="" ;;
            esac
            ;;
        MINGW*|MSYS*|CYGWIN*)
            PLATFORM="windows-x86_64"
            BIN_NAME="txtcode.exe"
            ;;
        *)
            PLATFORM=""
            ;;
    esac
}

# ── Try to install from pre-built binary ──────────────────────────────────────
try_binary_install() {
    detect_platform

    if [ -z "$PLATFORM" ]; then
        warn "No pre-built binary for this platform. Falling back to cargo build."
        return 1
    fi

    ASSET="txtcode-${PLATFORM}"
    [ "$BIN_NAME" = "txtcode.exe" ] && ASSET="${ASSET}.exe"

    BINARY_URL="${RELEASE_BASE}/${ASSET}"
    CHECKSUM_URL="${RELEASE_BASE}/${ASSET}.sha256"

    if command -v curl >/dev/null 2>&1; then
        DOWNLOAD_TO="curl -sSfL -o"
        HEAD_CHECK="curl -sSo /dev/null -w %{http_code} -L --head"
    elif command -v wget >/dev/null 2>&1; then
        DOWNLOAD_TO="wget -qO"
        HEAD_CHECK="wget -q --spider --server-response"
    else
        warn "Neither curl nor wget found. Falling back to cargo build."
        return 1
    fi

    # Test URL exists before downloading
    if command -v curl >/dev/null 2>&1; then
        HTTP_STATUS=$(curl -sSo /dev/null -w "%{http_code}" -L --head "$BINARY_URL" 2>/dev/null || echo 000)
    else
        HTTP_STATUS=$(wget -q --spider --server-response "$BINARY_URL" 2>&1 | awk '/HTTP\//{s=$2} END{print s}' || echo 000)
    fi

    if [ "$HTTP_STATUS" != "200" ]; then
        warn "Pre-built binary not found for ${PLATFORM} (HTTP ${HTTP_STATUS}). Falling back to cargo build."
        return 1
    fi

    ok "Downloading pre-built binary for ${PLATFORM}..."
    TMP_BIN="$(mktemp /tmp/txtcode_XXXXXX)"
    TMP_SUM="$(mktemp /tmp/txtcode_sha_XXXXXX)"

    $DOWNLOAD_TO "$TMP_BIN" "$BINARY_URL"  || { warn "Download failed. Falling back to cargo build."; rm -f "$TMP_BIN" "$TMP_SUM"; return 1; }
    $DOWNLOAD_TO "$TMP_SUM" "$CHECKSUM_URL" 2>/dev/null || true

    # Verify SHA-256 if checksum file was downloaded
    if [ -s "$TMP_SUM" ]; then
        EXPECTED=$(awk '{print $1}' "$TMP_SUM")
        if command -v sha256sum >/dev/null 2>&1; then
            ACTUAL=$(sha256sum "$TMP_BIN" | awk '{print $1}')
        elif command -v shasum >/dev/null 2>&1; then
            ACTUAL=$(shasum -a 256 "$TMP_BIN" | awk '{print $1}')
        else
            ACTUAL=""
        fi

        if [ -n "$ACTUAL" ] && [ "$EXPECTED" != "$ACTUAL" ]; then
            rm -f "$TMP_BIN" "$TMP_SUM"
            err "SHA-256 checksum mismatch! Binary may be corrupted or tampered with."
        fi
        [ -n "$ACTUAL" ] && ok "Checksum verified."
    fi

    chmod 755 "$TMP_BIN"
    mkdir -p "$INSTALL_DIR" || err "Cannot create '${INSTALL_DIR}'. Try: INSTALL_DIR=~/.local/bin sh install.sh"
    DEST="${INSTALL_DIR}/${BIN_NAME}"
    rm -f "$DEST"
    mv "$TMP_BIN" "$DEST"
    rm -f "$TMP_SUM"

    ok "Installed → ${DEST}"
    ok "$("$DEST" --version)"
    printf "\n${BOLD}Done.${RESET} Run: txtcode repl\n\n"
    return 0
}

# ── Fallback: build from source ───────────────────────────────────────────────
build_from_source() {
    [ -f "Cargo.toml" ] || err "No pre-built binary for your platform and Cargo.toml not found. Install Rust: https://rustup.rs"

    ok "Building release binary from source (requires Rust)..."
    if [ -n "$SUDO_USER" ]; then
        su - "$SUDO_USER" -c "cd '$PWD' && cargo build --release --quiet" \
            || err "Build failed."
    else
        cargo build --release --quiet || err "cargo not found. Install Rust: https://rustup.rs"
    fi

    SRC="target/release/${BIN_NAME}"
    [ -f "$SRC" ] || err "Build succeeded but binary not found at ${SRC}"

    mkdir -p "$INSTALL_DIR" || err "Cannot create '${INSTALL_DIR}'. Try: INSTALL_DIR=~/.local/bin sh install.sh"
    DEST="${INSTALL_DIR}/${BIN_NAME}"
    rm -f "$DEST"
    cp "$SRC" "$DEST"
    chmod 755 "$DEST"

    ok "Installed → ${DEST}"
    ok "$("$DEST" --version)"
    printf "\n${BOLD}Done.${RESET} Run: txtcode repl\n\n"
}

# ── Main ──────────────────────────────────────────────────────────────────────
if try_binary_install; then
    exit 0
fi

build_from_source
