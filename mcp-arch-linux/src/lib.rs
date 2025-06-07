pub mod mcp;
pub mod system;
pub mod security;
pub mod plugins;

use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use thiserror::Error;
use tracing::{info, warn, error};

#[derive(Debug, Error)]
pub enum MCPError {
    #[error("JSON-RPC error: {0}")]
    JsonRpc(String),
    
    #[error("System command failed: {0}")]
    SystemCommand(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Resource locked: {0}")]
    ResourceLocked(String),
    
    #[error("Invalid configuration: {0}")]
    Configuration(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, MCPError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub bind_address: String,
    pub max_concurrent_operations: usize,
    pub require_auth: bool,
    pub audit_log_path: String,
    pub plugins: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:8080".to_string(),
            max_concurrent_operations: 10,
            require_auth: true,
            audit_log_path: "/var/log/mcp-arch-linux/audit.log".to_string(),
            plugins: vec!["arch_install".to_string(), "hyprland".to_string()],
        }
    }
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenv::dotenv().ok();
        
        let mut config = Config::default();
        
        if let Ok(addr) = std::env::var("MCP_BIND_ADDRESS") {
            config.bind_address = addr;
        }
        
        if let Ok(max_ops) = std::env::var("MCP_MAX_CONCURRENT_OPS") {
            config.max_concurrent_operations = max_ops.parse()
                .map_err(|_| MCPError::Configuration("Invalid max concurrent ops".to_string()))?;
        }
        
        if let Ok(auth) = std::env::var("MCP_REQUIRE_AUTH") {
            config.require_auth = auth.parse()
                .unwrap_or(true);
        }
        
        Ok(config)
    }
}

#[derive(Debug, Clone)]
pub struct LinuxMCPServer {
    config: Arc<Config>,
    semaphore: Arc<Semaphore>,
    plugins: Arc<RwLock<plugins::PluginRegistry>>,
    security_manager: Arc<security::SecurityManager>,
}

impl LinuxMCPServer {
    pub fn builder() -> LinuxMCPServerBuilder {
        LinuxMCPServerBuilder::default()
    }
    
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down MCP server");
        Ok(())
    }
}

#[derive(Default)]
pub struct LinuxMCPServerBuilder {
    config: Option<Config>,
    plugins: Vec<Box<dyn plugins::MCPPlugin>>,
}

impl LinuxMCPServerBuilder {
    pub fn with_config(mut self, config: Config) -> Self {
        self.config = Some(config);
        self
    }
    
    pub fn with_plugin(mut self, plugin: Box<dyn plugins::MCPPlugin>) -> Self {
        self.plugins.push(plugin);
        self
    }
    
    pub fn build(self) -> Result<LinuxMCPServer> {
        let config = Arc::new(self.config.unwrap_or_default());
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent_operations));
        
        let mut registry = plugins::PluginRegistry::new();
        for plugin in self.plugins {
            registry.register(plugin)?;
        }
        
        let security_manager = Arc::new(security::SecurityManager::new(
            config.require_auth,
            &config.audit_log_path,
        )?);
        
        Ok(LinuxMCPServer {
            config,
            semaphore,
            plugins: Arc::new(RwLock::new(registry)),
            security_manager,
        })
    }
}