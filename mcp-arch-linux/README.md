# MCP Arch Linux Server

A robust, production-ready MCP (Model Context Protocol) server for Arch Linux system control with Hyprland integration. Built in Node.js for stability, ease of deployment, and rapid development.

## 🚀 Features

### System Management
- **Package Management**: Install, update, and manage packages with pacman
- **Service Control**: Start, stop, enable, and monitor systemd services
- **Process Management**: List, monitor, and control system processes
- **System Information**: Comprehensive hardware and system status
- **Snapshot & Rollback**: Create system snapshots and rollback capabilities

### Arch Linux Installation
- **Automated Installation**: Complete Arch Linux installation automation
- **Disk Partitioning**: UEFI and BIOS partition schemes
- **Base System Setup**: Automated pacstrap and system configuration
- **Bootloader Installation**: GRUB and systemd-boot support
- **User Management**: Automated user creation and configuration

### Hyprland Integration
- **Window Management**: Control windows, workspaces, and layouts
- **IPC Communication**: Direct Hyprland socket communication
- **Configuration Control**: Dynamic Hyprland configuration updates
- **Monitor Management**: Multi-monitor setup and control
- **Real-time Status**: Live window and workspace information

### Screen Capture
- **Screenshots**: Full screen, window, or region capture
- **Screen Recording**: High-quality video recording with audio
- **Multiple Formats**: PNG, JPEG, WebP images; MP4, WebM videos
- **Interactive Selection**: User-driven area selection
- **File Management**: Built-in capture file organization

### Security & Reliability
- **Audit Logging**: Comprehensive operation tracking
- **Command Validation**: Whitelist-based command security
- **Resource Limits**: Concurrent operation and timeout controls
- **Snapshot System**: Automatic rollback capabilities
- **Privilege Management**: Minimal required permissions

## 📦 Installation

### Quick Setup (Recommended)

```bash
# Clone the repository
git clone https://github.com/Grow-Myelin/architect.git
cd architect/mcp-arch-linux

# Run the automated setup
./scripts/quick-setup.sh
```

This script will:
- Update your system
- Install Node.js and dependencies
- Build and install the MCP server
- Configure systemd service
- Start the server automatically

### Manual Installation

1. **Install Prerequisites**:
   ```bash
   sudo pacman -S nodejs npm git base-devel
   ```

2. **Install Dependencies**:
   ```bash
   npm install
   ```

3. **Install System-wide**:
   ```bash
   sudo ./scripts/install.sh
   ```

4. **Start Service**:
   ```bash
   sudo systemctl enable --now mcp-arch-linux
   ```

### Optional Dependencies

For full functionality, install these optional packages:

```bash
# Hyprland and window management
sudo pacman -S hyprland

# Screen capture tools
sudo pacman -S grim slurp wf-recorder

# System monitoring
sudo pacman -S smartmontools
```

## 🔧 Configuration

The server configuration is located at `/etc/mcp-arch-linux/server.yaml`:

```yaml
server:
  host: "localhost"
  port: 8080

security:
  requireAuth: false  # Enable for production
  allowedCommands:
    - "pacman"
    - "systemctl"
    # ... more commands

plugins:
  system:
    enabled: true
  archInstall:
    enabled: true
    allowDiskOperations: true
  hyprland:
    enabled: true
  screenCapture:
    enabled: true
    allowRecording: true
```

## 🚦 Usage

### Health Check

```bash
curl http://localhost:8080/health
```

### List Available Tools

```bash
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/list","id":1}'
```

### Execute a Tool

```bash
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc":"2.0",
    "method":"tools/call",
    "params":{
      "name":"system_info",
      "arguments":{"detailed":true}
    },
    "id":1
  }'
```

## 🛠️ Available Tools

### System Management
- `system_exec` - Execute system commands with security controls
- `system_info` - Get comprehensive system information
- `system_services` - Manage systemd services
- `system_package` - Package management with pacman
- `system_snapshot` - Create system snapshots
- `system_rollback` - Rollback to previous snapshots
- `system_process` - Process management

### Arch Installation
- `arch_partition_disk` - Partition disks for installation
- `arch_install_base` - Install Arch Linux base system
- `arch_configure_system` - Configure installed system
- `arch_install_bootloader` - Install bootloader
- `arch_mount_system` - Mount installation partitions
- `arch_list_disks` - List available disks
- `arch_installation_status` - Get installation progress

