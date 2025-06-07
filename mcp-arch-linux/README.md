# MCP Arch Linux Server

A powerful MCP (Model Context Protocol) server for Arch Linux system control with Hyprland integration. This server allows AI assistants like Claude to interact with and control Arch Linux systems programmatically.

## Features

- **System Command Execution**: Execute privileged commands with proper security controls
- **Arch Linux Installation Automation**: Automate Arch Linux installation including partitioning, base system setup, and configuration
- **Hyprland Window Manager Integration**: Control windows, workspaces, and monitor layouts
- **Screen Capture**: Capture screenshots and recordings using Wayland-native tools
- **Security & Audit Logging**: Comprehensive audit trail and rollback capabilities
- **Plugin Architecture**: Extensible design for adding new functionality

## Architecture

The server is built in Rust for memory safety and performance, featuring:

- Async/await pattern using Tokio
- JSON-RPC 2.0 protocol implementation
- Capability-based security model
- Plugin system for modular functionality
- Comprehensive error handling and logging

## Installation

### Prerequisites

- Rust 1.70+ and Cargo
- Arch Linux system
- Hyprland (for window management features)
- Screen capture tools: `grim`, `wf-recorder` (optional)

### Building from Source

```bash
git clone https://github.com/yourusername/mcp-arch-linux
cd mcp-arch-linux
cargo build --release
```

### Installing as System Service

```bash
# Copy the binary
sudo cp target/release/mcp-arch-server /usr/local/bin/

# Copy systemd service file
sudo cp systemd/mcp-arch-linux.service /etc/systemd/system/

# Create required directories
sudo mkdir -p /var/log/mcp-arch-linux
sudo mkdir -p /var/lib/mcp-arch-linux/{snapshots,captures}

# Enable and start the service
sudo systemctl enable mcp-arch-linux
sudo systemctl start mcp-arch-linux
```

## Configuration

Configure the server via environment variables or the systemd service file:

- `MCP_BIND_ADDRESS`: Server bind address (default: `127.0.0.1:8080`)
- `MCP_MAX_CONCURRENT_OPS`: Maximum concurrent operations (default: 10)
- `MCP_REQUIRE_AUTH`: Require authentication (default: true)
- `MCP_AUDIT_LOG_PATH`: Path to audit log file
- `MCP_SNAPSHOTS_DIR`: Directory for system snapshots
- `MCP_CAPTURE_DIR`: Directory for screen captures

## Available Tools

### System Management
- `system_exec`: Execute system commands with privilege control
- `system_snapshot`: Create system state snapshots
- `system_rollback`: Rollback to a previous snapshot

### Arch Linux Installation
- `arch_install_partition`: Partition disks for Arch installation
- `arch_install_base`: Install Arch Linux base system
- `arch_install_configure`: Configure the installed system
- `arch_install_bootloader`: Install and configure bootloader

### Hyprland Control
- `hyprland_dispatch`: Execute Hyprland dispatcher commands
- `hyprland_keyword`: Set Hyprland configuration keywords
- `hyprland_window_info`: Get window information
- `hyprland_workspaces`: List workspaces
- `hyprland_monitors`: List monitors
- `hyprland_reload`: Reload Hyprland configuration

### Screen Capture
- `capture_screenshot`: Capture screenshots
- `capture_window`: Capture specific windows
- `start_recording`: Start screen recording
- `stop_recording`: Stop screen recording

## Security

The server implements multiple security layers:

1. **Capability-based permissions**: Only required Linux capabilities are retained
2. **Audit logging**: All operations are logged with full context
3. **Rollback support**: System changes can be reverted using snapshots
4. **Input validation**: All inputs are sanitized and validated
5. **Resource limits**: CPU, memory, and file descriptor limits enforced

## Development

### Running in Development Mode

```bash
# Set development environment
export RUST_LOG=mcp_arch_linux=debug

# Run the server
cargo run
```

### Running Tests

```bash
cargo test
```

### Adding New Plugins

1. Create a new module in `src/plugins/`
2. Implement the `MCPPlugin` trait
3. Register the plugin in `main.rs`

## Example Usage

Connect to the server using any MCP-compatible client:

```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "hyprland_workspaces",
    "arguments": {}
  },
  "id": 1
}
```

## License

MIT License - see LICENSE file for details

## Contributing

Contributions are welcome! Please read CONTRIBUTING.md for guidelines.

## Safety Warning

This server requires elevated privileges for many operations. Always:
- Run in a controlled environment
- Review audit logs regularly
- Use snapshots before major changes
- Implement proper authentication in production