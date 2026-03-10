#!/usr/bin/env sh
# ─────────────────────────────────────────────────────────────────────────────
# Txt-code Installer  (case-sensitive repo: iga2x/Txtcode)
# Usage: curl -sSf https://raw.githubusercontent.com/iga2x/Txtcode/main/install.sh | sh
# ─────────────────────────────────────────────────────────────────────────────
set -e

REPO="iga2x/Txtcode"
BIN_NAME="txtcode"
INSTALL_DIR="${TXTCODE_INSTALL_DIR:-$HOME/.local/bin}"
VERSION="${TXTCODE_VERSION:-latest}"

# ── Colours ──────────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BLUE='\033[0;34m'; BOLD='\033[1m'; RESET='\033[0m'

info()    { printf "${BLUE}info${RESET}  %s\n" "$*"; }
success() { printf "${GREEN}ok${RESET}    %s\n" "$*"; }
warn()    { printf "${YELLOW}warn${RESET}  %s\n" "$*"; }
error()   { printf "${RED}error${RESET} %s\n" "$*" >&2; exit 1; }

# ── Banner ───────────────────────────────────────────────────────────────────
printf "\n${BOLD}╔══════════════════════════════════════════╗${RESET}\n"
printf "${BOLD}║     Txt-code Installer                   ║${RESET}\n"
printf "${BOLD}╚══════════════════════════════════════════╝${RESET}\n\n"

# ── Detect OS and architecture ───────────────────────────────────────────────
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux*)              OS_NAME="linux" ;;
    Darwin*)             OS_NAME="macos" ;;
    MINGW*|MSYS*|CYGWIN*) OS_NAME="windows" ;;
    *)                   error "Unsupported operating system: $OS" ;;
esac

case "$ARCH" in
    x86_64|amd64)  ARCH_NAME="x86_64" ;;
    aarch64|arm64) ARCH_NAME="arm64" ;;
    *)             ARCH_NAME="" ;;
esac

info "System: ${OS_NAME} / ${ARCH_NAME:-unknown}"

# ── Install directory ─────────────────────────────────────────────────────────
mkdir -p "$INSTALL_DIR"

# ── Helper: download a URL to a file ─────────────────────────────────────────
download() {
    URL="$1"; DEST="$2"
    if command -v curl >/dev/null 2>&1; then
        curl -sSfL "$URL" -o "$DEST"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$DEST" "$URL"
    else
        error "Neither curl nor wget found. Please install one and retry."
    fi
}

# ── Safe binary install (handles "text file busy" on running binary) ──────────
install_binary() {
    SRC="$1"; DEST="$2"
    # Remove existing binary first — avoids ETXTBSY on Linux
    rm -f "$DEST"
    cp "$SRC" "$DEST"
    chmod 755 "$DEST"
}

