#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# shellkeeper — setup script
# Installs Rust + system dependencies on Linux (apt/dnf) and macOS.
# Run once before the first `cargo run`.
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
RESET='\033[0m'

step()  { echo -e "\n${CYAN}▶  $*${RESET}"; }
ok()    { echo -e "${GREEN}✓  $*${RESET}"; }
warn()  { echo -e "${YELLOW}⚠  $*${RESET}"; }
die()   { echo -e "${RED}✗  $*${RESET}"; exit 1; }

echo -e "${CYAN}"
echo "  ╔══════════════════════════════╗"
echo "  ║   shellkeeper  —  setup      ║"
echo "  ╚══════════════════════════════╝"
echo -e "${RESET}"

# ── 1. Detect OS ──────────────────────────────────────────────────────────────
OS="$(uname -s)"
step "Detected OS: $OS"

install_linux_deps() {
    if command -v apt-get &>/dev/null; then
        # Only install what's missing — idempotent
        PKGS=(
            libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev
            libxkbcommon-dev libssl-dev pkg-config build-essential
            libfontconfig1-dev libdbus-1-dev
        )
        MISSING=()
        for pkg in "${PKGS[@]}"; do
            dpkg -s "$pkg" &>/dev/null || MISSING+=("$pkg")
        done
        if [ ${#MISSING[@]} -gt 0 ]; then
            step "Installing missing system dependencies: ${MISSING[*]}"
            sudo apt-get update -qq
            sudo apt-get install -y "${MISSING[@]}"
            ok "apt dependencies installed"
        else
            ok "System dependencies already installed"
        fi

    elif command -v dnf &>/dev/null; then
        step "Installing system dependencies (dnf)…"
        sudo dnf install -y \
            libxcb-devel \
            libxkbcommon-devel \
            openssl-devel \
            pkg-config \
            gcc \
            fontconfig-devel \
            dbus-devel
        ok "dnf dependencies installed"

    elif command -v pacman &>/dev/null; then
        step "Installing system dependencies (pacman)…"
        sudo pacman -Sy --noconfirm \
            libxcb \
            libxkbcommon \
            openssl \
            pkg-config \
            fontconfig \
            dbus
        ok "pacman dependencies installed"

    else
        warn "Unknown package manager. Install manually:"
        warn "  libxcb-render, libxcb-shape, libxcb-xfixes, libxkbcommon, openssl, pkg-config"
    fi
}

install_macos_deps() {
    if ! command -v brew &>/dev/null; then
        warn "Homebrew not found. Installing…"
        /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
    fi
    # macOS has everything needed via Xcode CLI tools
    if ! xcode-select -p &>/dev/null; then
        step "Installing Xcode Command Line Tools…"
        xcode-select --install
        warn "Re-run this script after Xcode CLI tools finish installing."
        exit 0
    fi
    # Install optional runtime tools via brew if missing
    BREW_PKGS=()
    command -v sshpass &>/dev/null || BREW_PKGS+=(sshpass)
    command -v tmux    &>/dev/null || BREW_PKGS+=(tmux)
    if [ ${#BREW_PKGS[@]} -gt 0 ]; then
        step "Installing missing brew packages: ${BREW_PKGS[*]}"
        brew install "${BREW_PKGS[@]}"
    fi
    ok "macOS dependencies OK"
}

case "$OS" in
    Linux)  install_linux_deps ;;
    Darwin) install_macos_deps ;;
    *)      die "Unsupported OS: $OS" ;;
esac

# ── 2. Install Rust ───────────────────────────────────────────────────────────
if command -v cargo &>/dev/null && command -v rustc &>/dev/null; then
    ok "Rust already installed: $(cargo --version)"
else
    step "Installing Rust via rustup…"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    # Source the env for this session
    # shellcheck source=/dev/null
    source "$HOME/.cargo/env"
    ok "Rust installed: $(cargo --version)"
fi

# Ensure cargo is on PATH for this session
export PATH="$HOME/.cargo/bin:$PATH"

# ── 3. Verify ─────────────────────────────────────────────────────────────────
step "Verifying setup…"
cargo --version || die "cargo not found after install. Open a new terminal and try again."
rustc --version
ok "All good!"

echo -e "\n${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "  Setup complete! Run the app with:"
echo -e ""
echo -e "    make run      ← development (fast compile)"
echo -e "    make release  ← optimised binary"
echo -e "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
