#!/usr/bin/env sh
# ─────────────────────────────────────────────────────────────────────────────
# Txt-code Standalone Uninstaller
# Use this ONLY if the txtcode binary is already gone or broken.
# Normally use: txtcode self uninstall
# ─────────────────────────────────────────────────────────────────────────────
set -e

BIN_NAME="txtcode"

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BOLD='\033[1m'; RESET='\033[0m'

info()    { printf "${BOLD}info${RESET}  %s\n" "$*"; }
success() { printf "${GREEN}ok${RESET}    %s\n" "$*"; }
warn()    { printf "${YELLOW}warn${RESET}  %s\n" "$*"; }

printf "\n${BOLD}╔══════════════════════════════════════════╗${RESET}\n"
printf "${BOLD}║     Txt-code Uninstaller (standalone)    ║${RESET}\n"
printf "${BOLD}╚══════════════════════════════════════════╝${RESET}\n\n"
printf "${YELLOW}Tip: If txtcode is still working, prefer: txtcode self uninstall${RESET}\n\n"

# ── Resolve actual user home (works under sudo) ───────────────────────────────
if [ -n "$SUDO_USER" ]; then
    ACTUAL_HOME=$(getent passwd "$SUDO_USER" 2>/dev/null | cut -d: -f6)
    [ -z "$ACTUAL_HOME" ] && ACTUAL_HOME=$(eval echo "~$SUDO_USER")
else
    ACTUAL_HOME="$HOME"
fi

TXTCODE_HOME="${ACTUAL_HOME}/.txtcode"

# Search all standard locations including cargo
INSTALL_DIRS="/usr/local/bin /usr/bin ${ACTUAL_HOME}/.local/bin ${ACTUAL_HOME}/.cargo/bin"

# ── Find ALL installed binaries ───────────────────────────────────────────────
FOUND_BINS=""
for dir in $INSTALL_DIRS; do
    if [ -f "${dir}/${BIN_NAME}" ]; then
        FOUND_BINS="${FOUND_BINS}${dir}/${BIN_NAME} "
    fi
done
FOUND_BINS="${FOUND_BINS% }"  # trim trailing space

if [ -n "$FOUND_BINS" ]; then
    for b in $FOUND_BINS; do
        info "Found binary: $b"
    done
else
    warn "Binary not found in standard locations — may already be removed"
fi

# ── Mode selection ────────────────────────────────────────────────────────────
printf "\nWhat would you like to remove?\n\n"
if [ -n "$FOUND_BINS" ]; then
    BINS_DISPLAY=$(echo "$FOUND_BINS" | tr ' ' '\n' | sed 's/^/     /')
else
    BINS_DISPLAY="     <not found>"
fi

printf "  1) Binary only (safest)\n"
printf "     Removes :\n%s\n" "$BINS_DISPLAY"
printf "     Keeps   : ~/.txtcode/ and all project .txtcode-env/ dirs\n\n"
printf "  2) Binary + global data\n"
printf "     Removes :\n%s\n" "$BINS_DISPLAY"
printf "     Removes : %s\n" "$TXTCODE_HOME"
printf "     Keeps   : All project .txtcode-env/ dirs\n\n"
printf "  3) Complete wipe (everything)\n"
printf "     Removes : binaries + %s + all .txtcode-env/ dirs under home\n" "$TXTCODE_HOME"
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
    PROJECT_ENVS=$(find "$ACTUAL_HOME" -maxdepth 8 -name ".txtcode-env" -type d 2>/dev/null || true)
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
        warn "Could not remove $TXTCODE_HOME (try with sudo)"
    fi
fi

# Clean PATH entries from shell configs
for rc in "${ACTUAL_HOME}/.bashrc" "${ACTUAL_HOME}/.bash_profile" \
          "${ACTUAL_HOME}/.zshrc" "${ACTUAL_HOME}/.zprofile" "${ACTUAL_HOME}/.profile"; do
    if [ -f "$rc" ] && grep -q "txtcode" "$rc" 2>/dev/null; then
        TMP=$(mktemp)
        grep -v "txtcode" "$rc" > "$TMP" || true
        mv "$TMP" "$rc"
        success "Cleaned PATH entry from ${rc##*/}"
    fi
done

# Clean fish config
FISH_CFG="${ACTUAL_HOME}/.config/fish/config.fish"
if [ -f "$FISH_CFG" ] && grep -q "txtcode" "$FISH_CFG" 2>/dev/null; then
    TMP=$(mktemp)
    grep -v "txtcode" "$FISH_CFG" > "$TMP" || true
    mv "$TMP" "$FISH_CFG"
    success "Cleaned fish config"
fi

# Remove ALL found binaries
if [ -n "$FOUND_BINS" ]; then
    for b in $FOUND_BINS; do
        if [ -f "$b" ]; then
            if rm -f "$b" 2>/dev/null; then
                success "Removed binary: $b"
            else
                warn "Could not remove $b (try: sudo rm $b)"
            fi
        fi
    done
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
