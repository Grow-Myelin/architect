#!/bin/bash

set -euo pipefail

INSTALL_PREFIX="${INSTALL_PREFIX:-/usr/local}"
SERVICE_USER="${SERVICE_USER:-mcp-server}"
CONFIG_DIR="/etc/mcp-arch-linux"
LOG_DIR="/var/log/mcp-arch-linux"
LIB_DIR="/var/lib/mcp-arch-linux"

echo "MCP Arch Linux Server Installation Script"
echo "========================================"

# Check if running as root
if [[ $EUID -ne 0 ]]; then
   echo "This script must be run as root" 
   exit 1
fi

# Check for Rust installation
if ! command -v cargo &> /dev/null; then
    echo "Error: Rust/Cargo not found. Please install Rust first."
    echo "Visit: https://rustup.rs/"
    exit 1
fi

# Build the project
echo "Building MCP Arch Linux Server..."
cargo build --release

# Create directories
echo "Creating directories..."
mkdir -p "$INSTALL_PREFIX/bin"
mkdir -p "$CONFIG_DIR"
mkdir -p "$LOG_DIR"
mkdir -p "$LIB_DIR"/{snapshots,captures}

# Install binary
echo "Installing binary..."
cp target/release/mcp-arch-server "$INSTALL_PREFIX/bin/"
chmod +x "$INSTALL_PREFIX/bin/mcp-arch-server"

# Install systemd service
echo "Installing systemd service..."
cp systemd/mcp-arch-linux.service /etc/systemd/system/
systemctl daemon-reload

# Create default configuration
echo "Creating default configuration..."
cat > "$CONFIG_DIR/config.toml" <<EOF
# MCP Arch Linux Server Configuration

bind_address = "127.0.0.1:8080"
max_concurrent_operations = 10
require_auth = true
audit_log_path = "$LOG_DIR/audit.log"

# Plugins to load
plugins = ["arch_install", "hyprland", "screen_capture"]

[security]
allowed_commands = [
    "pacman",
    "systemctl",
    "hyprctl",
    "grim",
    "wf-recorder"
]
EOF

# Set permissions
echo "Setting permissions..."
chown -R root:root "$CONFIG_DIR"
chmod 600 "$CONFIG_DIR/config.toml"
chown -R "$SERVICE_USER:$SERVICE_USER" "$LOG_DIR" "$LIB_DIR" 2>/dev/null || true

# Optional: Install dependencies for screen capture
echo "Checking optional dependencies..."
if command -v pacman &> /dev/null; then
    echo "Installing optional dependencies..."
    pacman -S --needed --noconfirm grim wf-recorder jq
fi

echo ""
echo "Installation complete!"
echo ""
echo "To start the service:"
echo "  systemctl enable mcp-arch-linux"
echo "  systemctl start mcp-arch-linux"
echo ""
echo "To check status:"
echo "  systemctl status mcp-arch-linux"
echo ""
echo "Logs can be found at:"
echo "  journalctl -u mcp-arch-linux -f"
echo ""
echo "Configuration file:"
echo "  $CONFIG_DIR/config.toml"