### Hyprland Control
- `hyprland_dispatch` - Execute Hyprland commands
- `hyprland_keyword` - Set configuration values
- `hyprland_windows` - Window information and control
- `hyprland_workspaces` - Workspace management
- `hyprland_monitors` - Monitor configuration
- `hyprland_layout` - Layout management
- `hyprland_window_control` - Advanced window control

### Screen Capture
- `capture_screenshot` - Take screenshots
- `capture_window` - Capture specific windows
- `capture_selection` - Interactive area selection
- `start_recording` - Begin screen recording
- `stop_recording` - End screen recording
- `list_captures` - List captured files
- `get_capture` - Retrieve capture files

## 🔗 Claude Code Integration

### Setup Claude Code

1. **Install Claude Code CLI**:
   ```bash
   npm install -g @anthropic/claude-code
   ```

2. **Configure MCP Server**:
   Create or edit `~/.config/claude-code/config.json`:
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
         ]
       }
     }
   }
   ```

3. **Start Claude Code**:
   ```bash
   claude --mcp arch-linux
   ```

### Example Interactions

Ask Claude Code to:

- **"Install docker and start the service"**
- **"Take a screenshot of the current window"**
- **"Switch to workspace 3 in Hyprland"**
- **"Create a system snapshot before updating"**
- **"Show me all running services"**
- **"Partition /dev/sdb for a new Arch installation"**

## 📊 Monitoring & Logs

### Service Status
```bash
sudo systemctl status mcp-arch-linux
```

### Live Logs
```bash
sudo journalctl -u mcp-arch-linux -f
```

### Audit Logs
```bash
sudo tail -f /var/log/mcp-arch-linux/audit-*.log
```

### Application Logs
```bash
sudo tail -f /var/log/mcp-arch-linux/app-*.log
```

## 🔒 Security

### Production Setup

1. **Enable Authentication**:
   ```yaml
   security:
     requireAuth: true
   ```

2. **Review Allowed Commands**:
   ```yaml
   security:
     allowedCommands:
       - "pacman"  # Only allow specific commands
   ```

3. **Set Resource Limits**:
   ```yaml
   security:
     maxConcurrentOperations: 5
     commandTimeout: 180000
   ```

4. **Configure Audit Logging**:
   ```yaml
   security:
     auditAll: true
   ```

### Security Features

- **Command Whitelisting**: Only pre-approved commands can be executed
- **Input Validation**: All parameters are validated and sanitized
- **Audit Logging**: Every operation is logged with full context
- **Resource Limits**: Protection against resource exhaustion
- **Privilege Separation**: Runs with minimal required permissions
- **Snapshot System**: Automatic rollback capabilities

## 🐛 Troubleshooting

### Server Won't Start

```bash
# Check logs
sudo journalctl -u mcp-arch-linux -e

# Common issues:
# - Node.js version too old
# - Port already in use
# - Permission issues
```

### Connection Issues

```bash
# Test local connection
curl http://localhost:8080/health

# Check firewall
sudo iptables -L

# Verify service is running
sudo systemctl is-active mcp-arch-linux
```

### Hyprland Features Not Working

```bash
# Check if Hyprland is running
echo $HYPRLAND_INSTANCE_SIGNATURE

# Verify socket exists
ls $XDG_RUNTIME_DIR/hypr/*/
```

### Permission Denied Errors

```bash
# Check service user permissions
sudo -u mcp-server ls /var/lib/mcp-arch-linux

# Review audit logs for security violations
sudo grep "DENIED" /var/log/mcp-arch-linux/audit-*.log
```

## 📁 Directory Structure

```
mcp-arch-linux/
├── src/
│   ├── core/              # Core MCP server implementation
│   ├── plugins/           # Feature plugins
│   ├── system/            # System integration
│   └── security/          # Security and audit
├── config/
│   └── server.yaml        # Default configuration
├── scripts/
│   ├── install.sh         # System installation
│   └── quick-setup.sh     # Automated setup
├── tests/                 # Test suites
└── docs/                  # Additional documentation
```

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Submit a pull request

### Development Setup

```bash
# Install development dependencies
npm install

# Run in development mode
npm run dev

# Run tests
npm test

# Lint code
npm run lint
```

## 📄 License

MIT License - see [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built for the [MCP Protocol](https://modelcontextprotocol.io/)
- Integrates with [Hyprland](https://hyprland.org/)
- Designed for [Claude Code](https://claude.ai/code)
- Optimized for [Arch Linux](https://archlinux.org/)

---

**Note**: This server requires elevated privileges for many operations. Always review the security configuration and audit logs in production environments.