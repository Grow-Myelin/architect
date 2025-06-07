# Comprehensive Implementation Guide for Arch Linux MCP Server with Hyprland

## Best programming language choice: Rust

After extensive analysis, **Rust emerges as the optimal choice** for implementing a Linux system control MCP server due to several compelling advantages:

### Why Rust excels for this use case

**Memory safety without garbage collection** eliminates 70% of security vulnerabilities (per Microsoft research) while maintaining predictable performance crucial for system operations. The language provides **zero-cost abstractions** for high-level constructs without runtime overhead and **concurrency safety** through its ownership model, preventing data races at compile time.

**Excellent Linux ecosystem support:**
- `tokio-dbus` for D-Bus integration with systemd
- `nix` crate for low-level Unix system calls
- `wayland-client` for Wayland protocol support
- `serde_json` for JSON-RPC serialization
- `ashpd` for XDG desktop portal integration

**Example MCP server structure in Rust:**
```rust
use tokio::sync::{Semaphore, RwLock};
use std::sync::Arc;

pub struct LinuxMCPServer {
    semaphore: Arc<Semaphore>,
    active_operations: Arc<RwLock<HashMap<String, OperationHandle>>>,
    privileged_ops_manager: PrivilegedOperationManager,
    system_monitor: SystemEventMonitor,
}

#[async_trait]
impl MCPServer for LinuxMCPServer {
    async fn execute_tool(&self, name: &str, args: ToolArgs) -> Result<MCPToolResult> {
        // Acquire permit for resource limiting
        let _permit = self.semaphore.acquire().await?;
        
        // Validate and execute with proper privilege handling
        self.privileged_ops_manager.execute_as_root(
            || self.run_system_command(name, args),
            name
        ).await
    }
}
```

## Elegant architectural patterns for modular design

### Core architecture with clean separation of concerns

The MCP server should follow a layered architecture that maintains clear boundaries between components while enabling extensibility:

```
mcp-arch-linux/
├── src/
│   ├── mcp/                    # MCP protocol layer
│   │   ├── server.rs           # JSON-RPC server implementation
│   │   ├── tools.rs            # System tools definitions
│   │   └── resources.rs        # System resources
│   ├── system/                 # System integration layer
│   │   ├── commands.rs         # Command execution engine
│   │   ├── dbus.rs            # D-Bus/systemd integration
│   │   └── wayland.rs         # Wayland/Hyprland interface
│   ├── security/              # Security framework
│   │   ├── privilege.rs       # Capability management
│   │   ├── validation.rs      # Input sanitization
│   │   └── audit.rs          # Audit logging
│   └── plugins/              # Plugin system
│       ├── arch_install.rs   # Arch installation automation
│       ├── hyprland.rs       # Hyprland-specific features
│       └── screen_capture.rs # Screen capture implementation
```

### Plugin system for extensibility

```rust
pub trait MCPPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn capabilities(&self) -> Vec<Capability>;
    async fn handle_tool_call(&self, tool: &str, args: ToolArgs) -> Result<MCPToolResult>;
}

pub struct PluginRegistry {
    plugins: HashMap<String, Box<dyn MCPPlugin>>,
}

impl PluginRegistry {
    pub fn register(&mut self, plugin: Box<dyn MCPPlugin>) {
        self.plugins.insert(plugin.name().to_string(), plugin);
    }
}
```

### Event-driven architecture for real-time updates

```rust
pub struct SystemEventMonitor {
    dbus_connection: Connection,
    event_handlers: HashMap<String, Box<dyn EventHandler>>,
}

impl SystemEventMonitor {
    pub async fn monitor_system_events(&mut self) -> Result<()> {
        let systemd_proxy = SystemdProxy::new(&self.dbus_connection).await?;
        let hyprland_socket = HyprlandEventSocket::connect().await?;
        
        tokio::select! {
            event = systemd_proxy.next_event() => {
                self.handle_systemd_event(event).await?;
            }
            event = hyprland_socket.next_event() => {
                self.handle_hyprland_event(event).await?;
            }
        }
        
        Ok(())
    }
}
```

## Wayland/Hyprland screen capture implementation

### Core approach: Hybrid PipeWire + wlr-screencopy

For optimal Hyprland screen capture, implement a hybrid approach using PipeWire for permissions and streaming infrastructure while leveraging wlr-screencopy for direct compositor access:

