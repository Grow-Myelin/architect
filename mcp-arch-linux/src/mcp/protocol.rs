use serde::{Serialize, Deserialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "jsonrpc")]
pub enum JsonRpcMessage {
    #[serde(rename = "2.0")]
    Request(JsonRpcRequest),
    #[serde(rename = "2.0")]
    Response(JsonRpcResponse),
    #[serde(rename = "2.0")]
    Notification(JsonRpcNotification),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub id: Value,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
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