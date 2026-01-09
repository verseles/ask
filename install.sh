#!/bin/sh
# ask installer for Unix systems
# https://github.com/verseles/ask
#
# Licensed under AGPL-3.0

set -eu

REPO="verseles/ask"
BINARY_NAME="ask"
INSTALL_DIR="${HOME}/.local/bin"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

info() {
    printf "${CYAN}info${NC}: %s\n" "$1"
}

success() {
    printf "${GREEN}success${NC}: %s\n" "$1"
}

warn() {
    printf "${YELLOW}warning${NC}: %s\n" "$1"
}

error() {
    printf "${RED}error${NC}: %s\n" "$1" >&2
    exit 1
}

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Linux*)  echo "linux" ;;
        Darwin*) echo "darwin" ;;
        *)       error "Unsupported operating system: $(uname -s)" ;;
    esac
}

# Check if running in Termux
is_termux() {
    [ -n "${TERMUX_VERSION:-}" ] || [ -d "/data/data/com.termux" ]
}

# Detect architecture
detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)  echo "x86_64" ;;
        aarch64|arm64) echo "aarch64" ;;
        *)             error "Unsupported architecture: $(uname -m)" ;;
    esac
}

# Get latest version from GitHub
get_latest_version() {
    if command -v curl >/dev/null 2>&1; then
        curl -sL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/'
    elif command -v wget >/dev/null 2>&1; then
        wget -qO- "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/'
    else
        error "curl or wget is required"
    fi
}

# Download file
download() {
    url="$1"
    output="$2"

    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$url" -o "$output"
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$url" -O "$output"
    else
        error "curl or wget is required"
    fi
}

# Verify checksum
verify_checksum() {
    file="$1"
    expected="$2"

    if command -v sha256sum >/dev/null 2>&1; then
        actual=$(sha256sum "$file" | awk '{print $1}')
    elif command -v shasum >/dev/null 2>&1; then
        actual=$(shasum -a 256 "$file" | awk '{print $1}')
    else
        warn "sha256sum/shasum not found, skipping checksum verification"
        return 0
    fi

    if [ "$actual" != "$expected" ]; then
        error "Checksum verification failed"
    fi
}

setup_uinput() {
    UDEV_RULE='KERNEL=="uinput", SUBSYSTEM=="misc", TAG+="uaccess", OPTIONS+="static_node=uinput"'
    UDEV_FILE="/etc/udev/rules.d/99-uinput.rules"

    if [ -f "$UDEV_FILE" ]; then
        return 0
    fi

    echo ""
    info "Command injection requires access to /dev/uinput"
    echo ""
    echo "This allows 'ask' to type commands directly into your terminal."
    echo "Without this, you'll need to manually confirm each command."
    echo ""

    if [ -t 1 ] && [ -e /dev/tty ]; then
        printf "Setup uinput access now? [Y/n] "
        read -r answer < /dev/tty
        case "$answer" in
            [nN]*)
                warn "Skipped. Commands will require manual confirmation."
                return 0
                ;;
        esac
    else
        warn "Non-interactive mode. Skipping uinput setup."
        return 0
    fi

    echo "$UDEV_RULE" | sudo tee "$UDEV_FILE" > /dev/null 2>&1
    if [ $? -eq 0 ]; then
        sudo udevadm control --reload-rules 2>/dev/null
        sudo udevadm trigger 2>/dev/null
        sudo usermod -a -G input "$(whoami)" 2>/dev/null
        success "uinput configured. Log out and back in for full effect."
    else
        warn "Could not configure uinput. Commands will require manual confirmation."
    fi
}

setup_macos_accessibility() {
    echo ""
    info "Command injection requires Accessibility permission"
    echo ""
    echo "To enable automatic command pasting:"
    echo "  1. Open System Settings → Privacy & Security → Accessibility"
    echo "  2. Click '+' and add your terminal app (Terminal, iTerm2, etc.)"
    echo "  3. Enable the toggle next to it"
    echo ""
    echo "Without this, commands will require manual confirmation."
    echo ""
}