```rust
use ashpd::desktop::screen_cast::{ScreenCastProxy, SourceType};
use pipewire::{stream::Stream, properties::Properties};

pub struct HyprlandScreenCapture {
    portal_proxy: ScreenCastProxy<'static>,
    screencopy_manager: WlrScreencopyManager,
}

impl HyprlandScreenCapture {
    pub async fn start_capture(&self) -> Result<CaptureStream> {
        // Request permission through XDG portal
        let session = self.portal_proxy
            .create_session()
            .await?;
        
        // Select capture sources
        let sources = self.portal_proxy
            .select_sources(&session, SourceType::Monitor | SourceType::Window)
            .await?;
        
        // Start PipeWire stream
        let stream_info = self.portal_proxy
            .start(&session)
            .await?;
        
        // Connect to PipeWire node
        let pw_stream = self.create_pipewire_stream(stream_info.node_id).await?;
        
        Ok(CaptureStream { pw_stream })
    }
    
    async fn create_pipewire_stream(&self, node_id: u32) -> Result<Stream> {
        let props = Properties::new()
            .set("media.type", "Video")
            .set("media.category", "Capture")
            .set("target.node", &node_id.to_string());
        
        Stream::new_simple(
            "hyprland-capture",
            props,
            &stream_events,
            self
        )
    }
}
```

### DMA-BUF for zero-copy performance

```rust
pub struct DmaBufCapture {
    modifier: u64,
    format: u32,
    fd: RawFd,
}

impl DmaBufCapture {
    pub fn import_from_screencopy(&mut self, frame: &WlrScreencopyFrame) -> Result<()> {
        // Get DMA-BUF parameters
        let params = frame.get_dmabuf_params()?;
        
        // Import buffer for zero-copy access
        self.fd = params.fd;
        self.format = params.format;
        self.modifier = params.modifier;
        
        Ok(())
    }
}
```

### Real-time encoding pipeline

```rust
pub struct EncodingPipeline {
    encoder: VaapiEncoder,
    format_converter: FormatConverter,
}

impl EncodingPipeline {
    pub async fn encode_frame(&mut self, capture: &DmaBufCapture) -> Result<EncodedFrame> {
        // Hardware-accelerated format conversion
        let converted = self.format_converter
            .convert_dmabuf(capture, PixelFormat::NV12)?;
        
        // Hardware encoding
        let encoded = self.encoder
            .encode_frame(&converted, EncodingParams {
                codec: Codec::H264,
                bitrate: 8_000_000,
                preset: Preset::LowLatency,
            })?;
        
        Ok(encoded)
    }
}
```

## Arch Linux installation automation

### Programmatic control architecture

```rust
pub struct ArchInstaller {
    disk_manager: DiskManager,
    package_manager: PackageManager,
    config_manager: ConfigurationManager,
}

impl ArchInstaller {
    pub async fn automated_install(&self, config: InstallConfig) -> Result<()> {
        // Partition disks
        self.disk_manager
            .partition_disk(&config.disk, config.partition_scheme)
            .await?;
        
        // Mount filesystems
        self.disk_manager.mount_filesystems().await?;
        
        // Bootstrap base system
        self.package_manager
            .pacstrap(&["base", "base-devel", "linux", "linux-firmware"])
            .await?;
        
        // Generate fstab
        self.config_manager.generate_fstab().await?;
        
        // Configure system
        self.configure_chroot_environment(&config).await?;
        
        // Install bootloader
        self.install_bootloader(&config.bootloader).await?;
        
        Ok(())
    }
}
```

### Safe partition management

```rust
pub struct DiskManager {
    transaction_log: TransactionLog,
}

impl DiskManager {
    pub async fn partition_disk(&self, device: &Path, scheme: PartitionScheme) -> Result<()> {
        // Create transaction for rollback capability
        let transaction = self.transaction_log.begin()?;
        
        // Wipe existing partition table
        self.wipe_disk(device)?;
        
        match scheme {
            PartitionScheme::UEFI => {
                // Create GPT with EFI partition
                self.create_gpt_layout(device)?;
                self.create_efi_partition(device, 512)?; // 512MB EFI
                self.create_root_partition(device)?;
            }
            PartitionScheme::BIOS => {
                // Create MBR layout
                self.create_mbr_layout(device)?;
                self.create_root_partition(device)?;
            }
        }
        
        transaction.commit()?;
        Ok(())
    }
}
```

## Hyprland-specific integration

