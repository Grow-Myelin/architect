# Complete Setup Guide: Fresh Arch Linux to Claude Code with MCP

This guide will walk you through setting up the MCP Arch Linux server on a fresh Arch installation to enable Claude Code to control your system.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Quick Setup (Recommended)](#quick-setup-recommended)
3. [Manual Setup](#manual-setup)
4. [Claude Code Configuration](#claude-code-configuration)
5. [Testing the Setup](#testing-the-setup)
6. [Security Configuration](#security-configuration)
7. [Troubleshooting](#troubleshooting)

## Prerequisites

You should have:
- Fresh Arch Linux installation with network connectivity
- Root access or sudo privileges
- Basic terminal knowledge

## Quick Setup (Recommended)

### Step 1: Clone and Setup

```bash
# Update system
sudo pacman -Syu

# Clone the repository
git clone https://github.com/Grow-Myelin/architect.git
cd architect/mcp-arch-linux

# Run automated setup
./scripts/quick-setup.sh
```

The script will:
- Install Node.js and dependencies
- Build the MCP server
- Install system-wide with proper permissions
- Configure and start systemd service
- Test the installation

### Step 2: Verify Installation

```bash
# Check service status
sudo systemctl status mcp-arch-linux

# Test server response
curl http://localhost:8080/health

# View available tools
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/list","id":1}'
```

## Manual Setup

If you prefer manual installation or the quick setup fails:

### Step 1: Install Prerequisites

```bash
# Update system
sudo pacman -Syu

# Install required packages
sudo pacman -S nodejs npm git base-devel

# Install optional packages for full functionality
sudo pacman -S hyprland grim slurp wf-recorder smartmontools
```

### Step 2: Build Application

```bash
cd architect/mcp-arch-linux

# Install Node.js dependencies
npm install

# Test the application
node src/server.js --help
```

### Step 3: Install System-wide

```bash
# Run installation script
sudo ./scripts/install.sh

# The script will:
# - Create service user
# - Install to /usr/local
# - Create configuration files
# - Setup systemd service
```

### Step 4: Configure and Start Service

```bash
# Review configuration
sudo nano /etc/mcp-arch-linux/server.yaml

# Enable and start service
sudo systemctl enable mcp-arch-linux
sudo systemctl start mcp-arch-linux

# Check status
sudo systemctl status mcp-arch-linux
```

## Claude Code Configuration

### Install Claude Code

On your development machine (can be the same Arch machine or a different one):

```bash
# Install via npm
npm install -g @anthropic/claude-code

# Or download binary from GitHub releases
# https://github.com/anthropics/claude-code/releases
```

### Configure MCP Server

Create or edit Claude Code configuration:

```bash
mkdir -p ~/.config/claude-code
nano ~/.config/claude-code/config.json
```

Add the MCP server configuration:

```json
{
  "mcpServers": {
    "arch-linux": {
      "command": "curl",
      "args": [
        "-X", "POST",
        "http://localhost:8080/mcp",
        "-H", "Content-Type: application/json",
        "-d", "@-"
      ],
      "env": {}
    }
  }
}
```

### For Remote Setup

If Claude Code is on a different machine:

```json
{
  "mcpServers": {
    "arch-linux": {
      "command": "ssh",
      "args": [
        "user@your-arch-machine",
        "curl -X POST http://localhost:8080/mcp -H 'Content-Type: application/json' -d @-"
      ],
      "env": {}
    }
  }
}
```

## Testing the Setup

### Basic Connectivity Test

```bash
# Health check
curl http://localhost:8080/health

# Should return something like:
# {
#   "status": "healthy",
#   "version": "1.0.0",
#   "timestamp": "2024-01-01T00:00:00.000Z",
#   "plugins": [...]
# }
```

### Test MCP Protocol

```bash
# List available tools
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/list",
    "id": 1
  }'

# Execute a simple command
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "system_info",
      "arguments": {}
    },
    "id": 1
  }'
```

### Test Claude Code Integration

```bash
# Start Claude Code with MCP
claude --mcp arch-linux

# In Claude Code, try these commands:
# "Show me system information"
# "List all systemd services"
# "Take a screenshot"
```

## Security Configuration

### Production Security Settings

Edit `/etc/mcp-arch-linux/server.yaml`:

```yaml
security:
  requireAuth: true
  allowedCommands:
    # Only allow specific commands you need
    - "pacman"
    - "systemctl"
    - "hyprctl"
  maxConcurrentOperations: 5
  commandTimeout: 180000  # 3 minutes
  auditAll: true

server:
  host: "127.0.0.1"  # Only local connections
  port: 8080
```

### Network Security

If exposing to network:

```bash
# Use SSH tunneling instead of direct exposure
ssh -L 8080:localhost:8080 user@arch-machine

# Or setup reverse proxy with authentication
# (nginx, apache, etc.)
```

### User Permissions

The service runs as `mcp-server` user with minimal permissions:

```bash
# Check service user
id mcp-server

# Review systemd security settings
sudo systemctl show mcp-arch-linux | grep -E "(User|Group|NoNewPrivileges|ProtectSystem)"
```

## Advanced Configuration

### Custom Plugin Configuration

```yaml
plugins:
  system:
    enabled: true
    snapshotDir: "/var/lib/mcp-arch-linux/snapshots"
  
  archInstall:
    enabled: true
    allowDiskOperations: false  # Disable for security
  
  hyprland:
    enabled: true
    socketPath: "/run/user/1000/hypr/instance/socket.sock"
  
  screenCapture:
    enabled: true
    captureDir: "/var/lib/mcp-arch-linux/captures"
    maxFileSize: "100MB"
    allowRecording: false  # Disable recording
```

### Logging Configuration

```yaml
logging:
  level: "info"  # debug, info, warn, error
  logDir: "/var/log/mcp-arch-linux"
  maxFiles: "30d"
  maxSize: "50m"
```

### Environment Variables

You can also configure via environment variables:

```bash
# Add to systemd service or shell profile
export MCP_HOST=localhost
export MCP_PORT=8080
export MCP_LOG_LEVEL=debug
export MCP_REQUIRE_AUTH=true
```

## Troubleshooting

### Service Won't Start

```bash
# Check systemd logs
sudo journalctl -u mcp-arch-linux -e

# Common issues:
sudo systemctl status mcp-arch-linux

# Check if port is in use
sudo ss -tlnp | grep 8080

# Verify Node.js version
node --version  # Should be >= 18.0.0
```

### Permission Errors

```bash
# Check file permissions
ls -la /var/log/mcp-arch-linux/
ls -la /var/lib/mcp-arch-linux/

# Fix ownership if needed
sudo chown -R mcp-server:mcp-server /var/lib/mcp-arch-linux/
sudo chown -R mcp-server:mcp-server /var/log/mcp-arch-linux/
```

### Hyprland Integration Issues

```bash
# Check if Hyprland is running
echo $HYPRLAND_INSTANCE_SIGNATURE
echo $XDG_RUNTIME_DIR

# Verify socket exists
ls $XDG_RUNTIME_DIR/hypr/*/

# Test Hyprland commands directly
hyprctl version
```

### Screen Capture Not Working

```bash
# Install missing tools
sudo pacman -S grim slurp wf-recorder

# Test tools directly
grim /tmp/test.png
slurp
```

### Claude Code Connection Issues

```bash
# Test MCP server directly
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}'

# Check Claude Code configuration
cat ~/.config/claude-code/config.json

# Test SSH connection (if using remote)
ssh user@arch-machine echo "test"
```

### High CPU/Memory Usage

```bash
# Check resource usage
systemctl status mcp-arch-linux
journalctl -u mcp-arch-linux --since "1 hour ago"

# Adjust resource limits in systemd service
sudo systemctl edit mcp-arch-linux

# Add override:
[Service]
MemoryMax=1G
CPUQuota=100%
```

## Example Use Cases

Once everything is set up, you can ask Claude Code to:

### System Administration
- "Update all packages and restart services that need it"
- "Show me disk usage and clean up if needed"
- "Create a system snapshot before making changes"
- "Monitor system performance for the next minute"

### Window Management (Hyprland)
- "Switch to workspace 3 and open a terminal"
- "Take a screenshot of the current window"
- "Move the active window to monitor 2"
- "Show me all open windows and their positions"

### Development Workflow
- "Install docker and configure it for my user"
- "Set up a new development environment"
- "Create a backup before updating my system"
- "Monitor logs while I test my application"

### Arch Linux Installation
- "Partition /dev/sdb for a new Arch installation"
- "Install Arch Linux with my preferred configuration"
- "Set up dual boot with GRUB"
- "Create a minimal Arch installation for testing"

## Support and Documentation

- **View logs**: `sudo journalctl -u mcp-arch-linux -f`
- **Configuration**: `/etc/mcp-arch-linux/server.yaml`
- **Source code**: [GitHub Repository](https://github.com/Grow-Myelin/architect)
- **Report issues**: [GitHub Issues](https://github.com/Grow-Myelin/architect/issues)

The MCP server provides comprehensive system control while maintaining security through proper privilege separation, audit logging, and configurable restrictions.