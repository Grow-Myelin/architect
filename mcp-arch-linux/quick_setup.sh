#!/bin/bash

set -euo pipefail

echo "==================================================="
echo "   MCP Arch Linux Server - Quick Setup Script    "
echo "==================================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running as regular user (not root)
if [[ $EUID -eq 0 ]]; then
   log_error "This script should NOT be run as root for the installation part."
   log_info "It will ask for sudo when needed."
   exit 1
fi

# Step 1: Check prerequisites
log_info "Checking prerequisites..."

if ! command -v pacman &> /dev/null; then
    log_error "This script is for Arch Linux only!"
    exit 1
fi

# Update system
log_info "Updating system packages..."
sudo pacman -Syu --noconfirm

# Install essential packages
log_info "Installing essential packages..."
sudo pacman -S --needed --noconfirm \
    base-devel \
    git \
    vim \
    curl \
    dbus \
    systemd-libs \
    jq

# Install Rust if not present
if ! command -v cargo &> /dev/null; then
    log_info "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    export PATH="$HOME/.cargo/bin:$PATH"
else
    log_info "Rust already installed, updating..."
    rustup update
fi

# Ensure we have a recent enough Rust version
log_info "Checking Rust version..."
RUST_VERSION=$(rustc --version | cut -d' ' -f2)
log_info "Current Rust version: $RUST_VERSION"

# Update Rust if version is too old
if rustc --version | grep -q "1\.[0-7][0-9]\."; then
    log_warn "Rust version may be too old, updating..."
    rustup update
fi

# Install Hyprland and related tools (optional)
read -p "Do you want to install Hyprland and screen capture tools? (y/N): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    log_info "Installing Hyprland and screen capture tools..."
    sudo pacman -S --needed --noconfirm \
        hyprland \
        waybar \
        wofi \
        grim \
        slurp \
        wf-recorder
fi

# Step 2: Build the MCP server
log_info "Building MCP server..."
if [ ! -f "Cargo.toml" ]; then
    log_error "Please run this script from the mcp-arch-linux directory"
    exit 1
fi

log_info "This may take a few minutes for the first build..."
if ! cargo build --release; then
    log_error "Build failed! Please check the error messages above."
    log_info "You may need to update your Rust installation:"
    log_info "  rustup update"
    exit 1
fi

log_info "Build successful!"

# Step 3: Install the server
log_info "Installing MCP server..."
sudo cp target/release/mcp-arch-server /usr/local/bin/
sudo chmod +x /usr/local/bin/mcp-arch-server

# Create directories
sudo mkdir -p /var/log/mcp-arch-linux
sudo mkdir -p /var/lib/mcp-arch-linux/{snapshots,captures}
sudo mkdir -p /etc/mcp-arch-linux

# Install systemd service
sudo cp systemd/mcp-arch-linux.service /etc/systemd/system/

# Create basic config
sudo tee /etc/mcp-arch-linux/config.toml > /dev/null <<EOF
# MCP Arch Linux Server Configuration
bind_address = "127.0.0.1:8080"
max_concurrent_operations = 10
require_auth = false  # Disabled for easier setup
audit_log_path = "/var/log/mcp-arch-linux/audit.log"

# Plugins to load
plugins = ["arch_install", "hyprland", "screen_capture"]
EOF

# Set permissions
sudo chown -R root:root /etc/mcp-arch-linux
sudo chmod 644 /etc/mcp-arch-linux/config.toml

# For development, make logs accessible to current user
sudo chown -R $USER:$USER /var/log/mcp-arch-linux
sudo chown -R $USER:$USER /var/lib/mcp-arch-linux

# Step 4: Start the service
log_info "Starting MCP server..."
sudo systemctl daemon-reload
sudo systemctl enable mcp-arch-linux
sudo systemctl start mcp-arch-linux

# Check status
sleep 2
if sudo systemctl is-active --quiet mcp-arch-linux; then
    log_info "MCP server is running successfully!"
else
    log_error "MCP server failed to start. Checking logs..."
    sudo journalctl -u mcp-arch-linux -n 20 --no-pager
    exit 1
fi

# Step 5: Test the server
log_info "Testing server connection..."
if timeout 5 bash -c 'until nc -z localhost 8080; do sleep 1; done'; then
    log_info "Server is accepting connections on port 8080"
else
    log_warn "Server might not be listening yet. Check with: sudo systemctl status mcp-arch-linux"
fi

# Display next steps
echo ""
echo "==================================================="
echo -e "${GREEN}   Setup Complete!${NC}"
echo "==================================================="
echo ""
echo "Your MCP Arch Linux server is now running!"
echo ""
echo -e "${BLUE}Next steps:${NC}"
echo "1. Check server status:"
echo "   sudo systemctl status mcp-arch-linux"
echo ""
echo "2. View logs:"
echo "   sudo journalctl -u mcp-arch-linux -f"
echo ""
echo "3. Test connection:"
echo "   curl -X POST http://localhost:8080 \\"
echo "     -H 'Content-Type: application/json' \\"
echo "     -d '{\"jsonrpc\":\"2.0\",\"method\":\"tools/list\",\"id\":1}'"
echo ""
echo "4. Configure Claude Code on your client machine:"
echo "   See SETUP_GUIDE.md for detailed instructions"
echo ""
echo -e "${YELLOW}Security Note:${NC} Authentication is disabled for easier testing."
echo "Enable it in /etc/mcp-arch-linux/config.toml for production use."
echo ""

# Offer to show live logs
read -p "Do you want to view live logs now? (y/N): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "Press Ctrl+C to exit log view"
    sleep 2
    sudo journalctl -u mcp-arch-linux -f
fi