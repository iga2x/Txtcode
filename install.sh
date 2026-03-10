#!/usr/bin/env sh
# ─────────────────────────────────────────────────────────────────────────────
# Txt-code Installer
# Usage: curl -sSf https://raw.githubusercontent.com/iga2x/txtcode/main/install.sh | sh
# ─────────────────────────────────────────────────────────────────────────────
set -e

REPO="iga2x/txtcode"
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
    Linux*)   OS_NAME="linux" ;;
    Darwin*)  OS_NAME="macos" ;;
    MINGW*|MSYS*|CYGWIN*) OS_NAME="windows" ;;
    *)        error "Unsupported operating system: $OS" ;;
esac

case "$ARCH" in
    x86_64|amd64) ARCH_NAME="x86_64" ;;
    aarch64|arm64) ARCH_NAME="aarch64" ;;
    armv7*) ARCH_NAME="armv7" ;;
    *) warn "Unknown architecture: $ARCH — will attempt source build" ;;
esac

info "System: ${OS_NAME} / ${ARCH_NAME}"

# ── Install directory ─────────────────────────────────────────────────────────
mkdir -p "$INSTALL_DIR"

# ── Try to download pre-built binary first ────────────────────────────────────
download_prebuilt() {
    if [ "$VERSION" = "latest" ]; then
        RELEASE_URL="https://api.github.com/repos/${REPO}/releases/latest"
        if command -v curl >/dev/null 2>&1; then
            TAG=$(curl -sSf "$RELEASE_URL" 2>/dev/null | grep '"tag_name"' | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
        elif command -v wget >/dev/null 2>&1; then
            TAG=$(wget -qO- "$RELEASE_URL" 2>/dev/null | grep '"tag_name"' | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
        fi
        VERSION="${TAG:-}"
    fi

    if [ -z "$VERSION" ]; then
        return 1
    fi

    ASSET_NAME="${BIN_NAME}-${OS_NAME}-${ARCH_NAME}"
    if [ "$OS_NAME" = "windows" ]; then
        ASSET_NAME="${ASSET_NAME}.exe"
    fi

    DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${ASSET_NAME}"
    DEST="${INSTALL_DIR}/${BIN_NAME}"

    info "Downloading ${BIN_NAME} ${VERSION}..."
    if command -v curl >/dev/null 2>&1; then
        curl -sSfL "$DOWNLOAD_URL" -o "$DEST" 2>/dev/null || return 1
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$DEST" "$DOWNLOAD_URL" 2>/dev/null || return 1
    else
        return 1
    fi

    chmod +x "$DEST"
    return 0
}

# ── Build from source ─────────────────────────────────────────────────────────
build_from_source() {
    if ! command -v cargo >/dev/null 2>&1; then
        error "cargo not found. Install Rust first: https://rustup.rs\nThen re-run this installer."
    fi

    info "Building from source (this takes a minute)..."

    TMP_DIR="$(mktemp -d)"
    trap 'rm -rf "$TMP_DIR"' EXIT

    if command -v git >/dev/null 2>&1; then
        git clone --depth 1 "https://github.com/${REPO}.git" "$TMP_DIR/txtcode" >/dev/null 2>&1 \
            || error "Failed to clone repository"
        BUILD_DIR="$TMP_DIR/txtcode"
    else
        # Download tarball
        TARBALL_URL="https://github.com/${REPO}/archive/refs/heads/main.tar.gz"
        info "Downloading source tarball..."
        if command -v curl >/dev/null 2>&1; then
            curl -sSfL "$TARBALL_URL" | tar -xz -C "$TMP_DIR" || error "Failed to download source"
        else
            wget -qO- "$TARBALL_URL" | tar -xz -C "$TMP_DIR" || error "Failed to download source"
        fi
        BUILD_DIR="$TMP_DIR/txtcode-main"
    fi

    (cd "$BUILD_DIR" && cargo build --release --quiet) \
        || error "Build failed. Check the output above for details."

    cp "$BUILD_DIR/target/release/${BIN_NAME}" "${INSTALL_DIR}/${BIN_NAME}"
    chmod +x "${INSTALL_DIR}/${BIN_NAME}"
}

# ── Try pre-built, fall back to source ───────────────────────────────────────
if download_prebuilt; then
    success "Downloaded pre-built binary"
else
    warn "No pre-built binary found for ${OS_NAME}/${ARCH_NAME}"
    info  "Building from source instead..."
    build_from_source
    success "Built from source"
fi

# ── Verify the binary ─────────────────────────────────────────────────────────
if ! "${INSTALL_DIR}/${BIN_NAME}" --version >/dev/null 2>&1; then
    error "Installed binary failed to run. Please report this issue."
fi

INSTALLED_VERSION=$("${INSTALL_DIR}/${BIN_NAME}" --version 2>/dev/null || echo "unknown")
success "Installed ${INSTALLED_VERSION} to ${INSTALL_DIR}/${BIN_NAME}"

# ── Create global data directories ────────────────────────────────────────────
TXTCODE_HOME="$HOME/.txtcode"
mkdir -p "${TXTCODE_HOME}/cache" "${TXTCODE_HOME}/packages" "${TXTCODE_HOME}/logs"
success "Created ${TXTCODE_HOME}/"

# ── Add to PATH ───────────────────────────────────────────────────────────────
PATH_LINE="export PATH=\"\$HOME/.local/bin:\$PATH\" # txtcode"

add_to_shell() {
    FILE="$1"
    if [ -f "$FILE" ]; then
        if ! grep -q "txtcode" "$FILE" 2>/dev/null; then
            printf "\n%s\n" "$PATH_LINE" >> "$FILE"
            success "Added PATH entry to ~/${FILE##*/}"
        fi
    fi
}

case "$SHELL" in
    */zsh)   add_to_shell "$HOME/.zshrc"; add_to_shell "$HOME/.zprofile" ;;
    */bash)  add_to_shell "$HOME/.bashrc"; add_to_shell "$HOME/.bash_profile" ;;
    */fish)  mkdir -p "$HOME/.config/fish"
             echo "set -gx PATH \$HOME/.local/bin \$PATH # txtcode" >> "$HOME/.config/fish/config.fish"
             success "Added PATH entry to fish config" ;;
    *)       add_to_shell "$HOME/.profile" ;;
esac

# ── Done ──────────────────────────────────────────────────────────────────────
printf "\n${BOLD}${GREEN}Installation complete!${RESET}\n\n"
printf "  Restart your terminal or run:\n"
printf "    ${BOLD}source ~/.bashrc${RESET}   (bash)\n"
printf "    ${BOLD}source ~/.zshrc${RESET}    (zsh)\n\n"
printf "  Then try:\n"
printf "    ${BOLD}txtcode --version${RESET}\n"
printf "    ${BOLD}txtcode repl${RESET}\n\n"
printf "  To uninstall:\n"
printf "    ${BOLD}txtcode self uninstall${RESET}\n\n"
