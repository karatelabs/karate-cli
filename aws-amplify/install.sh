#!/bin/sh
# Karate CLI Installer for Unix/macOS
# Usage: curl -fsSL https://karate.sh/install.sh | sh
#        curl -fsSL https://karate.sh/install.sh | sh -s -- --all
#
# Options:
#   --all           Download JRE + JAR immediately after install
#   --bin-dir DIR   Install to custom directory (default: ~/.local/bin)
#   --version VER   Install specific version (default: latest)

set -e

# Configuration
GITHUB_REPO="karatelabs/karate-cli"
INSTALL_DIR="${HOME}/.local/bin"
VERSION="latest"
AUTO_SETUP=""

# Colors (respect NO_COLOR)
if [ -z "${NO_COLOR:-}" ] && [ -t 1 ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    BLUE='\033[0;34m'
    BOLD='\033[1m'
    RESET='\033[0m'
else
    RED=''
    GREEN=''
    YELLOW=''
    BLUE=''
    BOLD=''
    RESET=''
fi

info() {
    printf "${BLUE}==>${RESET} ${BOLD}%s${RESET}\n" "$1"
}

success() {
    printf "${GREEN}==>${RESET} ${BOLD}%s${RESET}\n" "$1"
}

warn() {
    printf "${YELLOW}Warning:${RESET} %s\n" "$1"
}

error() {
    printf "${RED}Error:${RESET} %s\n" "$1" >&2
    exit 1
}

# Parse arguments
while [ $# -gt 0 ]; do
    case "$1" in
        --all|-a)
            AUTO_SETUP="--all"
            shift
            ;;
        --bin-dir)
            INSTALL_DIR="$2"
            shift 2
            ;;
        --version)
            VERSION="$2"
            shift 2
            ;;
        --help|-h)
            echo "Karate CLI Installer"
            echo ""
            echo "Usage: curl -fsSL https://karate.sh/install.sh | sh -s -- [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --all, -a        Download JRE + JAR immediately after install"
            echo "  --bin-dir DIR    Install to custom directory (default: ~/.local/bin)"
            echo "  --version VER    Install specific version (default: latest)"
            echo "  --help, -h       Show this help"
            exit 0
            ;;
        *)
            warn "Unknown option: $1"
            shift
            ;;
    esac
done

# Detect OS
detect_os() {
    OS="$(uname -s)"
    case "$OS" in
        Darwin)
            echo "darwin"
            ;;
        Linux)
            echo "linux"
            ;;
        MINGW*|MSYS*|CYGWIN*)
            error "Windows detected. Please use PowerShell:\n  irm https://karate.sh/install.ps1 | iex"
            ;;
        *)
            error "Unsupported operating system: $OS"
            ;;
    esac
}

# Detect architecture
detect_arch() {
    ARCH="$(uname -m)"
    case "$ARCH" in
        x86_64|amd64)
            echo "x64"
            ;;
        aarch64|arm64)
            echo "arm64"
            ;;
        *)
            error "Unsupported architecture: $ARCH"
            ;;
    esac
}

# Check for required tools
check_requirements() {
    if ! command -v curl >/dev/null 2>&1 && ! command -v wget >/dev/null 2>&1; then
        error "curl or wget is required but not installed"
    fi

    if ! command -v tar >/dev/null 2>&1; then
        error "tar is required but not installed"
    fi
}

# Download file with curl or wget
download() {
    URL="$1"
    OUTPUT="$2"

    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$URL" -o "$OUTPUT"
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$URL" -O "$OUTPUT"
    fi
}

