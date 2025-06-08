#!/bin/bash

set -euo pipefail

# MCP Arch Linux Server - Quick Setup Script

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_arch_linux() {
    if [[ ! -f /etc/arch-release ]]; then
        log_error "This script is designed for Arch Linux only!"
        exit 1
    fi
}

check_user() {
    if [[ $EUID -eq 0 ]]; then
        log_error "This script should NOT be run as root for the installation part."
        log_info "It will ask for sudo when needed."
        exit 1
    fi
}

update_system() {
    log_info "Updating system packages..."
    sudo pacman -Syu --noconfirm
}

install_dependencies() {
    log_info "Installing essential packages..."
    
    local packages=(
        "nodejs"
        "npm"
        "git"
        "base-devel"
        "systemd"
    )
    
    sudo pacman -S --needed --noconfirm "${packages[@]}"
}

install_optional_tools() {
    read -p "Do you want to install Hyprland and screen capture tools? (y/N): " -n 1 -r
    echo
    
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        log_info "Installing Hyprland and screen capture tools..."
        
        local optional_packages=(
            "hyprland"
            "grim"
            "slurp"
            "wf-recorder"
            "smartmontools"
        )
        
        sudo pacman -S --needed --noconfirm "${optional_packages[@]}"
    fi
}

check_node_version() {
    local node_version
    node_version=$(node --version | cut -d'v' -f2)
    local required_version="18.0.0"
    
    if ! printf '%s\n%s\n' "$required_version" "$node_version" | sort -V -C; then
        log_error "Node.js version $node_version is too old. Minimum required: $required_version"
        log_info "Updating Node.js..."
        sudo pacman -S nodejs --noconfirm
    fi
    
    log_info "Node.js version: $(node --version) ✓"
}

build_application() {
    log_info "Installing Node.js dependencies..."
    
    if [[ ! -f "package.json" ]]; then
        log_error "Please run this script from the mcp-arch-linux directory"
        exit 1
    fi
    
    # Install dependencies
    npm install
    
    log_info "Dependencies installed successfully"
}

test_application() {
    log_info "Testing application..."
    
    # Run basic validation
    if node -e "require('./src/server.js')" 2>/dev/null; then
        log_info "Application validation passed ✓"
    else
        log_warn "Application validation failed, but continuing..."
    fi
}

install_system() {
    log_info "Installing to system..."
    sudo ./scripts/install.sh
}

configure_service() {
    log_info "Configuring service..."
    
    # Enable service
    sudo systemctl enable mcp-arch-linux
    
    # Start service
    sudo systemctl start mcp-arch-linux
    
    # Wait a moment for startup
    sleep 3
    
    # Check status
    if sudo systemctl is-active --quiet mcp-arch-linux; then
        log_info "MCP server is running successfully! ✓"
    else
        log_error "MCP server failed to start. Checking logs..."
        sudo journalctl -u mcp-arch-linux -n 20 --no-pager
        exit 1
    fi
}

test_server() {
    log_info "Testing server connection..."
    
    # Wait for server to be ready
    local max_attempts=10
    local attempt=1
    
    while (( attempt <= max_attempts )); do
        if curl -s -f http://localhost:8080/health >/dev/null 2>&1; then
            log_info "Server is responding on port 8080 ✓"
            break
        fi
        
        log_info "Waiting for server... (attempt $attempt/$max_attempts)"
        sleep 2
        ((attempt++))
    done
    
    if (( attempt > max_attempts )); then
        log_warn "Server health check timeout. Check with: sudo systemctl status mcp-arch-linux"
    fi
}

show_completion_info() {
    echo
    echo "=========================================="
    echo -e "${GREEN}   Setup Complete!${NC}"
    echo "=========================================="
    echo
    echo "Your MCP Arch Linux server is now running!"
    echo
    echo -e "${BLUE}Server Information:${NC}"
    echo "• URL: http://localhost:8080"
    echo "• Health endpoint: http://localhost:8080/health"
    echo "• MCP endpoint: http://localhost:8080/mcp"
    echo
    echo -e "${BLUE}Useful Commands:${NC}"
    echo "• Check status: sudo systemctl status mcp-arch-linux"
    echo "• View logs: sudo journalctl -u mcp-arch-linux -f"
    echo "• Restart: sudo systemctl restart mcp-arch-linux"
    echo "• Stop: sudo systemctl stop mcp-arch-linux"
    echo
    echo -e "${BLUE}Configuration:${NC}"
    echo "• Config file: /etc/mcp-arch-linux/server.yaml"
    echo "• Log directory: /var/log/mcp-arch-linux"
    echo "• Data directory: /var/lib/mcp-arch-linux"
    echo
    echo -e "${BLUE}Testing the Server:${NC}"
    echo "Test with curl:"
    echo "  curl http://localhost:8080/health"
    echo
    echo "List available tools:"
    echo '  curl -X POST http://localhost:8080/mcp \'
    echo '    -H "Content-Type: application/json" \'
    echo '    -d '\''{"jsonrpc":"2.0","method":"tools/list","id":1}'\'''
    echo
    echo -e "${BLUE}Next Steps:${NC}"
    echo "1. Configure Claude Code to use this MCP server"
    echo "2. Review security settings in the config file"
    echo "3. Set up authentication if needed"
    echo "4. Test system functionality"
    echo
    echo -e "${YELLOW}Security Note:${NC} Authentication is disabled by default."
    echo "Enable it in /etc/mcp-arch-linux/server.yaml for production use."
    echo
}

show_logs() {
    read -p "Do you want to view live logs now? (y/N): " -n 1 -r
    echo
    
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo "Press Ctrl+C to exit log view"
        sleep 2
        sudo journalctl -u mcp-arch-linux -f
    fi
}

main() {
    echo "==================================================="
    echo "   MCP Arch Linux Server - Quick Setup Script    "
    echo "==================================================="
    echo
    
    check_arch_linux
    check_user
    update_system
    install_dependencies
    install_optional_tools
    check_node_version
    build_application
    test_application
    install_system
    configure_service
    test_server
    show_completion_info
    show_logs
}

# Handle errors
trap 'log_error "Setup failed at line $LINENO. Check the error above."' ERR

# Run main function
main "$@"