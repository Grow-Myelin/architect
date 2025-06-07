use super::{Tool, Resource, MCPToolResult, ToolArgs};
use crate::{LinuxMCPServer, Result, MCPError};
use jsonrpc_v2::{Data, Id, Server, Params};
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{info, error, debug};

pub struct MCPJsonRpcServer {
    server: Arc<LinuxMCPServer>,
    rpc: Server<()>,
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

impl MCPJsonRpcServer {
    pub fn new(server: LinuxMCPServer) -> Self {
        let server = Arc::new(server);
        let mut rpc = Server::new();
        
        // Register MCP protocol methods
        rpc = rpc
            .with_method("initialize", Self::handle_initialize)
            .with_method("initialized", Self::handle_initialized)
            .with_method("tools/list", Self::handle_tools_list)
            .with_method("tools/call", Self::handle_tool_call)
            .with_method("resources/list", Self::handle_resources_list)
            .with_method("resources/read", Self::handle_resource_read)
            .with_method("completion/complete", Self::handle_completion);
        
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
    
    async fn handle_connection(&self, mut stream: tokio::net::TcpStream) -> Result<()> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        
        let (read_half, mut write_half) = stream.split();
        let mut reader = BufReader::new(read_half);
        let mut line = String::new();
        
        while reader.read_line(&mut line).await? > 0 {
            if let Ok(request) = serde_json::from_str(&line) {
                let response = self.rpc.handle(request).await;
                let response_str = serde_json::to_string(&response)?;
                write_half.write_all(response_str.as_bytes()).await?;
                write_half.write_all(b"\n").await?;
            }
            line.clear();
        }
        
        Ok(())
    }
    
    async fn handle_initialize(params: Params<InitializeParams>) -> jsonrpc_v2::Result<InitializeResult> {
        let params = params.parse()?;
        
        info!("Client initialized: {} v{}", params.client_info.name, params.client_info.version);
        
        Ok(InitializeResult {
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
        })
    }
    
    async fn handle_initialized(_params: Params<Value>) -> jsonrpc_v2::Result<()> {
        info!("Client initialization complete");
        Ok(())
    }
    
    async fn handle_tools_list(
        server: Data<Arc<MCPJsonRpcServer>>,
        _params: Params<Value>,
    ) -> jsonrpc_v2::Result<Vec<Tool>> {
        let plugins = server.server.plugins.read().await;
        let tools = plugins.list_tools().await;
        Ok(tools)
    }
    
    async fn handle_tool_call(
        server: Data<Arc<MCPJsonRpcServer>>,
        params: Params<ToolCallParams>,
    ) -> jsonrpc_v2::Result<MCPToolResult> {
        let params = params.parse()?;
        
        // Acquire semaphore permit for rate limiting
        let _permit = server.server.semaphore.acquire().await
            .map_err(|_| jsonrpc_v2::Error::internal_error())?;
        
        // Create tool args
        let args = if let Some(arguments) = params.arguments {
            match arguments {
                Value::Object(map) => ToolArgs { args: map },
                _ => return Err(jsonrpc_v2::Error::invalid_params("Arguments must be an object")),
            }
        } else {
            ToolArgs { args: serde_json::Map::new() }
        };
        
        // Execute tool with security checks
        let result = server.server.security_manager
            .execute_with_audit(&params.name, async {
                let plugins = server.server.plugins.read().await;
                plugins.execute_tool(&params.name, args).await
            })
            .await
            .map_err(|e| jsonrpc_v2::Error::new(-32603, e.to_string()))?;
        
        Ok(result)
    }
    
    async fn handle_resources_list(
        server: Data<Arc<MCPJsonRpcServer>>,
        _params: Params<Value>,
    ) -> jsonrpc_v2::Result<Vec<Resource>> {
        let plugins = server.server.plugins.read().await;
        let resources = plugins.list_resources().await;
        Ok(resources)
    }
    
    async fn handle_resource_read(
        server: Data<Arc<MCPJsonRpcServer>>,
        params: Params<Value>,
    ) -> jsonrpc_v2::Result<String> {
        let uri = params.parse::<serde_json::Map<String, Value>>()?
            .get("uri")
            .and_then(|v| v.as_str())
            .ok_or_else(|| jsonrpc_v2::Error::invalid_params("Missing uri parameter"))?;
        
        let plugins = server.server.plugins.read().await;
        let content = plugins.read_resource(uri).await
            .map_err(|e| jsonrpc_v2::Error::new(-32603, e.to_string()))?;
        
        Ok(content)
    }
    
    async fn handle_completion(
        _server: Data<Arc<MCPJsonRpcServer>>,
        params: Params<Value>,
    ) -> jsonrpc_v2::Result<Value> {
        // For now, return empty completions
        Ok(json!({
            "completion": {
                "values": []
            }
        }))
    }
}