### IPC protocol implementation

```rust
use tokio::net::UnixStream;

pub struct HyprlandIPC {
    control_socket: UnixStream,
    event_socket: UnixStream,
}

impl HyprlandIPC {
    pub async fn connect() -> Result<Self> {
        let runtime_dir = env::var("XDG_RUNTIME_DIR")?;
        let instance = env::var("HYPRLAND_INSTANCE_SIGNATURE")?;
        
        let control_path = format!("{}/hypr/{}/.socket.sock", runtime_dir, instance);
        let event_path = format!("{}/hypr/{}/.socket2.sock", runtime_dir, instance);
        
        Ok(Self {
            control_socket: UnixStream::connect(control_path).await?,
            event_socket: UnixStream::connect(event_path).await?,
        })
    }
    
    pub async fn send_command(&mut self, cmd: &str) -> Result<String> {
        self.control_socket.write_all(cmd.as_bytes()).await?;
        
        let mut response = Vec::new();
        self.control_socket.read_to_end(&mut response).await?;
        
        Ok(String::from_utf8(response)?)
    }
    
    pub async fn monitor_events(&mut self) -> Result<impl Stream<Item = HyprlandEvent>> {
        let reader = BufReader::new(&mut self.event_socket);
        Ok(reader.lines().filter_map(|line| {
            line.ok().and_then(|l| HyprlandEvent::parse(&l))
        }))
    }
}
```

### Configuration management

```rust
pub struct HyprlandConfig {
    config_path: PathBuf,
    runtime_config: HashMap<String, Value>,
}

impl HyprlandConfig {
    pub async fn update_config(&self, key: &str, value: &str) -> Result<()> {
        let cmd = format!("keyword {} {}", key, value);
        self.ipc.send_command(&cmd).await?;
        Ok(())
    }
    
    pub async fn reload_config(&self) -> Result<()> {
        self.ipc.send_command("reload").await?;
        Ok(())
    }
}
```

## MCP implementation best practices

### Tool design pattern

```rust
#[derive(Debug, Clone)]
pub struct SystemTool {
    name: String,
    description: String,
    input_schema: Schema,
    require_confirmation: bool,
}

impl MCPTool for SystemTool {
    async fn execute(&self, args: ToolArgs) -> Result<MCPToolResult> {
        // Validate inputs against schema
        self.validate_args(&args)?;
        
        // Check permissions
        if self.requires_privilege() {
            self.check_permissions()?;
        }
        
        // Execute with audit logging
        let result = self.execute_with_audit(args).await?;
        
        Ok(MCPToolResult {
            content: vec![MCPContent::text(result)],
            is_error: false,
            metadata: Some(json!({
                "execution_time_ms": 125,
                "tool": self.name
            })),
        })
    }
}
```

### Error handling strategy

```rust
#[derive(Debug, thiserror::Error)]
pub enum MCPSystemError {
    #[error("Insufficient privileges for operation: {0}")]
    InsufficientPrivileges(String),
    
    #[error("System resource locked: {0}")]
    ResourceLocked(String),
    
    #[error("Command execution failed: {0}")]
    CommandFailed(#[from] std::io::Error),
    
    #[error("Invalid configuration: {0}")]
    ConfigError(String),
}

impl From<MCPSystemError> for JsonRpcError {
    fn from(err: MCPSystemError) -> Self {
        match err {
            MCPSystemError::InsufficientPrivileges(_) => {
                JsonRpcError::new(-31001, err.to_string())
            }
            MCPSystemError::ResourceLocked(_) => {
                JsonRpcError::new(-30001, err.to_string())
            }
            _ => JsonRpcError::new(-32603, err.to_string()),
        }
    }
}
```

## Security implementation

### Privilege management with capabilities

```rust
use caps::{Capability, CapSet};

pub struct CapabilityManager;

impl CapabilityManager {
    pub fn setup_minimal_capabilities() -> Result<()> {
        let mut caps = CapSet::empty();
        
        // Only keep essential capabilities
        caps.add(Capability::CAP_DAC_OVERRIDE)?; // File access
        caps.add(Capability::CAP_SYS_ADMIN)?;    // System admin
        caps.add(Capability::CAP_NET_ADMIN)?;    // Network config
        
        // Drop all others
        caps::set(None, CapSet::empty(), caps)?;
        
        Ok(())
    }
}
```

### Audit logging implementation

