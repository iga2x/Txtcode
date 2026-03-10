# ─────────────────────────────────────────────────────────────────────────────
# Txt-code Makefile  —  for developers building from source
# Usage:
#   make install      Build release binary and install to /usr/local/bin
#   make install-user Install to ~/.local/bin  (no sudo)
#   make uninstall    Remove system-wide binary
#   make uninstall-user Remove user-local binary
#   make clean        Remove build artifacts
#   make test         Run the full test suite
#   make build        Build debug binary
#   make release      Build optimised release binary
# ─────────────────────────────────────────────────────────────────────────────

BIN         := txtcode
SYSTEM_DIR  := /usr/local/bin
USER_DIR    := $(HOME)/.local/bin
CARGO       := cargo

.PHONY: all build release install install-user uninstall uninstall-user \
        clean test lint fmt check help

all: release

## ── Build ───────────────────────────────────────────────────────────────────

build:
	$(CARGO) build

release:
	$(CARGO) build --release

## ── Install / Uninstall  (system-wide, needs sudo) ──────────────────────────

install: release
	@echo "Installing $(BIN) to $(SYSTEM_DIR) ..."
	@install -Dm755 target/release/$(BIN) $(SYSTEM_DIR)/$(BIN)
	@mkdir -p $(HOME)/.txtcode/cache $(HOME)/.txtcode/packages $(HOME)/.txtcode/logs
	@echo "Done. Run 'txtcode --version' to verify."

uninstall:
	@echo "Removing $(SYSTEM_DIR)/$(BIN) ..."
	@rm -f $(SYSTEM_DIR)/$(BIN)
	@echo "Binary removed."
	@echo "To also remove config/cache run:  rm -rf ~/.txtcode"
	@echo "To clean project envs run:        find ~ -name .txtcode-env -type d -exec rm -rf {} +"

## ── Install / Uninstall  (user-local, no sudo) ──────────────────────────────

install-user: release
	@echo "Installing $(BIN) to $(USER_DIR) ..."
	@mkdir -p $(USER_DIR)
	@cp target/release/$(BIN) $(USER_DIR)/$(BIN)
	@chmod +x $(USER_DIR)/$(BIN)
	@mkdir -p $(HOME)/.txtcode/cache $(HOME)/.txtcode/packages $(HOME)/.txtcode/logs
	@echo ""
	@echo "Done! Make sure $(USER_DIR) is in your PATH:"
	@echo "  echo 'export PATH=\"\$$HOME/.local/bin:\$$PATH\"' >> ~/.bashrc && source ~/.bashrc"
	@echo ""
	@echo "Or run the full installer which handles PATH automatically:"
	@echo "  ./install.sh"

uninstall-user:
	@echo "Removing $(USER_DIR)/$(BIN) ..."
	@rm -f $(USER_DIR)/$(BIN)
	@echo "Binary removed."
	@echo "To also remove config/cache run:  rm -rf ~/.txtcode"

## ── Development ─────────────────────────────────────────────────────────────

test:
	$(CARGO) test

lint:
	$(CARGO) clippy -- -D warnings

fmt:
	$(CARGO) fmt

check:
	$(CARGO) check

clean:
	$(CARGO) clean

## ── Help ────────────────────────────────────────────────────────────────────

help:
	@echo ""
	@echo "Txt-code Makefile targets:"
	@echo ""
	@echo "  make install        Build + install to /usr/local/bin  (needs sudo)"
	@echo "  make install-user   Build + install to ~/.local/bin    (no sudo)"
	@echo "  make uninstall      Remove from /usr/local/bin"
	@echo "  make uninstall-user Remove from ~/.local/bin"
	@echo "  make build          Debug build"
	@echo "  make release        Optimised release build"
	@echo "  make test           Run tests"
	@echo "  make lint           Run clippy"
	@echo "  make fmt            Format source with rustfmt"
	@echo "  make clean          Remove build artefacts"
	@echo ""
	@echo "For end-user installation (no source required):"
	@echo "  curl -sSf https://raw.githubusercontent.com/iga2x/txtcode/main/install.sh | sh"
	@echo ""
