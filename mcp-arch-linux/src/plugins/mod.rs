mod arch_install;
mod hyprland;
mod screen_capture;

pub use arch_install::ArchInstallPlugin;
pub use hyprland::HyprlandPlugin;
pub use screen_capture::ScreenCapturePlugin;

use crate::{Result, MCPError};
use crate::mcp::{Tool, Resource, MCPToolResult, ToolArgs};
use async_trait::async_trait;
use std::collections::HashMap;
use tracing::{info, error};

#[async_trait]
pub trait MCPPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn tools(&self) -> Vec<Tool>;
    fn resources(&self) -> Vec<Resource>;
    
    async fn handle_tool_call(&self, tool: &str, args: ToolArgs) -> Result<MCPToolResult>;
    async fn handle_resource_read(&self, uri: &str) -> Result<String>;
}

pub struct PluginRegistry {
    plugins: HashMap<String, Box<dyn MCPPlugin>>,
    tool_to_plugin: HashMap<String, String>,
    resource_to_plugin: HashMap<String, String>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            tool_to_plugin: HashMap::new(),
            resource_to_plugin: HashMap::new(),
        }
    }
    
    pub fn register(&mut self, plugin: Box<dyn MCPPlugin>) -> Result<()> {
        let name = plugin.name().to_string();
        
        // Register tools
        for tool in plugin.tools() {
            if self.tool_to_plugin.contains_key(&tool.name) {
                return Err(MCPError::Configuration(
                    format!("Tool {} already registered", tool.name)
                ));
            }
            self.tool_to_plugin.insert(tool.name.clone(), name.clone());
        }
        
        // Register resources
        for resource in plugin.resources() {
            if self.resource_to_plugin.contains_key(&resource.uri) {
                return Err(MCPError::Configuration(
                    format!("Resource {} already registered", resource.uri)
                ));
            }
            self.resource_to_plugin.insert(resource.uri.clone(), name.clone());
        }
        
        info!("Registered plugin: {}", name);
        self.plugins.insert(name, plugin);
        Ok(())
    }
    
    pub async fn list_tools(&self) -> Vec<Tool> {
        let mut tools = Vec::new();
        for plugin in self.plugins.values() {
            tools.extend(plugin.tools());
        }
        tools
    }
    
    pub async fn list_resources(&self) -> Vec<Resource> {
        let mut resources = Vec::new();
        for plugin in self.plugins.values() {
            resources.extend(plugin.resources());
        }
        resources
    }
    
    pub async fn execute_tool(&self, tool_name: &str, args: ToolArgs) -> Result<MCPToolResult> {
        let plugin_name = self.tool_to_plugin.get(tool_name)
            .ok_or_else(|| MCPError::Other(anyhow::anyhow!("Tool not found: {}", tool_name)))?;
        
        let plugin = self.plugins.get(plugin_name)
            .ok_or_else(|| MCPError::Other(anyhow::anyhow!("Plugin not found: {}", plugin_name)))?;
        
        plugin.handle_tool_call(tool_name, args).await
    }
    
    pub async fn read_resource(&self, uri: &str) -> Result<String> {
        let plugin_name = self.resource_to_plugin.get(uri)
            .ok_or_else(|| MCPError::Other(anyhow::anyhow!("Resource not found: {}", uri)))?;
        
        let plugin = self.plugins.get(plugin_name)
            .ok_or_else(|| MCPError::Other(anyhow::anyhow!("Plugin not found: {}", plugin_name)))?;
        
        plugin.handle_resource_read(uri).await
    }
}