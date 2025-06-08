use crate::{Result, MCPError};
use std::path::Path;
use std::future::Future;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use serde_json::json;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tracing::{info, warn, error};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditableOperation {
    pub id: String,
    pub name: String,
    pub parameters: serde_json::Value,
    pub user_context: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub result: Result<String>,
    pub session_id: String,
}

pub struct SecurityManager {
    require_auth: bool,
    audit_logger: AuditLogger,
    session_id: String,
}

impl SecurityManager {
    pub fn new(require_auth: bool, audit_log_path: &str) -> Result<Self> {
        let audit_logger = AuditLogger::new(audit_log_path)?;
        let session_id = Uuid::new_v4().to_string();
        
        Ok(Self {
            require_auth,
            audit_logger,
            session_id,
        })
    }
    
    pub async fn execute_with_audit<F, T>(&self, operation_name: &str, operation: F) -> Result<T>
    where
        F: Future<Output = Result<T>>,
        T: Serialize,
    {
        let start_time = Utc::now();
        let operation_id = Uuid::new_v4().to_string();
        
        info!("Starting audited operation: {} ({})", operation_name, operation_id);
        
        // Execute the operation
        let result = operation.await;
        
        // Log the audit entry
        let audit_entry = AuditableOperation {
            id: operation_id.clone(),
            name: operation_name.to_string(),
            parameters: json!({}), // Parameters should be passed in for real usage
            user_context: self.get_user_context(),
            timestamp: start_time,
            result: result.as_ref().map(|_| "Success".to_string()).map_err(|e| e.clone()),
            session_id: self.session_id.clone(),
        };
        
        self.audit_logger.log(&audit_entry).await?;
        
        match &result {
            Ok(_) => info!("Operation {} completed successfully", operation_id),
            Err(e) => error!("Operation {} failed: {}", operation_id, e),
        }
        
        result
    }
    
    fn get_user_context(&self) -> Option<String> {
        // In a real implementation, this would get the authenticated user
        std::env::var("USER").ok()
    }
    
    pub fn check_permission(&self, operation: &str) -> Result<()> {
        if self.require_auth {
            // In a real implementation, check actual permissions
            info!("Permission check for operation: {}", operation);
        }
        Ok(())
    }
}

struct AuditLogger {
    log_path: String,
}

