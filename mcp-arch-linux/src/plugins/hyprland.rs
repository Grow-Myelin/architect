use super::MCPPlugin;
use crate::{Result, MCPError};
use crate::mcp::{Tool, Resource, MCPToolResult, ToolArgs, MCPContent};
use crate::system::hyprland::HyprlandIPC;
use async_trait::async_trait;
use serde_json::{json, Value};
use tracing::{info, warn, error};

pub struct HyprlandPlugin {
    ipc: Option<HyprlandIPC>,
}

impl HyprlandPlugin {
    pub fn new() -> Self {
        Self { ipc: None }
    }
    
    async fn ensure_connected(&mut self) -> Result<&mut HyprlandIPC> {
        if self.ipc.is_none() {
            self.ipc = Some(HyprlandIPC::connect().await?);
        }
        Ok(self.ipc.as_mut().unwrap())
    }
}

#[async_trait]
impl MCPPlugin for HyprlandPlugin {
    fn name(&self) -> &str {
        "hyprland"
    }
    
    fn tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                name: "hyprland_dispatch".to_string(),
                description: "Execute Hyprland dispatcher command".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "Dispatcher command (e.g., workspace, movewindow)"
                        },
                        "args": {
                            "type": "string",
                            "description": "Command arguments"
                        }
                    },
                    "required": ["command"]
                }),
            },
            Tool {
                name: "hyprland_keyword".to_string(),
                description: "Set Hyprland configuration keyword".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "keyword": {
                            "type": "string",
                            "description": "Configuration keyword"
                        },
                        "value": {
                            "type": "string",
                            "description": "Value to set"
                        }
                    },
                    "required": ["keyword", "value"]
                }),
            },
            Tool {
                name: "hyprland_window_info".to_string(),
                description: "Get information about windows".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "window_id": {
                            "type": "string",
                            "description": "Window ID (optional, defaults to active window)"
                        }
                    }
                }),
            },
            Tool {
                name: "hyprland_workspaces".to_string(),
                description: "List all workspaces".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "hyprland_monitors".to_string(),
                description: "List all monitors".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "hyprland_reload".to_string(),
                description: "Reload Hyprland configuration".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        ]
    }
    
    fn resources(&self) -> Vec<Resource> {
        vec![
            Resource {
                uri: "hyprland://config".to_string(),
                name: "Hyprland Configuration".to_string(),
                description: Some("Current Hyprland configuration".to_string()),
                mime_type: Some("text/plain".to_string()),
            },
            Resource {
                uri: "hyprland://layout".to_string(),
                name: "Window Layout".to_string(),
                description: Some("Current window layout information".to_string()),
                mime_type: Some("application/json".to_string()),
            },
        ]
    }
    
    async fn handle_tool_call(&self, tool: &str, args: ToolArgs) -> Result<MCPToolResult> {
        // Clone self to get mutable access
        let mut plugin = Self::new();
        
        match tool {
            "hyprland_dispatch" => plugin.handle_dispatch(args).await,
            "hyprland_keyword" => plugin.handle_keyword(args).await,
            "hyprland_window_info" => plugin.handle_window_info(args).await,
            "hyprland_workspaces" => plugin.handle_workspaces(args).await,
            "hyprland_monitors" => plugin.handle_monitors(args).await,
            "hyprland_reload" => plugin.handle_reload(args).await,
            _ => Err(MCPError::Other(anyhow::anyhow!("Unknown tool: {}", tool))),
        }
    }
    
    async fn handle_resource_read(&self, uri: &str) -> Result<String> {
        let mut plugin = Self::new();
        
        match uri {
            "hyprland://config" => plugin.get_config().await,
            "hyprland://layout" => plugin.get_layout().await,
            _ => Err(MCPError::Other(anyhow::anyhow!("Unknown resource: {}", uri))),
        }
    }
}

impl HyprlandPlugin {
    async fn handle_dispatch(&mut self, args: ToolArgs) -> Result<MCPToolResult> {
        let command = args.args.get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MCPError::Other(anyhow::anyhow!("Missing command parameter")))?;
        
