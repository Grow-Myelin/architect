# MCP Arch Linux Server API Documentation

This document provides comprehensive API reference for the MCP Arch Linux Server.

## Table of Contents

1. [Protocol Overview](#protocol-overview)
2. [Authentication](#authentication)
3. [System Tools](#system-tools)
4. [Arch Installation Tools](#arch-installation-tools)
5. [Hyprland Tools](#hyprland-tools)
6. [Screen Capture Tools](#screen-capture-tools)
7. [Resources](#resources)
8. [Error Handling](#error-handling)

## Protocol Overview

The server implements MCP (Model Context Protocol) over JSON-RPC 2.0. All requests should be sent as POST to `/mcp` endpoint.

### Base URL
```
http://localhost:8080/mcp
```

### Request Format
```json
{
  "jsonrpc": "2.0",
  "method": "method_name",
  "params": {
    // method parameters
  },
  "id": 1
}
```

### Response Format
```json
{
  "jsonrpc": "2.0",
  "result": {
    // method result
  },
  "id": 1
}
```

## Authentication

Authentication is configurable via the `security.requireAuth` setting.

### Initialize
Initialize the MCP session:

```json
{
  "jsonrpc": "2.0",
  "method": "initialize",
  "params": {
    "protocolVersion": "2024-11-05",
    "capabilities": {},
    "clientInfo": {
      "name": "client-name",
      "version": "1.0.0"
    }
  },
  "id": 1
}
```

## System Tools

### system_exec

Execute system commands with security controls.

**Parameters:**
- `command` (string, required): Command to execute
- `args` (array, optional): Command arguments
- `requireRoot` (boolean, optional): Whether command requires root privileges
- `timeout` (integer, optional): Timeout in milliseconds

**Example:**
```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "system_exec",
    "arguments": {
      "command": "ls",
      "args": ["-la", "/home"],
      "requireRoot": false,
      "timeout": 10000
    }
  },
  "id": 1
}
```

### system_info

Get comprehensive system information.

**Parameters:**
- `detailed` (boolean, optional): Include detailed hardware information

**Example:**
```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "system_info",
    "arguments": {
      "detailed": true
    }
  },
  "id": 1
}
```

### system_services

Manage systemd services.

**Parameters:**
- `action` (string, required): Action to perform (`list`, `status`, `start`, `stop`, `restart`, `enable`, `disable`)
- `service` (string, optional): Service name (required for actions other than `list`)

**Example:**
```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "system_services",
    "arguments": {
      "action": "start",
      "service": "nginx"
    }
  },
  "id": 1
}
```

### system_package

Manage system packages using pacman.

**Parameters:**
- `action` (string, required): Package action (`update`, `upgrade`, `install`, `remove`, `search`, `info`)
- `packages` (array, optional): Package names
- `noconfirm` (boolean, optional): Skip confirmation prompts

**Example:**
```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "system_package",
    "arguments": {
      "action": "install",
      "packages": ["docker", "docker-compose"],
      "noconfirm": true
    }
  },
  "id": 1
}
```

### system_snapshot

Create a system state snapshot for rollback.

**Parameters:**
- `description` (string, required): Snapshot description
- `files` (array, optional): Files to include in snapshot

**Example:**
```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "system_snapshot",
    "arguments": {
      "description": "Before docker installation",
      "files": ["/etc/pacman.conf", "/etc/fstab"]
    }
  },
  "id": 1
}
```

### system_rollback

Rollback to a previous system snapshot.

**Parameters:**
- `snapshotId` (string, required): Snapshot ID to rollback to

**Example:**
```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "system_rollback",
    "arguments": {
      "snapshotId": "550e8400-e29b-41d4-a716-446655440000"
    }
  },
  "id": 1
}
```

## Arch Installation Tools

### arch_partition_disk

Partition a disk for Arch Linux installation.

**Parameters:**
- `device` (string, required): Device path (e.g., `/dev/sda`)
- `scheme` (string, required): Partition scheme (`uefi` or `bios`)
- `swapSize` (string, optional): Swap partition size (default: `4G`)
- `rootSize` (string, optional): Root partition size (default: `remaining`)
- `dryRun` (boolean, optional): Preview operations without executing

**Example:**
```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "arch_partition_disk",
    "arguments": {
      "device": "/dev/sdb",
      "scheme": "uefi",
      "swapSize": "8G",
      "dryRun": true
    }
  },
  "id": 1
}
```

### arch_install_base

Install Arch Linux base system.

**Parameters:**
- `target` (string, optional): Mount point for installation (default: `/mnt`)
- `packages` (array, optional): Additional packages to install
- `mirror` (string, optional): Pacman mirror URL

**Example:**
```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "arch_install_base",
    "arguments": {
      "target": "/mnt",
      "packages": ["base", "base-devel", "linux", "linux-firmware", "networkmanager", "vim", "git"],
      "mirror": "https://mirror.example.com/archlinux/$repo/os/$arch"
    }
  },
  "id": 1
}
```

### arch_configure_system

Configure the installed Arch Linux system.

**Parameters:**
- `hostname` (string, required): System hostname
- `timezone` (string, required): Timezone (e.g., `Europe/London`)
- `locale` (string, optional): System locale (default: `en_US.UTF-8`)
- `keymap` (string, optional): Console keymap (default: `us`)
- `users` (array, optional): Users to create

**Example:**
```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "arch_configure_system",
    "arguments": {
      "hostname": "archbox",
      "timezone": "America/New_York",
      "locale": "en_US.UTF-8",
      "users": [
        {
          "username": "admin",
          "groups": ["wheel", "docker"],
          "shell": "/bin/bash"
        }
      ]
    }
  },
  "id": 1
}
```

## Hyprland Tools

### hyprland_dispatch

Execute Hyprland dispatcher command.

**Parameters:**
- `dispatcher` (string, required): Dispatcher command
- `args` (string, optional): Dispatcher arguments

**Example:**
```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "hyprland_dispatch",
    "arguments": {
      "dispatcher": "workspace",
      "args": "3"
    }
  },
  "id": 1
}
```

### hyprland_windows

Get information about windows.

**Parameters:**
- `format` (string, optional): Output format (`json` or `text`, default: `json`)

**Example:**
```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "hyprland_windows",
    "arguments": {
      "format": "json"
    }
  },
  "id": 1
}
```

### hyprland_window_control

Control specific windows.

**Parameters:**
- `action` (string, required): Window action (`focus`, `move`, `resize`, `close`, `float`, `fullscreen`)
- `target` (string, optional): Window identifier or direction
- `args` (string, optional): Additional arguments for the action

**Example:**
```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "hyprland_window_control",
    "arguments": {
      "action": "resize",
      "args": "50 50"
    }
  },
  "id": 1
}
```

## Screen Capture Tools

### capture_screenshot

Capture a screenshot of the screen or specific area.

**Parameters:**
- `output` (string, optional): Output name (monitor) to capture
- `region` (object, optional): Specific region to capture
- `format` (string, optional): Image format (`png`, `jpg`, `webp`, default: `png`)
- `quality` (integer, optional): Image quality for lossy formats (1-100, default: 90)
- `filename` (string, optional): Custom filename (without extension)

**Example:**
```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "capture_screenshot",
    "arguments": {
      "region": {
        "x": 100,
        "y": 100,
        "width": 800,
        "height": 600
      },
      "format": "png",
      "filename": "my-screenshot"
    }
  },
  "id": 1
}
```

### capture_window

Capture a screenshot of a specific window.

**Parameters:**
- `selector` (string, optional): Window selector (default: `active`)
- `format` (string, optional): Image format (default: `png`)
- `filename` (string, optional): Custom filename (without extension)

**Example:**
```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "capture_window",
    "arguments": {
      "selector": "active",
      "format": "png"
    }
  },
  "id": 1
}
```

### start_recording

Start screen recording.

**Parameters:**
- `output` (string, optional): Output name to record
- `audio` (boolean, optional): Include audio in recording (default: false)
- `format` (string, optional): Video format (`mp4`, `webm`, `mkv`, default: `mp4`)
- `fps` (integer, optional): Frames per second (1-60, default: 30)
- `filename` (string, optional): Custom filename (without extension)

**Example:**
```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "start_recording",
    "arguments": {
      "audio": true,
      "format": "mp4",
      "fps": 60
    }
  },
  "id": 1
}
```

## Resources

Resources provide read-only access to system information.

### Available Resources

- `system://info` - System information
- `system://logs` - System logs
- `system://services` - Service status
- `system://snapshots` - System snapshots
- `system://processes` - Running processes
- `hyprland://config` - Hyprland configuration
- `hyprland://status` - Hyprland status
- `hyprland://layout` - Window layout
- `capture://list` - Capture list
- `capture://latest` - Latest capture
- `capture://status` - Capture status

### Reading Resources

```json
{
  "jsonrpc": "2.0",
  "method": "resources/read",
  "params": {
    "uri": "system://info"
  },
  "id": 1
}
```

## Error Handling

### Error Response Format

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32603,
    "message": "Internal error",
    "data": "Additional error information"
  },
  "id": 1
}
```

### Error Codes

- `-32700` - Parse error
- `-32600` - Invalid Request
- `-32601` - Method not found
- `-32602` - Invalid params
- `-32603` - Internal error
- `-32002` - Server not initialized
- `-31001` - Insufficient privileges
- `-30001` - Resource locked

### Example Error Responses

**Command not allowed:**
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -31001,
    "message": "Command not allowed: rm"
  },
  "id": 1
}
```

**Invalid parameters:**
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32602,
    "message": "Required argument missing: device"
  },
  "id": 1
}
```

## Rate Limiting

The server implements concurrent operation limits:

- Default maximum concurrent operations: 10
- Configurable via `security.maxConcurrentOperations`
- Operations that exceed the limit will receive a "Resource locked" error

## Timeouts

Default command timeout is 5 minutes (300,000ms), configurable via:
- Global: `security.commandTimeout`
- Per-request: `timeout` parameter in tool arguments

## Content Types

The server returns different content types in tool results:

### Text Content
```json
{
  "content": [
    {
      "type": "text",
      "text": "Command output or information"
    }
  ]
}
```

### Image Content
```json
{
  "content": [
    {
      "type": "image",
      "data": "base64-encoded-image-data",
      "mimeType": "image/png"
    }
  ]
}
```

### Resource Content
```json
{
  "content": [
    {
      "type": "resource",
      "uri": "system://info"
    }
  ]
}
```

This API provides comprehensive system control while maintaining security through proper validation, authentication, and audit logging.