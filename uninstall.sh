#!/usr/bin/env sh
# ─────────────────────────────────────────────────────────────────────────────
# Txt-code Standalone Uninstaller
# Use this ONLY if the txtcode binary is already gone or broken.
# Normally use: txtcode self uninstall
# ─────────────────────────────────────────────────────────────────────────────
set -e

BIN_NAME="txtcode"
INSTALL_DIRS="$HOME/.local/bin /usr/local/bin /usr/bin"
TXTCODE_HOME="$HOME/.txtcode"

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BOLD='\033[1m'; RESET='\033[0m'

info()    { printf "${BOLD}info${RESET}  %s\n" "$*"; }
success() { printf "${GREEN}ok${RESET}    %s\n" "$*"; }
warn()    { printf "${YELLOW}warn${RESET}  %s\n" "$*"; }

printf "\n${BOLD}╔══════════════════════════════════════════╗${RESET}\n"
printf "${BOLD}║     Txt-code Uninstaller (standalone)    ║${RESET}\n"
printf "${BOLD}╚══════════════════════════════════════════╝${RESET}\n\n"
printf "${YELLOW}Tip: If txtcode is still working, prefer: txtcode self uninstall${RESET}\n\n"

# ── Find binary ───────────────────────────────────────────────────────────────
FOUND_BIN=""
for dir in $INSTALL_DIRS; do
    if [ -f "${dir}/${BIN_NAME}" ]; then
        FOUND_BIN="${dir}/${BIN_NAME}"
        break
    fi
done

if [ -n "$FOUND_BIN" ]; then
    info "Found binary: $FOUND_BIN"
else
    warn "Binary not found in standard locations — may already be removed"
fi

# ── Mode selection ────────────────────────────────────────────────────────────
printf "What would you like to remove?\n\n"
printf "  1) Binary only (safest)\n"
printf "     Removes : %s\n" "${FOUND_BIN:-<not found>}"
printf "     Keeps   : ~/.txtcode/ and all project .txtcode-env/ dirs\n\n"
printf "  2) Binary + global data\n"
printf "     Removes : %s\n" "${FOUND_BIN:-<not found>}"
printf "     Removes : %s\n" "$TXTCODE_HOME"
printf "     Keeps   : All project .txtcode-env/ dirs\n\n"
printf "  3) Complete wipe (everything)\n"
printf "     Removes : binary + %s + all .txtcode-env/ dirs under home\n" "$TXTCODE_HOME"
printf "     WARNING : This cannot be undone\n\n"

printf "Enter choice [1/2/3] (or q to quit): "
read -r CHOICE

case "$CHOICE" in
    1|2|3) ;;
    q|Q) printf "Uninstall cancelled.\n"; exit 0 ;;
    *)   printf "Invalid choice. Exiting.\n"; exit 1 ;;
esac

# ── For mode 3, show project envs ─────────────────────────────────────────────
PROJECT_ENVS=""
if [ "$CHOICE" = "3" ]; then
    printf "\nSearching for .txtcode-env/ directories...\n"
    PROJECT_ENVS=$(find "$HOME" -maxdepth 8 -name ".txtcode-env" -type d 2>/dev/null || true)
    if [ -n "$PROJECT_ENVS" ]; then
        printf "Found:\n"
        echo "$PROJECT_ENVS" | while read -r env; do printf "  - %s\n" "$env"; done
    else
        printf "  (none found)\n"
    fi
fi

# ── Confirmation ──────────────────────────────────────────────────────────────
if [ "$CHOICE" = "3" ]; then
    printf "\nType 'DELETE ALL' to confirm complete wipe: "
    read -r CONFIRM
    if [ "$CONFIRM" != "DELETE ALL" ]; then
        printf "Uninstall cancelled.\n"
        exit 0
    fi
else
    printf "\nProceed? [y/N]: "
    read -r CONFIRM
    case "$CONFIRM" in
        y|Y) ;;
        *) printf "Uninstall cancelled.\n"; exit 0 ;;
    esac
fi

# ── Execute ───────────────────────────────────────────────────────────────────
printf "\nUninstalling...\n"

# Remove project envs (mode 3)
if [ "$CHOICE" = "3" ] && [ -n "$PROJECT_ENVS" ]; then
    echo "$PROJECT_ENVS" | while read -r env; do
        if rm -rf "$env" 2>/dev/null; then
            success "Removed $env"
        else
            warn "Could not remove $env"
        fi
    done
fi

# Remove global data (mode 2 or 3)
if [ "$CHOICE" != "1" ] && [ -d "$TXTCODE_HOME" ]; then
    if rm -rf "$TXTCODE_HOME"; then
        success "Removed $TXTCODE_HOME"
    else
        warn "Could not remove $TXTCODE_HOME"
    fi
fi

# Clean PATH entries from shell configs
for rc in "$HOME/.bashrc" "$HOME/.bash_profile" "$HOME/.zshrc" "$HOME/.zprofile" "$HOME/.profile"; do
    if [ -f "$rc" ] && grep -q "txtcode" "$rc" 2>/dev/null; then
        TMP=$(mktemp)
        grep -v "txtcode" "$rc" > "$TMP" || true
        mv "$TMP" "$rc"
        success "Cleaned PATH entry from ${rc##*/}"
    fi
done

# Remove binary
if [ -n "$FOUND_BIN" ] && [ -f "$FOUND_BIN" ]; then
    if rm -f "$FOUND_BIN"; then
        success "Removed binary: $FOUND_BIN"
    else
        warn "Could not remove $FOUND_BIN (try: sudo rm $FOUND_BIN)"
    fi
fi

# Clean fish config
FISH_CFG="$HOME/.config/fish/config.fish"
if [ -f "$FISH_CFG" ] && grep -q "txtcode" "$FISH_CFG" 2>/dev/null; then
    TMP=$(mktemp)
    grep -v "txtcode" "$FISH_CFG" > "$TMP" || true
    mv "$TMP" "$FISH_CFG"
    success "Cleaned fish config"
fi

printf "\n${GREEN}${BOLD}Txt-code has been uninstalled.${RESET}\n"
if [ "$CHOICE" = "1" ]; then
    printf "Your data in ~/.txtcode/ and all project dirs are untouched.\n"
elif [ "$CHOICE" = "2" ]; then
    printf "Your project .txtcode-env/ directories are untouched.\n"
else
    printf "All Txt-code data has been removed.\n"
fi
printf "\nTo reinstall: curl -sSf https://raw.githubusercontent.com/iga2x/txtcode/main/install.sh | sh\n\n"
