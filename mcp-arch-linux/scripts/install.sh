#!/bin/bash

set -euo pipefail

# MCP Arch Linux Server Installation Script

INSTALL_PREFIX="${INSTALL_PREFIX:-/usr/local}"
SERVICE_USER="${SERVICE_USER:-mcp-server}"
CONFIG_DIR="/etc/mcp-arch-linux"
LOG_DIR="/var/log/mcp-arch-linux"
LIB_DIR="/var/lib/mcp-arch-linux"
SYSTEMD_DIR="/etc/systemd/system"

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

check_root() {
    if [[ $EUID -ne 0 ]]; then
        log_error "This script must be run as root"
        exit 1
    fi
}

check_node() {
    if ! command -v node &> /dev/null; then
        log_error "Node.js is required but not installed"
        log_info "Install Node.js with: sudo pacman -S nodejs npm"
        exit 1
    fi
    
    local node_version=$(node --version | cut -d'v' -f2)
    local required_version="18.0.0"
    
    if ! printf '%s\n%s\n' "$required_version" "$node_version" | sort -V -C; then
        log_error "Node.js version $node_version is too old. Minimum required: $required_version"
        exit 1
    fi
    
    log_info "Node.js version: $node_version ✓"
}

create_user() {
    if ! id "$SERVICE_USER" &>/dev/null; then
        log_info "Creating service user: $SERVICE_USER"
        useradd -r -s /bin/false -d /var/lib/mcp-arch-linux "$SERVICE_USER"
    else
        log_info "Service user $SERVICE_USER already exists"
    fi
}

create_directories() {
    log_info "Creating directories..."
    
    # Create main directories
    mkdir -p "$CONFIG_DIR"
    mkdir -p "$LOG_DIR"
    mkdir -p "$LIB_DIR"/{snapshots,captures}
    mkdir -p "$INSTALL_PREFIX/lib/mcp-arch-linux"
    
    # Set ownership
    chown root:root "$CONFIG_DIR"
    chown "$SERVICE_USER:$SERVICE_USER" "$LOG_DIR" "$LIB_DIR"
    find "$LIB_DIR" -type d -exec chown "$SERVICE_USER:$SERVICE_USER" {} \;
    
    # Set permissions
    chmod 755 "$CONFIG_DIR"
    chmod 755 "$LOG_DIR"
    chmod 755 "$LIB_DIR"
}

install_dependencies() {
    log_info "Installing Node.js dependencies..."
    
    # Copy package files
    cp package.json "$INSTALL_PREFIX/lib/mcp-arch-linux/"
    
    # Install dependencies
    cd "$INSTALL_PREFIX/lib/mcp-arch-linux"
    npm install --production --no-audit --no-fund
    
    log_info "Dependencies installed successfully"
}

install_application() {
    log_info "Installing MCP Arch Linux Server..."
    
    # Copy source files
    cp -r src/ "$INSTALL_PREFIX/lib/mcp-arch-linux/"
    
    # Create executable script
    cat > "$INSTALL_PREFIX/bin/mcp-arch-server" << EOF
#!/bin/bash
cd "$INSTALL_PREFIX/lib/mcp-arch-linux"
exec node src/server.js "\$@"
EOF
    
    chmod +x "$INSTALL_PREFIX/bin/mcp-arch-server"
    
    log_info "Application installed to $INSTALL_PREFIX"
}

install_config() {
    log_info "Installing configuration..."
    
    if [[ ! -f "$CONFIG_DIR/server.yaml" ]]; then
        cp config/server.yaml "$CONFIG_DIR/"
        log_info "Default configuration installed"
    else
        log_warn "Configuration file already exists, skipping"
    fi
    
    # Set permissions
    chmod 644 "$CONFIG_DIR/server.yaml"
}

install_systemd_service() {
    log_info "Installing systemd service..."
    
    cat > "$SYSTEMD_DIR/mcp-arch-linux.service" << EOF
[Unit]
Description=MCP Arch Linux Server
After=network.target
Requires=dbus.service
Documentation=https://github.com/Grow-Myelin/architect

[Service]
Type=simple
ExecStart=$INSTALL_PREFIX/bin/mcp-arch-server --config $CONFIG_DIR/server.yaml
Restart=always
RestartSec=10
TimeoutStopSec=20

# User/Group
User=$SERVICE_USER
Group=$SERVICE_USER

# Working directory
WorkingDirectory=$INSTALL_PREFIX/lib/mcp-arch-linux

# Environment
Environment=NODE_ENV=production
Environment=MCP_CONFIG_DIR=$CONFIG_DIR
Environment=MCP_LOG_DIR=$LOG_DIR
Environment=MCP_LIB_DIR=$LIB_DIR

# Security hardening
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
ProtectKernelTunables=yes
ProtectControlGroups=yes
ProtectKernelModules=yes
RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6
RestrictNamespaces=yes
LockPersonality=yes
RestrictRealtime=yes
RestrictSUIDSGID=yes
RemoveIPC=yes

# File system access
ReadWritePaths=$LOG_DIR $LIB_DIR /tmp
ReadOnlyPaths=$CONFIG_DIR

# Resource limits
LimitNOFILE=65536
MemoryMax=2G
CPUQuota=200%
TasksMax=512

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=mcp-arch-linux

[Install]
WantedBy=multi-user.target
EOF
    
    # Reload systemd
    systemctl daemon-reload
    
    log_info "Systemd service installed"
}

install_optional_dependencies() {
    log_info "Checking optional dependencies..."
    
    local missing_packages=()
    
    # Screen capture tools
    if ! command -v grim &> /dev/null; then
        missing_packages+=("grim")
    fi
    
    if ! command -v wf-recorder &> /dev/null; then
        missing_packages+=("wf-recorder")
    fi
    
    if ! command -v slurp &> /dev/null; then
        missing_packages+=("slurp")
    fi
    
    # System tools
    if ! command -v smartctl &> /dev/null; then
        missing_packages+=("smartmontools")
    fi
    
    if [[ ${#missing_packages[@]} -gt 0 ]]; then
        log_warn "Optional packages not installed: ${missing_packages[*]}"
        log_info "Install with: sudo pacman -S ${missing_packages[*]}"
    else
        log_info "All optional dependencies are available"
    fi
}

main() {
    echo "=========================================="
    echo "  MCP Arch Linux Server Installation"
    echo "=========================================="
    echo
    
    check_root
    check_node
    create_user
    create_directories
    install_dependencies
    install_application
    install_config
    install_systemd_service
    install_optional_dependencies
    
    echo
    echo "=========================================="
    echo -e "${GREEN}  Installation Complete!${NC}"
    echo "=========================================="
    echo
    echo "Next steps:"
    echo "1. Review configuration: $CONFIG_DIR/server.yaml"
    echo "2. Enable the service: systemctl enable mcp-arch-linux"
    echo "3. Start the service: systemctl start mcp-arch-linux"
    echo "4. Check status: systemctl status mcp-arch-linux"
    echo "5. View logs: journalctl -u mcp-arch-linux -f"
    echo
    echo "The server will be available at: http://localhost:8080"
    echo
}

# Check if script is being sourced or executed
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi