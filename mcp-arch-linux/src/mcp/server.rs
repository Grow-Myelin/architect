use super::{Tool, Resource, MCPToolResult, ToolArgs};
use super::jsonrpc::{JsonRpcServer, JsonRpcHandler, JsonRpcError};
use crate::{LinuxMCPServer, Result, MCPError};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{info, error, debug, warn};

pub struct MCPJsonRpcServer {
    server: Arc<LinuxMCPServer>,
    rpc: JsonRpcServer,
}

#[derive(Debug, Serialize, Deserialize)]
struct InitializeParams {
    protocol_version: String,
    capabilities: ClientCapabilities,
    client_info: ClientInfo,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClientCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    roots: Option<RootCapabilities>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sampling: Option<SamplingCapabilities>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RootCapabilities {
    list_changed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct SamplingCapabilities {
    supported: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClientInfo {
    name: String,
    version: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ServerCapabilities {
    tools: ToolsCapability,
    resources: ResourcesCapability,
    prompts: PromptsCapability,
}

#[derive(Debug, Serialize, Deserialize)]
struct ToolsCapability {
    list_changed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ResourcesCapability {
    subscribe: bool,
    list_changed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct PromptsCapability {
    list_changed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct InitializeResult {
    protocol_version: String,
    capabilities: ServerCapabilities,
    server_info: ServerInfo,
}

#[derive(Debug, Serialize, Deserialize)]
struct ServerInfo {
    name: String,
    version: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ToolCallParams {
    name: String,
    arguments: Option<Value>,
}

struct InitializeHandler {
    server: Arc<LinuxMCPServer>,
}

#[async_trait]
impl JsonRpcHandler for InitializeHandler {
    async fn handle(&self, _method: &str, params: Option<Value>) -> std::result::Result<Value, JsonRpcError> {
        let params: InitializeParams = if let Some(p) = params {
            serde_json::from_value(p).map_err(|_| JsonRpcError::invalid_params())?
        } else {
            return Err(JsonRpcError::invalid_params());
        };
        
        info!("Client initialized: {} v{}", params.client_info.name, params.client_info.version);
        
        let result = InitializeResult {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ServerCapabilities {
                tools: ToolsCapability { list_changed: true },
                resources: ResourcesCapability { 
                    subscribe: true,
                    list_changed: true,
                },
                prompts: PromptsCapability { list_changed: true },
            },
            server_info: ServerInfo {
                name: "mcp-arch-linux".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };
        
        Ok(serde_json::to_value(result).unwrap())
    }
}

struct InitializedHandler;

#[async_trait]
impl JsonRpcHandler for InitializedHandler {
    async fn handle(&self, _method: &str, _params: Option<Value>) -> std::result::Result<Value, JsonRpcError> {
        info!("Client initialization complete");
        Ok(json!({}))
    }
}

struct ToolsListHandler {
    server: Arc<LinuxMCPServer>,
}

#[async_trait]
impl JsonRpcHandler for ToolsListHandler {
    async fn handle(&self, _method: &str, _params: Option<Value>) -> std::result::Result<Value, JsonRpcError> {
        let plugins = self.server.plugins.read().await;
        let tools = plugins.list_tools().await;
        Ok(serde_json::to_value(tools).unwrap())
    }
}

struct ToolCallHandler {
    server: Arc<LinuxMCPServer>,
}

#[async_trait]
impl JsonRpcHandler for ToolCallHandler {
    async fn handle(&self, _method: &str, params: Option<Value>) -> std::result::Result<Value, JsonRpcError> {
        let params: ToolCallParams = if let Some(p) = params {
            serde_json::from_value(p).map_err(|_| JsonRpcError::invalid_params())?
        } else {
            return Err(JsonRpcError::invalid_params());
        };
        
        // Acquire semaphore permit for rate limiting
        let _permit = self.server.semaphore.acquire().await
            .map_err(|_| JsonRpcError::internal_error())?;
        
        // Create tool args
        let args = if let Some(arguments) = params.arguments {
            match arguments {
                Value::Object(map) => ToolArgs { args: map },
                _ => return Err(JsonRpcError::invalid_params()),
            }
        } else {
            ToolArgs { args: serde_json::Map::new() }
        };
        
        // Execute tool with security checks
        let result = self.server.security_manager
            .execute_with_audit(&params.name, async {
                let plugins = self.server.plugins.read().await;
                plugins.execute_tool(&params.name, args).await
            })
            .await
            .map_err(|e| JsonRpcError::new(-32603, e.to_string()))?;
        
        Ok(serde_json::to_value(result).unwrap())
    }
}

struct ResourcesListHandler {
    server: Arc<LinuxMCPServer>,
}

#[async_trait]
impl JsonRpcHandler for ResourcesListHandler {
    async fn handle(&self, _method: &str, _params: Option<Value>) -> std::result::Result<Value, JsonRpcError> {
        let plugins = self.server.plugins.read().await;
        let resources = plugins.list_resources().await;
        Ok(serde_json::to_value(resources).unwrap())
    }
}

struct ResourceReadHandler {
    server: Arc<LinuxMCPServer>,
}

#[async_trait]
impl JsonRpcHandler for ResourceReadHandler {
    async fn handle(&self, _method: &str, params: Option<Value>) -> std::result::Result<Value, JsonRpcError> {
        let uri = params
            .and_then(|p| p.get("uri"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError::invalid_params())?;
        
        let plugins = self.server.plugins.read().await;
        let content = plugins.read_resource(uri).await
            .map_err(|e| JsonRpcError::new(-32603, e.to_string()))?;
        
        Ok(json!({ "content": content }))
    }
}

impl MCPJsonRpcServer {
    pub async fn new(server: LinuxMCPServer) -> Self {
        let server = Arc::new(server);
        let rpc = JsonRpcServer::new();
        
        // Register MCP protocol methods
        rpc.register_handler(
            "initialize".to_string(),
            Box::new(InitializeHandler { server: Arc::clone(&server) })
        ).await;
        
        rpc.register_handler(
            "initialized".to_string(),
            Box::new(InitializedHandler)
        ).await;
        
        rpc.register_handler(
            "tools/list".to_string(),
            Box::new(ToolsListHandler { server: Arc::clone(&server) })
        ).await;
        
        rpc.register_handler(
            "tools/call".to_string(),
            Box::new(ToolCallHandler { server: Arc::clone(&server) })
        ).await;
        
        rpc.register_handler(
            "resources/list".to_string(),
            Box::new(ResourcesListHandler { server: Arc::clone(&server) })
        ).await;
        
        rpc.register_handler(
            "resources/read".to_string(),
            Box::new(ResourceReadHandler { server: Arc::clone(&server) })
        ).await;
        
        Self { server, rpc }
    }
    
    pub async fn serve<F>(self, addr: SocketAddr, shutdown: F) -> Result<()>
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let listener = TcpListener::bind(addr).await?;
        let server = Arc::new(self);
        
        info!("JSON-RPC server listening on {}", addr);
        
        loop {
            tokio::select! {
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((stream, peer_addr)) => {
                            debug!("New connection from {}", peer_addr);
                            let server = Arc::clone(&server);
                            
                            tokio::spawn(async move {
                                if let Err(e) = server.handle_connection(stream).await {
                                    error!("Error handling connection from {}: {}", peer_addr, e);
                                }
                            });
                        }
                        Err(e) => {
                            error!("Failed to accept connection: {}", e);
                        }
                    }
                }
                _ = shutdown => {
                    info!("Shutdown signal received");
                    break;
                }
            }
        }
        
        Ok(())
    }
    
    async fn handle_connection(&self, mut stream: TcpStream) -> Result<()> {
        let (read_half, mut write_half) = stream.split();
        let mut reader = BufReader::new(read_half);
        let mut line = String::new();
        
        while reader.read_line(&mut line).await? > 0 {
            let response = self.rpc.handle_message(&line).await;
            write_half.write_all(response.as_bytes()).await?;
            write_half.write_all(b"\n").await?;
            line.clear();
        }
        
        Ok(())
    }
}