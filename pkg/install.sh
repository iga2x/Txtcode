#!/usr/bin/env sh
# ─────────────────────────────────────────────────────────────────────────────
# Txt-code — Package Installer
# Bundled inside the release tar.gz. No internet connection required.
#
# Usage (from the extracted directory):
#   sh install.sh                              # → ~/.local/bin  (no sudo)
#   sudo sh install.sh                         # → /usr/local/bin
#   TXTCODE_INSTALL_DIR=/opt/bin sh install.sh # → custom dir
# ─────────────────────────────────────────────────────────────────────────────
set -e

BIN="txtcode"
GREEN='\033[0;32m'; YELLOW='\033[1;33m'; BOLD='\033[1m'; RESET='\033[0m'

ok()   { printf "${GREEN}ok${RESET}   %s\n" "$*"; }
info() { printf "${BOLD}info${RESET} %s\n" "$*"; }
warn() { printf "${YELLOW}warn${RESET} %s\n" "$*"; }

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SRC="${SCRIPT_DIR}/${BIN}"

[ -f "$SRC" ] || {
    echo "error: '${BIN}' binary not found next to install.sh"
    echo "Make sure you extracted the full package before running this script."
    exit 1
}

# ── Choose install directory ──────────────────────────────────────────────────
if [ -n "$TXTCODE_INSTALL_DIR" ]; then
    INSTALL_DIR="$TXTCODE_INSTALL_DIR"
elif [ "$(id -u)" = "0" ]; then
    INSTALL_DIR="/usr/local/bin"
else
    INSTALL_DIR="${HOME}/.local/bin"
fi

info "Installing to ${INSTALL_DIR}..."

mkdir -p "$INSTALL_DIR" || {
    echo "error: cannot create '${INSTALL_DIR}'"
    echo "Try: sudo sh install.sh   or   TXTCODE_INSTALL_DIR=~/bin sh install.sh"
    exit 1
}

DEST="${INSTALL_DIR}/${BIN}"
rm -f "$DEST"
cp "$SRC" "$DEST"
chmod 755 "$DEST"

ok "Installed → ${DEST}"
ok "$("$DEST" --version)"

# ── PATH hint if install dir is not already in PATH ───────────────────────────
case ":${PATH}:" in
    *":${INSTALL_DIR}:"*) ;;
    *)
        printf "\n${YELLOW}Note:${RESET} ${INSTALL_DIR} is not in your PATH.\n"
        printf "Add this line to your ~/.bashrc or ~/.zshrc:\n\n"
        printf "  export PATH=\"%s:\$PATH\"\n\n" "$INSTALL_DIR"
        ;;
esac

printf "\n${BOLD}Done.${RESET} Run: txtcode repl\n\n"