# Setup PATH in shell configuration files
setup_path() {
    PATH_EXPORT='export PATH="$HOME/.local/bin:$PATH"'
    SHELLS_CONFIGURED=""

    # Ask user if they want automatic configuration
    if [ -t 1 ] && [ -e /dev/tty ]; then
        printf "Add ~/.local/bin to PATH automatically? [Y/n] "
        read -r answer < /dev/tty
        case "$answer" in
            [nN]*)
                echo ""
                echo "Add it manually to your shell configuration:"
                echo ""
                echo "  # For bash:"
                echo "  echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.bashrc"
                echo ""
                echo "  # For zsh:"
                echo "  echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.zshrc"
                echo ""
                echo "  # For fish:"
                echo "  fish_add_path ~/.local/bin"
                echo ""
                return 1
                ;;
        esac
    else
        # Non-interactive mode - show manual instructions
        echo ""
        echo "Add ~/.local/bin to your PATH:"
        echo ""
        echo "  # For bash:"
        echo "  echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.bashrc"
        echo ""
        echo "  # For zsh:"
        echo "  echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.zshrc"
        echo ""
        echo "  # For fish:"
        echo "  fish_add_path ~/.local/bin"
        echo ""
        return 1
    fi

    # Configure bash if .bashrc exists or bash is installed
    if [ -f "$HOME/.bashrc" ] || command -v bash >/dev/null 2>&1; then
        BASHRC="$HOME/.bashrc"
        if [ ! -f "$BASHRC" ]; then
            touch "$BASHRC"
        fi
        if ! grep -q '\.local/bin' "$BASHRC" 2>/dev/null; then
            echo "" >> "$BASHRC"
            echo "# Added by ask installer" >> "$BASHRC"
            echo "$PATH_EXPORT" >> "$BASHRC"
            SHELLS_CONFIGURED="${SHELLS_CONFIGURED}bash "
        fi
    fi

    # Configure zsh if .zshrc exists or zsh is installed
    if [ -f "$HOME/.zshrc" ] || command -v zsh >/dev/null 2>&1; then
        ZSHRC="$HOME/.zshrc"
        if [ ! -f "$ZSHRC" ]; then
            touch "$ZSHRC"
        fi
        if ! grep -q '\.local/bin' "$ZSHRC" 2>/dev/null; then
            echo "" >> "$ZSHRC"
            echo "# Added by ask installer" >> "$ZSHRC"
            echo "$PATH_EXPORT" >> "$ZSHRC"
            SHELLS_CONFIGURED="${SHELLS_CONFIGURED}zsh "
        fi
    fi

    # Configure fish if config exists or fish is installed
    if [ -f "$HOME/.config/fish/config.fish" ] || command -v fish >/dev/null 2>&1; then
        FISH_CONFIG="$HOME/.config/fish/config.fish"
        if [ ! -f "$FISH_CONFIG" ]; then
            mkdir -p "$HOME/.config/fish"
            touch "$FISH_CONFIG"
        fi
        if ! grep -q '\.local/bin' "$FISH_CONFIG" 2>/dev/null; then
            echo "" >> "$FISH_CONFIG"
            echo "# Added by ask installer" >> "$FISH_CONFIG"
            echo "fish_add_path \$HOME/.local/bin" >> "$FISH_CONFIG"
            SHELLS_CONFIGURED="${SHELLS_CONFIGURED}fish "
        fi
    fi

    if [ -n "$SHELLS_CONFIGURED" ]; then
        success "PATH configured for: ${SHELLS_CONFIGURED}"
        echo ""
        echo "Reload your shell or run:"
        case "$SHELL" in
            */bash) echo "  source ~/.bashrc" ;;
            */zsh)  echo "  source ~/.zshrc" ;;
            */fish) echo "  source ~/.config/fish/config.fish" ;;
            *)      echo "  source ~/.bashrc  # or your shell's config file" ;;
        esac
        echo ""
        # Export PATH for current session so ask init works
        export PATH="$HOME/.local/bin:$PATH"
        return 0
    else
        info "PATH already configured in shell files"
        export PATH="$HOME/.local/bin:$PATH"
        return 0
    fi
}