impl AuditLogger {
    fn new(log_path: &str) -> Result<Self> {
        // Ensure directory exists
        if let Some(parent) = Path::new(log_path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        Ok(Self {
            log_path: log_path.to_string(),
        })
    }
    
    async fn log(&self, entry: &AuditableOperation) -> Result<()> {
        let json_entry = serde_json::to_string(entry)?;
        
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .await?;
        
        file.write_all(json_entry.as_bytes()).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;
        
        Ok(())
    }
}

pub fn setup_minimal_capabilities() -> Result<()> {
    use caps::{Capability, CapSet};
    
    // Check if we're running as root
    if unsafe { libc::geteuid() } != 0 {
        warn!("Not running as root, skipping capability setup");
        return Ok(());
    }
    
    let mut effective = CapSet::empty();
    let mut permitted = CapSet::empty();
    
    // Add only necessary capabilities
    let required_caps = vec![
        Capability::CAP_DAC_OVERRIDE,  // Override file permissions
        Capability::CAP_SYS_ADMIN,     // System administration
        Capability::CAP_NET_ADMIN,     // Network configuration
        Capability::CAP_SYS_CHROOT,    // chroot for arch-chroot
    ];
    
    for cap in required_caps {
        effective.add(cap)?;
        permitted.add(cap)?;
    }
    
    // Set capabilities
    caps::set(None, caps::CapSet::empty(), effective)?;
    
    info!("Configured minimal capabilities");
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSnapshot {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub description: String,
    pub files_backup: Vec<FileBackup>,
    pub service_states: Vec<ServiceState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileBackup {
    pub path: String,
    pub content: String,
    pub permissions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceState {
    pub name: String,
    pub enabled: bool,
    pub active: bool,
}

pub struct RollbackManager {
    snapshots_dir: String,
}

impl RollbackManager {
    pub fn new() -> Self {
        let snapshots_dir = std::env::var("MCP_SNAPSHOTS_DIR")
            .unwrap_or_else(|_| "/var/lib/mcp-arch-linux/snapshots".to_string());
        
        Self { snapshots_dir }
    }
    
    pub async fn create_snapshot(&self, description: &str, files: Vec<&str>) -> Result<String> {
        let snapshot_id = Uuid::new_v4().to_string();
        let timestamp = Utc::now();
        
        info!("Creating snapshot {}: {}", snapshot_id, description);
        
        // Backup files
        let mut files_backup = Vec::new();
        for file_path in files {
            if Path::new(file_path).exists() {
                let content = tokio::fs::read_to_string(file_path).await?;
                let metadata = tokio::fs::metadata(file_path).await?;
                
                // Get permissions using nix
                use nix::sys::stat;
                let stat = stat::stat(file_path).map_err(|e| MCPError::Other(anyhow::anyhow!("Failed to get file stats: {}", e)))?;
                
                files_backup.push(FileBackup {
                    path: file_path.to_string(),
                    content,
                    permissions: stat.st_mode,
                });
            }
        }
        
        // Get service states
        let service_states = self.capture_service_states().await?;
        
        let snapshot = SystemSnapshot {
            id: snapshot_id.clone(),
            timestamp,
            description: description.to_string(),
            files_backup,
            service_states,
        };
        
        // Save snapshot
        let snapshot_path = format!("{}/{}.json", self.snapshots_dir, snapshot_id);
        tokio::fs::create_dir_all(&self.snapshots_dir).await?;
        
        let snapshot_json = serde_json::to_string_pretty(&snapshot)?;
        tokio::fs::write(&snapshot_path, snapshot_json).await?;
        
        info!("Snapshot {} created successfully", snapshot_id);
        Ok(snapshot_id)
    }
    
    pub async fn rollback(&self, snapshot_id: &str) -> Result<()> {
        info!("Rolling back to snapshot {}", snapshot_id);
        
        let snapshot_path = format!("{}/{}.json", self.snapshots_dir, snapshot_id);
        let snapshot_json = tokio::fs::read_to_string(&snapshot_path).await?;
        let snapshot: SystemSnapshot = serde_json::from_str(&snapshot_json)?;
        
        // Restore files
        for file_backup in &snapshot.files_backup {
            info!("Restoring file: {}", file_backup.path);
            tokio::fs::write(&file_backup.path, &file_backup.content).await?;
            
            // Restore permissions (using nix for cross-platform compatibility)
            use nix::sys::stat::Mode;
            use nix::unistd::fchmod;
            use std::os::unix::io::AsRawFd;
            
            let file = std::fs::File::open(&file_backup.path)?;
            let mode = Mode::from_bits_truncate(file_backup.permissions);
            fchmod(file.as_raw_fd(), mode).map_err(|e| MCPError::Other(anyhow::anyhow!("Failed to restore permissions: {}", e)))?;
        }
        
        // Restore service states
        for service_state in &snapshot.service_states {
            self.restore_service_state(service_state).await?;
        }
        
        info!("Rollback to snapshot {} completed", snapshot_id);
        Ok(())
    }
    
    async fn capture_service_states(&self) -> Result<Vec<ServiceState>> {
        // This is a simplified version - in production, query systemd
        Ok(vec![])
    }
    
    async fn restore_service_state(&self, state: &ServiceState) -> Result<()> {
        use crate::system::execute_privileged_command;
        
        if state.enabled {
            execute_privileged_command("systemctl", &["enable", &state.name], true).await?;
        } else {
            execute_privileged_command("systemctl", &["disable", &state.name], true).await?;
        }
        
        if state.active {
            execute_privileged_command("systemctl", &["start", &state.name], true).await?;
        } else {
            execute_privileged_command("systemctl", &["stop", &state.name], true).await?;
        }
        
        Ok(())
    }
    
    pub async fn list_snapshots(&self) -> Result<Vec<SystemSnapshot>> {
        let mut snapshots = Vec::new();
        
        if !Path::new(&self.snapshots_dir).exists() {
            return Ok(snapshots);
        }
        
        let mut entries = tokio::fs::read_dir(&self.snapshots_dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                let content = tokio::fs::read_to_string(entry.path()).await?;
                if let Ok(snapshot) = serde_json::from_str::<SystemSnapshot>(&content) {
                    snapshots.push(snapshot);
                }
            }
        }
        
        // Sort by timestamp
        snapshots.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        Ok(snapshots)
    }
}