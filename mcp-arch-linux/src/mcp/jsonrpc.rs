use serde::{Serialize, Deserialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use async_trait::async_trait;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcMessage {
    Request(JsonRpcRequest),
    Response(JsonRpcResponse),
    Notification(JsonRpcNotification),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    pub id: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }
    
    pub fn parse_error() -> Self {
        Self::new(-32700, "Parse error")
    }
    
    pub fn invalid_request() -> Self {
        Self::new(-32600, "Invalid Request")
    }
    
    pub fn method_not_found() -> Self {
        Self::new(-32601, "Method not found")
    }
    
    pub fn invalid_params() -> Self {
        Self::new(-32602, "Invalid params")
    }
    
    pub fn internal_error() -> Self {
        Self::new(-32603, "Internal error")
    }
}

#[async_trait]
pub trait JsonRpcHandler: Send + Sync {
    async fn handle(&self, method: &str, params: Option<Value>) -> Result<Value, JsonRpcError>;
}

pub struct JsonRpcServer {
    handlers: Arc<RwLock<HashMap<String, Box<dyn JsonRpcHandler>>>>,
}

impl JsonRpcServer {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn register_handler(&self, method: String, handler: Box<dyn JsonRpcHandler>) {
        let mut handlers = self.handlers.write().await;
        handlers.insert(method, handler);
    }
    
    pub async fn handle_message(&self, message: &str) -> String {
        let request: JsonRpcRequest = match serde_json::from_str(message) {
            Ok(req) => req,
            Err(_) => {
                let error_response = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: Value::Null,
                    result: None,
                    error: Some(JsonRpcError::parse_error()),
                };
                return serde_json::to_string(&error_response).unwrap_or_default();
            }
        };
        
        let handlers = self.handlers.read().await;
        let response = if let Some(handler) = handlers.get(&request.method) {
            match handler.handle(&request.method, request.params).await {
                Ok(result) => JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(result),
                    error: None,
                },
                Err(error) => JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: None,
                    error: Some(error),
                },
            }
        } else {
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(JsonRpcError::method_not_found()),
            }
        };
        
        serde_json::to_string(&response).unwrap_or_default()
    }
}