main() {
    info "Installing ${BINARY_NAME}..."

    OS=$(detect_os)
    ARCH=$(detect_arch)

    info "Detected: ${OS}-${ARCH}"

    # Map to artifact name
    case "${OS}-${ARCH}" in
        linux-x86_64)   ARTIFACT="ask-linux-x86_64" ;;
        linux-aarch64)  ARTIFACT="ask-linux-aarch64" ;;
        darwin-x86_64)  ARTIFACT="ask-macos-x86_64" ;;
        darwin-aarch64) ARTIFACT="ask-macos-aarch64" ;;
        *)              error "Unsupported platform: ${OS}-${ARCH}" ;;
    esac

    VERSION=$(get_latest_version)
    if [ -z "$VERSION" ]; then
        error "Could not determine latest version"
    fi

    info "Latest version: ${VERSION}"

    # Create temp directory
    TMP_DIR=$(mktemp -d)
    trap "rm -rf '$TMP_DIR'" EXIT

    BINARY_URL="https://github.com/${REPO}/releases/download/${VERSION}/${ARTIFACT}"
    CHECKSUM_URL="${BINARY_URL}.sha256"

    info "Downloading ${BINARY_NAME}..."
    download "$BINARY_URL" "${TMP_DIR}/${BINARY_NAME}"
    download "$CHECKSUM_URL" "${TMP_DIR}/${BINARY_NAME}.sha256"

    # Verify checksum
    EXPECTED_CHECKSUM=$(awk '{print $1}' "${TMP_DIR}/${BINARY_NAME}.sha256")
    verify_checksum "${TMP_DIR}/${BINARY_NAME}" "$EXPECTED_CHECKSUM"
    success "Checksum verified"

    # Install binary
    mkdir -p "$INSTALL_DIR"
    mv "${TMP_DIR}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
    chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

    success "Installed ${BINARY_NAME} to ${INSTALL_DIR}/${BINARY_NAME}"

    # Check PATH and configure if needed
    case ":$PATH:" in
        *":${INSTALL_DIR}:"*)
            # PATH is already configured
            ;;
        *)
            warn "${INSTALL_DIR} is not in your PATH"
            echo ""
            setup_path
            ;;
    esac

    echo ""
    success "Installation complete!"
    echo ""

    # Linux-specific: Setup uinput for command injection (skip on Termux)
    if [ "$OS" = "linux" ] && ! is_termux; then
        setup_uinput
    fi

    # Termux-specific message
    if is_termux; then
        echo ""
        info "Termux detected"
        echo ""
        echo "Note: Command injection is not available on Termux/Android."
        echo "Commands will be displayed for you to copy manually."
        echo ""
    fi

    # macOS-specific: Inform about Accessibility permission
    if [ "$OS" = "darwin" ]; then
        setup_macos_accessibility
    fi

    if [ -t 1 ] && [ -e /dev/tty ]; then
        printf "Configure API keys now? [Y/n] "
        read -r answer < /dev/tty
        case "$answer" in
            [nN]*)
                echo ""
                echo "Run '${BINARY_NAME} init' when ready to configure."
                ;;
            *)
                echo ""
                "${INSTALL_DIR}/${BINARY_NAME}" init < /dev/tty
                ;;
        esac
    else
        echo "Get started:"
        echo "  ${BINARY_NAME} init    # Configure API keys"
        echo "  ${BINARY_NAME} --help  # Show help"
    fi
    echo ""
}

main "$@"
