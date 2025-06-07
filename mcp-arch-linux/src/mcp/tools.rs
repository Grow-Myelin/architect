use super::{Tool, MCPToolResult, ToolArgs};
use crate::Result;
use serde_json::json;

pub fn get_system_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "system_exec".to_string(),
            description: "Execute a system command with proper privilege handling".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Command to execute"
                    },
                    "args": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Command arguments"
                    },
                    "require_root": {
                        "type": "boolean",
                        "description": "Whether the command requires root privileges",
                        "default": false
                    },
                    "timeout": {
                        "type": "integer",
                        "description": "Timeout in seconds",
                        "default": 300
                    }
                },
                "required": ["command"]
            }),
        },
        Tool {
            name: "system_snapshot".to_string(),
            description: "Create a system snapshot for rollback".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "description": {
                        "type": "string",
                        "description": "Snapshot description"
                    },
                    "files": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Files to include in snapshot"
                    }
                },
                "required": ["description"]
            }),
        },
        Tool {
            name: "system_rollback".to_string(),
            description: "Rollback to a previous system snapshot".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "snapshot_id": {
                        "type": "string",
                        "description": "Snapshot ID to rollback to"
                    }
                },
                "required": ["snapshot_id"]
            }),
        },
    ]
}