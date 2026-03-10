#!/usr/bin/env sh
# ─────────────────────────────────────────────────────────────────────────────
# Txt-code — LOCAL / DEV INSTALL
# Copies the locally-built release binary to a global directory.
# Run from the project root AFTER: cargo build --release
#
# Usage:
#   sh install.sh              # installs to /usr/local/bin (needs sudo on most systems)
#   INSTALL_DIR=~/.local/bin sh install.sh   # install to user dir (no sudo)
# ─────────────────────────────────────────────────────────────────────────────
set -e

INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
BIN_NAME="txtcode"
SRC="target/release/${BIN_NAME}"

# ── Colours ───────────────────────────────────────────────────────────────────
GREEN='\033[0;32m'; RED='\033[0;31m'; BOLD='\033[1m'; RESET='\033[0m'
ok()    { printf "${GREEN}ok${RESET}    %s\n" "$*"; }
error() { printf "${RED}error${RESET} %s\n" "$*" >&2; exit 1; }

printf "\n${BOLD}Txt-code — Dev Install${RESET}\n\n"

# ── Pre-flight ────────────────────────────────────────────────────────────────
[ -f "$SRC" ] || error "Binary not found at '${SRC}'. Run 'cargo build --release' first."

mkdir -p "$INSTALL_DIR" || error "Cannot create '${INSTALL_DIR}'. Try: INSTALL_DIR=~/.local/bin sh install.sh"

# ── Install ───────────────────────────────────────────────────────────────────
DEST="${INSTALL_DIR}/${BIN_NAME}"

# Remove first to avoid ETXTBSY if binary is currently running
rm -f "$DEST"
cp "$SRC" "$DEST"
chmod 755 "$DEST"

ok "Installed → ${DEST}"

# ── Verify ────────────────────────────────────────────────────────────────────
"$DEST" --version >/dev/null 2>&1 || error "Binary installed but failed to run."
ok "$("$DEST" --version)"

printf "\n${BOLD}Done.${RESET} Run: txtcode repl\n\n"