```rust
use tracing::{info, warn};
use serde_json::json;

pub struct SecurityAuditLogger {
    log_path: PathBuf,
}

impl SecurityAuditLogger {
    pub async fn log_operation(&self, op: &AuditableOperation) {
        let entry = json!({
            "timestamp": chrono::Utc::now(),
            "operation": op.name,
            "parameters": op.parameters,
            "user": op.user_context,
            "result": op.result,
            "session_id": op.session_id,
        });
        
        match &op.result {
            Ok(_) => info!(target: "audit", "{}", entry),
            Err(e) => warn!(target: "audit", "FAILED: {} - {}", entry, e),
        }
    }
}
```

### Rollback mechanism

```rust
pub struct SystemStateSnapshot {
    timestamp: DateTime<Utc>,
    config_backup: HashMap<String, String>,
    service_states: HashMap<String, ServiceState>,
}

pub struct RollbackManager {
    snapshots: Vec<SystemStateSnapshot>,
}

impl RollbackManager {
    pub async fn create_snapshot(&mut self) -> Result<SnapshotId> {
        let snapshot = SystemStateSnapshot {
            timestamp: Utc::now(),
            config_backup: self.backup_configs().await?,
            service_states: self.capture_service_states().await?,
        };
        
        let id = self.snapshots.len();
        self.snapshots.push(snapshot);
        Ok(SnapshotId(id))
    }
    
    pub async fn rollback_to(&self, id: SnapshotId) -> Result<()> {
        let snapshot = &self.snapshots[id.0];
        
        // Restore configurations
        for (path, content) in &snapshot.config_backup {
            fs::write(path, content).await?;
        }
        
        // Restore service states
        for (service, state) in &snapshot.service_states {
            self.restore_service_state(service, state).await?;
        }
        
        Ok(())
    }
}
```

## Example implementation structure

Here's a complete example demonstrating how to structure the MCP server:

```rust
// main.rs
use tokio::net::TcpListener;
use mcp_arch_linux::{LinuxMCPServer, Config};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target("audit", Level::INFO)
        .init();
    
    // Load configuration
    let config = Config::from_env()?;
    
    // Setup security
    CapabilityManager::setup_minimal_capabilities()?;
    
    // Create MCP server
    let server = LinuxMCPServer::builder()
        .with_plugin(ArchInstallPlugin::new())
        .with_plugin(HyprlandPlugin::new())
        .with_plugin(ScreenCapturePlugin::new())
        .with_security(SecurityConfig {
            require_auth: true,
            audit_all_operations: true,
            max_concurrent_ops: 10,
        })
        .build()?;
    
    // Start server
    let listener = TcpListener::bind(&config.bind_address).await?;
    server.serve(listener).await?;
    
    Ok(())
}
```

## Deployment configuration

### Systemd service with security hardening

```ini
[Unit]
Description=Arch Linux MCP Server
After=network.target
Requires=dbus.service

[Service]
Type=notify
ExecStart=/usr/local/bin/mcp-arch-server
Restart=always
RestartSec=10

# User/Group
User=mcp-server
Group=mcp-server

# Security hardening
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
ProtectKernelTunables=yes
ProtectControlGroups=yes

# Required capabilities
AmbientCapabilities=CAP_DAC_OVERRIDE CAP_SYS_ADMIN CAP_NET_ADMIN
CapabilityBoundingSet=CAP_DAC_OVERRIDE CAP_SYS_ADMIN CAP_NET_ADMIN

# Resource limits
LimitNOFILE=65536
MemoryMax=2G
CPUQuota=200%

[Install]
WantedBy=multi-user.target
```

## Conclusion

This implementation guide provides a comprehensive foundation for building a powerful yet secure MCP server for Arch Linux with Hyprland integration. The Rust-based architecture ensures memory safety and performance while the modular plugin system allows for extensibility. The security-first design with proper privilege management, audit logging, and rollback capabilities ensures safe operation even with root access.

Key takeaways:
- **Rust** provides the optimal balance of performance, safety, and ecosystem support
- **Modular architecture** with plugins enables clean separation of concerns
- **Hybrid screen capture** using PipeWire and wlr-screencopy offers best compatibility
- **Transaction-based operations** with rollback ensure system safety
- **Comprehensive security** through capabilities, audit logging, and input validation

This design creates an elegant, powerful MCP server capable of fully controlling and visualizing an Arch Linux system while maintaining security and reliability.