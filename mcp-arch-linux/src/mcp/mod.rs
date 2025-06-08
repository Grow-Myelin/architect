pub mod server;
pub mod protocol;
pub mod tools;
pub mod resources;
pub mod jsonrpc;

use serde::{Serialize, Deserialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MCPContent {
    #[serde(rename = "text")]
    Text { text: String },
    
    #[serde(rename = "image")]
    Image { 
        data: String,  // base64 encoded
        mime_type: String,
    },
    
    #[serde(rename = "resource")]
    Resource {
        uri: String,
        text: Option<String>,
        mime_type: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPToolResult {
    pub content: Vec<MCPContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

impl MCPToolResult {
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: vec![MCPContent::Text { text: content.into() }],
            is_error: None,
            metadata: None,
        }
    }
    
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: vec![MCPContent::Text { text: message.into() }],
            is_error: Some(true),
            metadata: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolArgs {
    #[serde(flatten)]
    pub args: serde_json::Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}