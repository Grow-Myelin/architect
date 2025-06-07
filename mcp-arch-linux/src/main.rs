use mcp_arch_linux::{LinuxMCPServer, Config, Result};
use mcp_arch_linux::plugins::{ArchInstallPlugin, HyprlandPlugin, ScreenCapturePlugin};
use mcp_arch_linux::mcp::server::MCPJsonRpcServer;
use tracing::{info, error};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env().add_directive("mcp_arch_linux=info".parse().unwrap()))
        .init();
    
    info!("Starting MCP Arch Linux Server");
    
    // Load configuration
    let config = Config::from_env()?;
    let bind_addr: SocketAddr = config.bind_address.parse()
        .map_err(|e| mcp_arch_linux::MCPError::Configuration(format!("Invalid bind address: {}", e)))?;
    
    // Setup security capabilities
    if let Err(e) = mcp_arch_linux::security::setup_minimal_capabilities() {
        error!("Failed to setup capabilities: {}", e);
        // Continue anyway in development, but in production this should be fatal
    }
    
    // Create MCP server with plugins
    let server = LinuxMCPServer::builder()
        .with_config(config)
        .with_plugin(Box::new(ArchInstallPlugin::new()))
        .with_plugin(Box::new(HyprlandPlugin::new()))
        .with_plugin(Box::new(ScreenCapturePlugin::new()))
        .build()?;
    
    // Create JSON-RPC server
    let jsonrpc_server = MCPJsonRpcServer::new(server);
    
    // Setup shutdown signal
    let shutdown_signal = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
        info!("Shutdown signal received");
    };
    
    // Start the server
    info!("MCP server listening on {}", bind_addr);
    jsonrpc_server.serve(bind_addr, shutdown_signal).await?;
    
    Ok(())
}