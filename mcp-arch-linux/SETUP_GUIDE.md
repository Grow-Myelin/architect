# Complete Setup Guide: Fresh Arch Linux to Claude Code with MCP

This guide will walk you through setting up the MCP Arch Linux server on a fresh Arch installation to enable Claude Code to control your system.

## Prerequisites

You should have:
- Fresh Arch Linux installation with network connectivity
- Root access or sudo privileges
- Basic terminal knowledge

## Step 1: Update System and Install Essential Packages

```bash
# Update system
sudo pacman -Syu

# Install essential development tools
sudo pacman -S base-devel git vim

# Install Rust (required for building the MCP server)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install Hyprland and related tools (if you want window management features)
sudo pacman -S hyprland waybar wofi grim slurp wf-recorder jq

# Install additional dependencies
sudo pacman -S dbus systemd-libs
```

## Step 2: Clone and Build the MCP Server

```bash
# Clone the repository (you'll need to transfer the code to your Arch system)
# Option 1: If you have the code in a git repository:
git clone https://github.com/yourusername/mcp-arch-linux
cd mcp-arch-linux

# Option 2: If you need to transfer from another machine:
# On your development machine:
tar -czf mcp-arch-linux.tar.gz mcp-arch-linux/
# Transfer via USB or network, then on Arch:
tar -xzf mcp-arch-linux.tar.gz
cd mcp-arch-linux

# Build the server
cargo build --release
```

## Step 3: Install the MCP Server

```bash
# Run the installation script
sudo ./install.sh

# Or manually:
sudo cp target/release/mcp-arch-server /usr/local/bin/
sudo cp systemd/mcp-arch-linux.service /etc/systemd/system/
sudo mkdir -p /var/log/mcp-arch-linux
sudo mkdir -p /var/lib/mcp-arch-linux/{snapshots,captures}
```

## Step 4: Configure the MCP Server

```bash
# Edit the configuration (optional)
sudo vim /etc/mcp-arch-linux/config.toml

# For development/testing, you might want to disable auth requirement:
# Change: require_auth = true
# To:     require_auth = false

# If running without root, adjust the service file:
sudo vim /etc/systemd/system/mcp-arch-linux.service
# Comment out the capability lines if not running as root
```

## Step 5: Start the MCP Server

```bash
# Enable and start the service
sudo systemctl daemon-reload
sudo systemctl enable mcp-arch-linux
sudo systemctl start mcp-arch-linux

# Check if it's running
sudo systemctl status mcp-arch-linux

# View logs
sudo journalctl -u mcp-arch-linux -f
```

## Step 6: Set up Claude Code on Your Development Machine

On your development machine (not the Arch system):

### Install Claude Code CLI

```bash
# Install via npm
npm install -g @anthropic/claude-code

# Or download the binary from:
# https://github.com/anthropics/claude-code/releases
```

### Configure Claude Code to Use MCP

Create or edit `~/.config/claude-code/config.json`:

```json
{
  "mcpServers": {
    "arch-linux": {
      "command": "ssh",
      "args": [
        "user@your-arch-machine",
        "nc localhost 8080"
      ],
      "env": {}
    }
  }
}
```

Or if running locally:

```json
{
  "mcpServers": {
    "arch-linux": {
      "command": "nc",
      "args": ["localhost", "8080"],
      "env": {}
    }
  }
}
```

## Step 7: Test the Connection

1. On your Arch machine, ensure the MCP server is running:
```bash
sudo systemctl status mcp-arch-linux
```

2. Test basic connectivity:
```bash
# On Arch machine
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"initialize","params":{"protocol_version":"2024-11-05","capabilities":{},"client_info":{"name":"test","version":"1.0"}},"id":1}'
```

3. Use Claude Code:
```bash
# On your development machine
claude --mcp arch-linux

# Example commands to Claude:
# "List all Hyprland workspaces"
# "Take a screenshot"
# "Show system information"
```

## Step 8: Configure SSH Access (if using remote connection)

If Claude Code is on a different machine:

```bash
# On your development machine
ssh-keygen -t ed25519 -C "claude-code-mcp"
ssh-copy-id user@your-arch-machine

# Test SSH connection
ssh user@your-arch-machine
```

## Security Considerations

1. **Network Security**: By default, the server only listens on localhost. To allow remote connections:
   - Edit the systemd service file
   - Change `MCP_BIND_ADDRESS=127.0.0.1:8080` to `MCP_BIND_ADDRESS=0.0.0.0:8080`
   - Use SSH tunneling for security

2. **Permissions**: The server runs with limited capabilities. For full system control:
   - Ensure the service user has necessary permissions
   - Or run as root (less secure)

3. **Audit Logs**: Monitor operations at:
   ```bash
   sudo tail -f /var/log/mcp-arch-linux/audit.log
   ```

## Troubleshooting

### Server Won't Start
```bash
# Check logs
sudo journalctl -u mcp-arch-linux -e

# Common issues:
# - Missing Rust/cargo
# - Permission issues with directories
# - Port already in use
```

### Connection Issues
```bash
# Test local connection
telnet localhost 8080

# Check firewall
sudo iptables -L

# For Hyprland features, ensure you're in a Hyprland session
echo $HYPRLAND_INSTANCE_SIGNATURE
```

### Permission Denied Errors
```bash
# The server may need additional permissions for certain operations
# Either run as root or grant specific capabilities:
sudo setcap cap_sys_admin+ep /usr/local/bin/mcp-arch-server
```

## Example Usage

Once everything is set up, you can tell Claude Code to:

1. **System Management**:
   - "Update all packages on the system"
   - "Show me running services"
   - "Create a system snapshot before making changes"

2. **Hyprland Control**:
   - "Move the current window to workspace 2"
   - "List all open windows"
   - "Change Hyprland gaps to 10 pixels"

3. **Screen Capture**:
   - "Take a screenshot of the current screen"
   - "Start recording the screen"
   - "Capture the active window"

4. **Arch Installation** (for setting up new systems):
   - "Partition /dev/sda for UEFI installation"
   - "Install Arch base system to /mnt"
   - "Configure the system with hostname 'archbox'"

## Next Steps

1. Review the security configuration in `/etc/mcp-arch-linux/config.toml`
2. Set up authentication if exposing to network
3. Create custom plugins for your specific needs
4. Monitor audit logs regularly

The MCP server is now ready to accept commands from Claude Code!