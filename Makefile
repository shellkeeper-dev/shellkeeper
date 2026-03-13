# ─────────────────────────────────────────────────────────────────────────────
# shellkeeper — Makefile
# ─────────────────────────────────────────────────────────────────────────────

.PHONY: setup run release build check clean install deb help

BINARY   := shellkeeper
CARGO    := cargo
PREFIX   ?= /usr/local/bin

# Detect OS
UNAME := $(shell uname -s)

## setup    — install Rust + system dependencies (run once)
setup:
	@chmod +x scripts/setup.sh
	@./scripts/setup.sh

## dev      — setup + run in one command (setup script is idempotent)
dev:
	@chmod +x scripts/setup.sh
	@./scripts/setup.sh
	@$(CARGO) run

## run      — build & run in development mode
run:
	@$(CARGO) run

## build    — build debug binary
build:
	@$(CARGO) build

## release  — build optimised release binary  →  ./target/release/shellkeeper
release:
	@$(CARGO) build --release
	@echo ""
	@echo "  Binary: ./target/release/$(BINARY)"
	@echo "  Size:   $$(du -sh target/release/$(BINARY) | cut -f1)"

## check    — fast type-check without compiling
check:
	@$(CARGO) check

## install  — install release binary to $(PREFIX)  (default: /usr/local/bin)
install: release
	@sudo cp target/release/$(BINARY) $(PREFIX)/$(BINARY)
	@echo "  Installed → $(PREFIX)/$(BINARY)"

## deb      — build a .deb package (Linux only)  →  ./target/debian/shellkeeper_*.deb
deb:
ifeq ($(UNAME), Linux)
	@which cargo-deb > /dev/null 2>&1 || cargo install cargo-deb
	@$(CARGO) deb
	@echo ""
	@echo "  Package: $$(ls target/debian/shellkeeper_*.deb 2>/dev/null | head -1)"
	@echo "  Install: sudo apt install ./target/debian/shellkeeper_*.deb"
else
	@echo "  .deb packaging is Linux-only. On macOS, use 'make release' + 'make install'."
endif

## clean    — remove build artifacts
clean:
	@$(CARGO) clean

## help     — show this help
help:
	@grep -E '^## ' Makefile | sed 's/## /  /'

.DEFAULT_GOAL := help
