#!/usr/bin/env sh
# ─────────────────────────────────────────────────────────────────────────────
# Txt-code — Dev Install (build from source → global dir)
# Usage:
#   sh install.sh                        # → /usr/local/bin  (may need sudo)
#   INSTALL_DIR=~/.local/bin sh install.sh   # → user dir, no sudo needed
# ─────────────────────────────────────────────────────────────────────────────
set -e

INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
BIN_NAME="txtcode"

GREEN='\033[0;32m'; RED='\033[0;31m'; BOLD='\033[1m'; RESET='\033[0m'
ok()    { printf "${GREEN}ok${RESET}  %s\n" "$*"; }
err()   { printf "${RED}error${RESET}  %s\n" "$*" >&2; exit 1; }

printf "\n${BOLD}Txt-code — Dev Install${RESET}\n\n"

# check cargo
command -v cargo >/dev/null 2>&1 || err "cargo not found. Install Rust: https://rustup.rs"

# must be run from project root
[ -f "Cargo.toml" ] || err "Run this from the project root (where Cargo.toml is)."

ok "Building release binary..."
cargo build --release --quiet

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