# ── Resolve latest tag from GitHub API ───────────────────────────────────────
resolve_version() {
    API="https://api.github.com/repos/${REPO}/releases/latest"
    if command -v curl >/dev/null 2>&1; then
        TAG=$(curl -sSf "$API" 2>/dev/null | grep '"tag_name"' | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
    else
        TAG=$(wget -qO- "$API" 2>/dev/null | grep '"tag_name"' | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
    fi
    printf '%s' "$TAG"
}

# ── Download pre-built package ────────────────────────────────────────────────
download_prebuilt() {
    if [ -z "$ARCH_NAME" ]; then
        warn "Architecture '$ARCH' not supported by pre-built packages."
        return 1
    fi

    if [ "$VERSION" = "latest" ]; then
        info "Resolving latest release..."
        VERSION="$(resolve_version)"
        if [ -z "$VERSION" ]; then
            warn "Could not resolve latest version from GitHub API."
            return 1
        fi
    fi

    info "Found release: ${VERSION}"

    # Package name mirrors release.yml artifact names
    if [ "$OS_NAME" = "windows" ]; then
        PKG="${BIN_NAME}-${OS_NAME}-${ARCH_NAME}.zip"
    else
        PKG="${BIN_NAME}-${OS_NAME}-${ARCH_NAME}.tar.gz"
    fi

    DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${PKG}"
    TMP_DIR="$(mktemp -d)"
    TMP_PKG="${TMP_DIR}/${PKG}"

    info "Downloading ${PKG} from ${VERSION}..."
    download "$DOWNLOAD_URL" "$TMP_PKG" || {
        warn "Pre-built package not found: ${DOWNLOAD_URL}"
        rm -rf "$TMP_DIR"
        return 1
    }

    # Extract
    if [ "$OS_NAME" = "windows" ]; then
        command -v unzip >/dev/null 2>&1 || error "unzip not found. Install it and retry."
        unzip -q "$TMP_PKG" -d "$TMP_DIR"
        install_binary "${TMP_DIR}/${BIN_NAME}.exe" "${INSTALL_DIR}/${BIN_NAME}.exe"
    else
        tar -xzf "$TMP_PKG" -C "$TMP_DIR"
        install_binary "${TMP_DIR}/${BIN_NAME}" "${INSTALL_DIR}/${BIN_NAME}"
    fi

    rm -rf "$TMP_DIR"
    return 0
}

# ── Build from source (fallback) ──────────────────────────────────────────────
build_from_source() {
    command -v cargo >/dev/null 2>&1 || \
        error "cargo not found. Install Rust from https://rustup.rs then retry."

    info "Building from source (this takes a minute)..."
    TMP_DIR="$(mktemp -d)"

    if command -v git >/dev/null 2>&1; then
        git clone --depth 1 "https://github.com/${REPO}.git" "$TMP_DIR/src" >/dev/null 2>&1 \
            || error "Failed to clone repository."
        BUILD_DIR="$TMP_DIR/src"
    else
        TARBALL="https://github.com/${REPO}/archive/refs/heads/main.tar.gz"
        info "Downloading source tarball..."
        download "$TARBALL" "$TMP_DIR/src.tar.gz"
        tar -xzf "$TMP_DIR/src.tar.gz" -C "$TMP_DIR"
        BUILD_DIR="$(ls -d "$TMP_DIR"/Txtcode-* 2>/dev/null | head -1)"
        [ -n "$BUILD_DIR" ] || error "Failed to locate extracted source."
    fi

    (cd "$BUILD_DIR" && cargo build --release --quiet) \
        || error "Build failed."

    install_binary "${BUILD_DIR}/target/release/${BIN_NAME}" "${INSTALL_DIR}/${BIN_NAME}"
    rm -rf "$TMP_DIR"
}

# ── Main: try pre-built, fall back to source ──────────────────────────────────
if download_prebuilt; then
    success "Installed pre-built binary"
else
    warn "No pre-built binary available for ${OS_NAME}/${ARCH_NAME:-unknown}."
    info  "Falling back to build from source..."
    build_from_source
    success "Built and installed from source"
fi

# ── Verify ────────────────────────────────────────────────────────────────────
DEST="${INSTALL_DIR}/${BIN_NAME}"
"$DEST" --version >/dev/null 2>&1 || error "Installed binary failed to run."
INSTALLED_VERSION=$("$DEST" --version 2>/dev/null || echo "unknown")
success "Installed ${INSTALLED_VERSION} → ${DEST}"

# ── Create data directories ───────────────────────────────────────────────────
TXTCODE_HOME="${TXTCODE_HOME:-$HOME/.txtcode}"
mkdir -p "${TXTCODE_HOME}/cache" "${TXTCODE_HOME}/packages" "${TXTCODE_HOME}/logs"
success "Data dir: ${TXTCODE_HOME}/"

# ── Add to PATH ───────────────────────────────────────────────────────────────
PATH_LINE="export PATH=\"\$HOME/.local/bin:\$PATH\" # txtcode"

add_to_shell() {
    FILE="$1"
    if [ -f "$FILE" ] && ! grep -q "# txtcode" "$FILE" 2>/dev/null; then
        printf "\n%s\n" "$PATH_LINE" >> "$FILE"
        success "Added PATH entry to ${FILE##*/}"
    fi
}

case "$SHELL" in
    */zsh)  add_to_shell "$HOME/.zshrc"; add_to_shell "$HOME/.zprofile" ;;
    */bash) add_to_shell "$HOME/.bashrc"; add_to_shell "$HOME/.bash_profile" ;;
    */fish) mkdir -p "$HOME/.config/fish"
            grep -q "# txtcode" "$HOME/.config/fish/config.fish" 2>/dev/null || \
            printf "\nset -gx PATH \$HOME/.local/bin \$PATH # txtcode\n" \
                >> "$HOME/.config/fish/config.fish"
            success "Added PATH entry to fish/config.fish" ;;
    *)      add_to_shell "$HOME/.profile" ;;
esac

# ── Done ──────────────────────────────────────────────────────────────────────
printf "\n${BOLD}${GREEN}Installation complete!${RESET}\n\n"
printf "  Reload your shell or run:\n"
printf "    ${BOLD}source ~/.zshrc${RESET}    (zsh)\n"
printf "    ${BOLD}source ~/.bashrc${RESET}   (bash)\n\n"
printf "  Then try:\n"
printf "    ${BOLD}txtcode --version${RESET}\n"
printf "    ${BOLD}txtcode repl${RESET}\n\n"
printf "  To uninstall:\n"
printf "    ${BOLD}txtcode self uninstall${RESET}\n\n"
