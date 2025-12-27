#!/bin/bash
# digrag install script
# Automatically detects architecture and installs the appropriate binary

set -e

VERSION="${DIGRAG_VERSION:-latest}"
INSTALL_DIR="${DIGRAG_INSTALL_DIR:-$HOME/.local/bin}"
GITHUB_REPO="takets/digrag"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

# Detect OS and architecture
detect_platform() {
    local os=$(uname -s | tr '[:upper:]' '[:lower:]')
    local arch=$(uname -m)
    
    case "$os" in
        linux)
            case "$arch" in
                x86_64|amd64)
                    echo "x86_64-unknown-linux-musl"
                    ;;
                aarch64|arm64)
                    echo "aarch64-unknown-linux-gnu"
                    ;;
                *)
                    error "Unsupported Linux architecture: $arch"
                    ;;
            esac
            ;;
        darwin)
            case "$arch" in
                x86_64|amd64)
                    echo "x86_64-apple-darwin"
                    ;;
                arm64|aarch64)
                    echo "aarch64-apple-darwin"
                    ;;
                *)
                    error "Unsupported macOS architecture: $arch"
                    ;;
            esac
            ;;
        *)
            error "Unsupported operating system: $os"
            ;;
    esac
}

# Get the latest release version
get_latest_version() {
    if command -v curl &> /dev/null; then
        curl -sL "https://api.github.com/repos/${GITHUB_REPO}/releases/latest" | \
            grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/'
    elif command -v wget &> /dev/null; then
        wget -qO- "https://api.github.com/repos/${GITHUB_REPO}/releases/latest" | \
            grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/'
    else
        error "Neither curl nor wget found. Please install one of them."
    fi
}

# Download and install binary
install_binary() {
    local platform="$1"
    local version="$2"
    local install_dir="$3"
    
    local binary_name="digrag-${version}-${platform}"
    local download_url="https://github.com/${GITHUB_REPO}/releases/download/${version}/${binary_name}.tar.gz"
    
    info "Downloading digrag ${version} for ${platform}..."
    
    local tmp_dir=$(mktemp -d)
    trap "rm -rf $tmp_dir" EXIT
    
    if command -v curl &> /dev/null; then
        curl -sL "$download_url" -o "${tmp_dir}/digrag.tar.gz" || \
            error "Failed to download from ${download_url}"
    else
        wget -q "$download_url" -O "${tmp_dir}/digrag.tar.gz" || \
            error "Failed to download from ${download_url}"
    fi
    
    info "Extracting..."
    tar -xzf "${tmp_dir}/digrag.tar.gz" -C "$tmp_dir"
    
    info "Installing to ${install_dir}..."
    mkdir -p "$install_dir"
    mv "${tmp_dir}/digrag" "${install_dir}/digrag"
    chmod +x "${install_dir}/digrag"
    
    info "Installation complete!"
}

# Build from source if release not available
build_from_source() {
    info "Building from source..."
    
    if ! command -v cargo &> /dev/null; then
        error "Cargo not found. Please install Rust: https://rustup.rs"
    fi
    
    if ! command -v git &> /dev/null; then
        error "Git not found. Please install git."
    fi
    
    local tmp_dir=$(mktemp -d)
    trap "rm -rf $tmp_dir" EXIT
    
    info "Cloning repository..."
    git clone --depth 1 "https://github.com/${GITHUB_REPO}.git" "$tmp_dir/digrag"
    
    info "Building..."
    cd "$tmp_dir/digrag"
    cargo build --release
    
    info "Installing to ${INSTALL_DIR}..."
    mkdir -p "$INSTALL_DIR"
    cp "target/release/digrag" "$INSTALL_DIR/"
    chmod +x "${INSTALL_DIR}/digrag"
    
    info "Installation complete!"
}

# Main installation flow
main() {
    info "digrag installer"
    echo ""
    
    local platform=$(detect_platform)
    info "Detected platform: $platform"
    
    # Create install directory
    mkdir -p "$INSTALL_DIR"
    
    if [ "$VERSION" = "latest" ]; then
        VERSION=$(get_latest_version)
        if [ -z "$VERSION" ]; then
            warn "Could not detect latest version, building from source..."
            build_from_source
            return
        fi
    fi
    
    info "Version: $VERSION"
    
    # Try to download release binary
    install_binary "$platform" "$VERSION" "$INSTALL_DIR" 2>/dev/null || {
        warn "Pre-built binary not available, building from source..."
        build_from_source
    }
    
    echo ""
    info "digrag installed to: ${INSTALL_DIR}/digrag"
    
    # Check if install dir is in PATH
    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        warn "Add ${INSTALL_DIR} to your PATH:"
        echo ""
        echo "  export PATH=\"\$PATH:${INSTALL_DIR}\""
        echo ""
        echo "Add this line to your ~/.bashrc or ~/.zshrc"
    fi
    
    # Run init
    info "Initializing configuration..."
    "${INSTALL_DIR}/digrag" init 2>/dev/null || true
    
    echo ""
    info "Installation complete! Run 'digrag --help' to get started."
}

main "$@"