# Get latest version from GitHub
get_latest_version() {
    LATEST_URL="https://api.github.com/repos/${GITHUB_REPO}/releases/latest"

    if command -v curl >/dev/null 2>&1; then
        VERSION_TAG=$(curl -fsSL "$LATEST_URL" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    else
        VERSION_TAG=$(wget -qO- "$LATEST_URL" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    fi

    if [ -z "$VERSION_TAG" ]; then
        error "Failed to fetch latest version from GitHub"
    fi

    # Remove 'v' prefix if present
    echo "$VERSION_TAG" | sed 's/^v//'
}

# Verify SHA256 checksum
verify_checksum() {
    FILE="$1"
    EXPECTED="$2"

    if command -v sha256sum >/dev/null 2>&1; then
        ACTUAL=$(sha256sum "$FILE" | cut -d ' ' -f 1)
    elif command -v shasum >/dev/null 2>&1; then
        ACTUAL=$(shasum -a 256 "$FILE" | cut -d ' ' -f 1)
    else
        warn "Cannot verify checksum (sha256sum/shasum not found)"
        return 0
    fi

    if [ "$ACTUAL" != "$EXPECTED" ]; then
        error "Checksum verification failed!\n  Expected: $EXPECTED\n  Actual:   $ACTUAL"
    fi
}

# Main installation
main() {
    info "Karate CLI Installer"
    echo ""

    check_requirements

    OS=$(detect_os)
    ARCH=$(detect_arch)
    PLATFORM="${OS}-${ARCH}"

    info "Detected platform: ${PLATFORM}"

    # Get version
    if [ "$VERSION" = "latest" ]; then
        info "Fetching latest version..."
        VERSION=$(get_latest_version)
    fi

    info "Installing Karate CLI v${VERSION}"

    # Construct download URLs
    BINARY_NAME="karate-${PLATFORM}"
    TARBALL_NAME="${BINARY_NAME}.tar.gz"
    DOWNLOAD_URL="https://github.com/${GITHUB_REPO}/releases/download/v${VERSION}/${TARBALL_NAME}"
    CHECKSUM_URL="https://github.com/${GITHUB_REPO}/releases/download/v${VERSION}/${TARBALL_NAME}.sha256"

    # Create temp directory
    TMP_DIR=$(mktemp -d)
    trap 'rm -rf "$TMP_DIR"' EXIT

    # Download tarball
    info "Downloading ${TARBALL_NAME}..."
    download "$DOWNLOAD_URL" "${TMP_DIR}/${TARBALL_NAME}"

    # Download and verify checksum
    info "Verifying checksum..."
    download "$CHECKSUM_URL" "${TMP_DIR}/checksum.txt"
    EXPECTED_CHECKSUM=$(cat "${TMP_DIR}/checksum.txt" | cut -d ' ' -f 1)
    verify_checksum "${TMP_DIR}/${TARBALL_NAME}" "$EXPECTED_CHECKSUM"

    # Extract
    info "Extracting..."
    tar -xzf "${TMP_DIR}/${TARBALL_NAME}" -C "$TMP_DIR"

    # Create install directory
    mkdir -p "$INSTALL_DIR"

    # Install binary
    info "Installing to ${INSTALL_DIR}/karate..."
    mv "${TMP_DIR}/karate" "${INSTALL_DIR}/karate"
    chmod +x "${INSTALL_DIR}/karate"

    success "Karate CLI v${VERSION} installed successfully!"
    echo ""

    # Check if in PATH
    case ":$PATH:" in
        *":${INSTALL_DIR}:"*)
            # Already in PATH
            ;;
        *)
            echo "${YELLOW}Note:${RESET} ${INSTALL_DIR} is not in your PATH."
            echo ""
            echo "Add it by running:"
            echo ""

            # Detect shell
            SHELL_NAME=$(basename "$SHELL")
            case "$SHELL_NAME" in
                zsh)
                    echo "  echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.zshrc"
                    echo "  source ~/.zshrc"
                    ;;
                bash)
                    if [ -f "$HOME/.bash_profile" ]; then
                        echo "  echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.bash_profile"
                        echo "  source ~/.bash_profile"
                    else
                        echo "  echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.bashrc"
                        echo "  source ~/.bashrc"
                    fi
                    ;;
                fish)
                    echo "  fish_add_path ~/.local/bin"
                    ;;
                *)
                    echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
                    ;;
            esac
            echo ""
            ;;
    esac

    # Run setup if requested
    if [ -n "$AUTO_SETUP" ]; then
        info "Running karate setup..."
        "${INSTALL_DIR}/karate" setup --all
    else
        echo "Next steps:"
        echo ""
        echo "  ${BOLD}karate setup${RESET}    # Download JRE and Karate JAR"
        echo "  ${BOLD}karate doctor${RESET}   # Verify installation"
        echo "  ${BOLD}karate run${RESET}      # Run your first test"
        echo ""
    fi
}

main
