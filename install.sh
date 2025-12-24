#!/usr/bin/env bash
#
# Sentinel Installation Script
# Usage: curl -sSL https://raw.githubusercontent.com/EngramAI-io/Core/main/install.sh | bash
#
# This script installs the latest release of Sentinel
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
REPO="EngramAI-io/Core"
BINARY_NAME="sentinel"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

# Detect OS and Architecture
detect_platform() {
    local os="$(uname -s)"
    local arch="$(uname -m)"
    
    case "$os" in
        Linux*)
            OS="linux"
            ;;
        Darwin*)
            OS="darwin"
            ;;
        MINGW*|MSYS*|CYGWIN*)
            OS="windows"
            ;;
        *)
            echo -e "${RED}❌ Unsupported operating system: $os${NC}"
            exit 1
            ;;
    esac
    
    case "$arch" in
        x86_64|amd64)
            ARCH="x86_64"
            ;;
        aarch64|arm64)
            ARCH="aarch64"
            ;;
        *)
            echo -e "${RED}❌ Unsupported architecture: $arch${NC}"
            exit 1
            ;;
    esac
    
    echo -e "${BLUE}🔍 Detected platform: ${OS}-${ARCH}${NC}"
}

# Get the latest release version
get_latest_version() {
    echo -e "${BLUE}📡 Fetching latest release...${NC}"
    
    LATEST_VERSION=$(curl -sSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    
    if [ -z "$LATEST_VERSION" ]; then
        echo -e "${YELLOW}⚠️  No releases found, trying to build from source...${NC}"
        install_from_source
        exit 0
    fi
    
    echo -e "${GREEN}✅ Latest version: ${LATEST_VERSION}${NC}"
}

# Download and install binary
install_binary() {
    local version="$1"
    local binary_name="${BINARY_NAME}"
    
    if [ "$OS" = "windows" ]; then
        binary_name="${BINARY_NAME}.exe"
    fi
    
    # Construct download URL (adjust based on your release naming)
    local download_url="https://github.com/${REPO}/releases/download/${version}/${BINARY_NAME}-${OS}-${ARCH}"
    
    if [ "$OS" = "windows" ]; then
        download_url="${download_url}.exe"
    fi
    
    echo -e "${BLUE}📥 Downloading from: ${download_url}${NC}"
    
    local tmp_dir="$(mktemp -d)"
    local tmp_file="${tmp_dir}/${binary_name}"
    
    if ! curl -sSL -o "${tmp_file}" "${download_url}"; then
        echo -e "${YELLOW}⚠️  Binary not found for ${OS}-${ARCH}, building from source...${NC}"
        rm -rf "${tmp_dir}"
        install_from_source
        return
    fi
    
    chmod +x "${tmp_file}"
    
    # Install binary
    echo -e "${BLUE}📦 Installing to ${INSTALL_DIR}...${NC}"
    
    if [ -w "$INSTALL_DIR" ]; then
        mv "${tmp_file}" "${INSTALL_DIR}/${binary_name}"
    else
        echo -e "${YELLOW}⚠️  Need sudo to install to ${INSTALL_DIR}${NC}"
        sudo mv "${tmp_file}" "${INSTALL_DIR}/${binary_name}"
    fi
    
    rm -rf "${tmp_dir}"
    
    echo -e "${GREEN}✅ Sentinel installed successfully!${NC}"
}

# Build and install from source
install_from_source() {
    echo -e "${BLUE}🔧 Building Sentinel from source...${NC}"
    
    # Check for dependencies
    if ! command -v cargo &> /dev/null; then
        echo -e "${YELLOW}📦 Installing Rust...${NC}"
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
    fi
    
    if ! command -v node &> /dev/null; then
        echo -e "${RED}❌ Node.js is required but not installed.${NC}"
        echo -e "${YELLOW}Please install Node.js 18+ from https://nodejs.org/${NC}"
        exit 1
    fi
    
    # Clone repository
    local tmp_dir="$(mktemp -d)"
    echo -e "${BLUE}📥 Cloning repository...${NC}"
    git clone --depth 1 "https://github.com/${REPO}.git" "${tmp_dir}/sentinel"
    cd "${tmp_dir}/sentinel"
    
    # Build frontend
    echo -e "${BLUE}🎨 Building frontend...${NC}"
    cd frontend/my-react-flow-app
    npm install
    npm run build
    cd ../..
    
    # Build Rust binary
    echo -e "${BLUE}⚙️  Building Rust binary...${NC}"
    cargo build --release
    
    # Install binary
    local binary_name="${BINARY_NAME}"
    if [ "$OS" = "windows" ]; then
        binary_name="${BINARY_NAME}.exe"
    fi
    
    echo -e "${BLUE}📦 Installing to ${INSTALL_DIR}...${NC}"
    
    if [ -w "$INSTALL_DIR" ]; then
        cp "target/release/${binary_name}" "${INSTALL_DIR}/${binary_name}"
    else
        sudo cp "target/release/${binary_name}" "${INSTALL_DIR}/${binary_name}"
    fi
    
    # Cleanup
    cd
    rm -rf "${tmp_dir}"
    
    echo -e "${GREEN}✅ Sentinel built and installed successfully!${NC}"
}

# Verify installation
verify_installation() {
    echo -e "${BLUE}🔍 Verifying installation...${NC}"
    
    if command -v sentinel &> /dev/null; then
        local version=$(sentinel --version 2>&1 || echo "unknown")
        echo -e "${GREEN}✅ Sentinel is installed: ${version}${NC}"
        
        echo ""
        echo -e "${BLUE}📚 Quick Start:${NC}"
        echo ""
        echo -e "  ${GREEN}# Generate signing keys${NC}"
        echo -e "  sentinel keygen --out-dir ./keys"
        echo ""
        echo -e "  ${GREEN}# Run with an MCP server${NC}"
        echo -e "  sentinel run --signing-key-b64-path ./keys/signing_key.b64 -- npx @modelcontextprotocol/server-filesystem"
        echo ""
        echo -e "  ${GREEN}# With authentication (recommended)${NC}"
        echo -e "  sentinel run --ws-token 'your-secret' --signing-key-b64-path ./keys/signing_key.b64 -- your-mcp-server"
        echo ""
        echo -e "${BLUE}📖 Full documentation: https://github.com/${REPO}${NC}"
        echo ""
    else
        echo -e "${RED}❌ Installation verification failed${NC}"
        echo -e "${YELLOW}Please ensure ${INSTALL_DIR} is in your PATH${NC}"
        exit 1
    fi
}

# Main installation flow
main() {
    echo ""
    echo -e "${BLUE}╔═══════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║   Sentinel Installation Script       ║${NC}"
    echo -e "${BLUE}║   Secure MCP Audit Logging           ║${NC}"
    echo -e "${BLUE}╚═══════════════════════════════════════╝${NC}"
    echo ""
    
    detect_platform
    
    # Check if running from GitHub releases or need to build from source
    if [ "$1" = "--source" ]; then
        install_from_source
    else
        get_latest_version
        install_binary "$LATEST_VERSION"
    fi
    
    verify_installation
    
    echo ""
    echo -e "${GREEN}🎉 Installation complete!${NC}"
    echo ""
}

# Handle script arguments
if [ "$1" = "--help" ] || [ "$1" = "-h" ]; then
    echo "Sentinel Installation Script"
    echo ""
    echo "Usage:"
    echo "  curl -sSL https://raw.githubusercontent.com/EngramAI-io/Core/main/install.sh | bash"
    echo "  curl -sSL https://raw.githubusercontent.com/EngramAI-io/Core/main/install.sh | bash -s -- --source"
    echo ""
    echo "Options:"
    echo "  --source    Build from source instead of downloading binary"
    echo "  --help      Show this help message"
    echo ""
    echo "Environment Variables:"
    echo "  INSTALL_DIR    Installation directory (default: /usr/local/bin)"
    echo ""
    exit 0
fi

main "$@"