        let cmd_args = args.args.get("args")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        let ipc = self.ensure_connected().await?;
        let full_command = if cmd_args.is_empty() {
            format!("dispatch {}", command)
        } else {
            format!("dispatch {} {}", command, cmd_args)
        };
        
        info!("Executing Hyprland command: {}", full_command);
        let response = ipc.send_command(&full_command).await?;
        
        Ok(MCPToolResult::text(response))
    }
    
    async fn handle_keyword(&mut self, args: ToolArgs) -> Result<MCPToolResult> {
        let keyword = args.args.get("keyword")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MCPError::Other(anyhow::anyhow!("Missing keyword parameter")))?;
        
        let value = args.args.get("value")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MCPError::Other(anyhow::anyhow!("Missing value parameter")))?;
        
        let ipc = self.ensure_connected().await?;
        let command = format!("keyword {} {}", keyword, value);
        
        info!("Setting Hyprland keyword: {}", command);
        let response = ipc.send_command(&command).await?;
        
        Ok(MCPToolResult::text(response))
    }
    
    async fn handle_window_info(&mut self, args: ToolArgs) -> Result<MCPToolResult> {
        let window_id = args.args.get("window_id")
            .and_then(|v| v.as_str());
        
        let ipc = self.ensure_connected().await?;
        let command = if let Some(id) = window_id {
            format!("j/clients | jq '.[] | select(.address == \"{}\")'", id)
        } else {
            "j/activewindow".to_string()
        };
        
        let response = ipc.send_command(&command).await?;
        let window_info: Value = serde_json::from_str(&response)?;
        
        Ok(MCPToolResult {
            content: vec![MCPContent::Text { 
                text: serde_json::to_string_pretty(&window_info)? 
            }],
            is_error: None,
            metadata: Some(json!({
                "type": "window_info"
            })),
        })
    }
    
    async fn handle_workspaces(&mut self, _args: ToolArgs) -> Result<MCPToolResult> {
        let ipc = self.ensure_connected().await?;
        let response = ipc.send_command("j/workspaces").await?;
        let workspaces: Value = serde_json::from_str(&response)?;
        
        Ok(MCPToolResult {
            content: vec![MCPContent::Text { 
                text: serde_json::to_string_pretty(&workspaces)? 
            }],
            is_error: None,
            metadata: Some(json!({
                "type": "workspaces"
            })),
        })
    }
    
    async fn handle_monitors(&mut self, _args: ToolArgs) -> Result<MCPToolResult> {
        let ipc = self.ensure_connected().await?;
        let response = ipc.send_command("j/monitors").await?;
        let monitors: Value = serde_json::from_str(&response)?;
        
        Ok(MCPToolResult {
            content: vec![MCPContent::Text { 
                text: serde_json::to_string_pretty(&monitors)? 
            }],
            is_error: None,
            metadata: Some(json!({
                "type": "monitors"
            })),
        })
    }
    
    async fn handle_reload(&mut self, _args: ToolArgs) -> Result<MCPToolResult> {
        let ipc = self.ensure_connected().await?;
        let response = ipc.send_command("reload").await?;
        
        Ok(MCPToolResult::text("Hyprland configuration reloaded"))
    }
    
    async fn get_config(&mut self) -> Result<String> {
        // Read Hyprland config file
        let config_path = std::env::var("HOME")
            .map(|home| format!("{}/.config/hypr/hyprland.conf", home))
            .unwrap_or_else(|_| "/etc/hypr/hyprland.conf".to_string());
        
        tokio::fs::read_to_string(&config_path)
            .await
            .map_err(|e| MCPError::Other(anyhow::anyhow!("Failed to read config: {}", e)))
    }
    
    async fn get_layout(&mut self) -> Result<String> {
        let ipc = self.ensure_connected().await?;
        
        // Get comprehensive layout information
        let clients = ipc.send_command("j/clients").await?;
        let workspaces = ipc.send_command("j/workspaces").await?;
        let monitors = ipc.send_command("j/monitors").await?;
        
        let layout = json!({
            "clients": serde_json::from_str::<Value>(&clients)?,
            "workspaces": serde_json::from_str::<Value>(&workspaces)?,
            "monitors": serde_json::from_str::<Value>(&monitors)?,
        });
        
        Ok(serde_json::to_string_pretty(&layout)?)
    }
}