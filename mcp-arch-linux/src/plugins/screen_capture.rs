use super::MCPPlugin;
use crate::{Result, MCPError};
use crate::mcp::{Tool, Resource, MCPToolResult, ToolArgs, MCPContent};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::process::Stdio;
use tokio::process::Command;
use tracing::{info, warn, error};

pub struct ScreenCapturePlugin {
    capture_dir: String,
}

impl ScreenCapturePlugin {
    pub fn new() -> Self {
        let capture_dir = std::env::var("MCP_CAPTURE_DIR")
            .unwrap_or_else(|_| "/tmp/mcp-captures".to_string());
        
        Self { capture_dir }
    }
    
    async fn ensure_capture_dir(&self) -> Result<()> {
        tokio::fs::create_dir_all(&self.capture_dir).await?;
        Ok(())
    }
}

#[async_trait]
impl MCPPlugin for ScreenCapturePlugin {
    fn name(&self) -> &str {
        "screen_capture"
    }
    
    fn tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                name: "capture_screenshot".to_string(),
                description: "Capture a screenshot of the current display".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "output": {
                            "type": "string",
                            "description": "Output name (monitor) to capture, or 'all' for all outputs"
                        },
                        "region": {
                            "type": "object",
                            "properties": {
                                "x": { "type": "integer" },
                                "y": { "type": "integer" },
                                "width": { "type": "integer" },
                                "height": { "type": "integer" }
                            },
                            "description": "Specific region to capture"
                        },
                        "format": {
                            "type": "string",
                            "enum": ["png", "jpg"],
                            "default": "png"
                        }
                    }
                }),
            },
            Tool {
                name: "capture_window".to_string(),
                description: "Capture a specific window".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "window_id": {
                            "type": "string",
                            "description": "Window ID to capture (optional, uses active window if not specified)"
                        },
                        "format": {
                            "type": "string",
                            "enum": ["png", "jpg"],
                            "default": "png"
                        }
                    }
                }),
            },
            Tool {
                name: "start_recording".to_string(),
                description: "Start screen recording".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "output": {
                            "type": "string",
                            "description": "Output name to record"
                        },
                        "audio": {
                            "type": "boolean",
                            "description": "Include audio in recording",
                            "default": false
                        },
                        "format": {
                            "type": "string",
                            "enum": ["mp4", "webm"],
                            "default": "mp4"
                        }
                    }
                }),
            },
            Tool {
                name: "stop_recording".to_string(),
                description: "Stop current screen recording".to_string(),
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
                uri: "capture://last".to_string(),
                name: "Last Capture".to_string(),
                description: Some("The most recent screen capture".to_string()),
                mime_type: Some("image/png".to_string()),
            },
            Resource {
                uri: "capture://list".to_string(),
                name: "Capture List".to_string(),
                description: Some("List of available captures".to_string()),
                mime_type: Some("application/json".to_string()),
            },
        ]
    }
    
    async fn handle_tool_call(&self, tool: &str, args: ToolArgs) -> Result<MCPToolResult> {
        match tool {
            "capture_screenshot" => self.handle_screenshot(args).await,
            "capture_window" => self.handle_window_capture(args).await,
            "start_recording" => self.handle_start_recording(args).await,
            "stop_recording" => self.handle_stop_recording(args).await,
            _ => Err(MCPError::Other(anyhow::anyhow!("Unknown tool: {}", tool))),
        }
    }
    
    async fn handle_resource_read(&self, uri: &str) -> Result<String> {
        match uri {
            "capture://last" => self.get_last_capture().await,
            "capture://list" => self.get_capture_list().await,
            _ => Err(MCPError::Other(anyhow::anyhow!("Unknown resource: {}", uri))),
        }
    }
}

impl ScreenCapturePlugin {
    async fn handle_screenshot(&self, args: ToolArgs) -> Result<MCPToolResult> {
        self.ensure_capture_dir().await?;
        
        let output = args.args.get("output")
            .and_then(|v| v.as_str());
        
        let format = args.args.get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("png");
        
        let timestamp = chrono::Utc::now().timestamp();
        let filename = format!("{}/screenshot_{}.{}", self.capture_dir, timestamp, format);
        
        // Try grim first (Wayland screenshot tool)
        let mut cmd = Command::new("grim");
        
        if let Some(output_name) = output {
            if output_name != "all" {
                cmd.arg("-o").arg(output_name);
            }
        }
        
        if let Some(region) = args.args.get("region") {
            if let (Some(x), Some(y), Some(width), Some(height)) = (
                region.get("x").and_then(|v| v.as_i64()),
                region.get("y").and_then(|v| v.as_i64()),
                region.get("width").and_then(|v| v.as_i64()),
                region.get("height").and_then(|v| v.as_i64()),
            ) {
                cmd.arg("-g").arg(format!("{},{} {}x{}", x, y, width, height));
            }
        }
        
        cmd.arg(&filename);
        
        let output = cmd.output().await?;
        
        if !output.status.success() {
            // Try wlr-screencopy as fallback
            let fallback = Command::new("wlr-screencopy")
                .arg(&filename)
                .output()
                .await;
            
            if let Ok(fallback_output) = fallback {
                if !fallback_output.status.success() {
                    return Ok(MCPToolResult::error("Failed to capture screenshot"));
                }
            } else {
                return Ok(MCPToolResult::error("No screenshot tool available"));
            }
        }
        
        // Read the captured image and encode as base64
        let image_data = tokio::fs::read(&filename).await?;
        let base64_data = base64::encode(&image_data);
        
        Ok(MCPToolResult {
            content: vec![MCPContent::Image {
                data: base64_data,
                mime_type: format!("image/{}", format),
            }],
            is_error: None,
            metadata: Some(json!({
                "filename": filename,
                "timestamp": timestamp,
                "size": image_data.len()
            })),
        })
    }
    
    async fn handle_window_capture(&self, args: ToolArgs) -> Result<MCPToolResult> {
        self.ensure_capture_dir().await?;
        
        let window_id = args.args.get("window_id")
            .and_then(|v| v.as_str());
        
        let format = args.args.get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("png");
        
        let timestamp = chrono::Utc::now().timestamp();
        let filename = format!("{}/window_{}.{}", self.capture_dir, timestamp, format);
        
        // Get active window if no ID specified
        let target_window = if let Some(id) = window_id {
            id.to_string()
        } else {
            // Get active window from Hyprland
            let output = Command::new("hyprctl")
                .args(&["activewindow", "-j"])
                .output()
                .await?;
            
            if !output.status.success() {
                return Ok(MCPToolResult::error("Failed to get active window"));
            }
            
            let window_info: Value = serde_json::from_slice(&output.stdout)?;
            window_info.get("address")
                .and_then(|v| v.as_str())
                .ok_or_else(|| MCPError::Other(anyhow::anyhow!("No active window")))?
                .to_string()
        };
        
        // Use grim with window selection
        let output = Command::new("grim")
            .arg("-g")
            .arg(format!("$(hyprctl clients -j | jq -r '.[] | select(.address == \"{}\") | \"\\(.at[0]),\\(.at[1]) \\(.size[0])x\\(.size[1])\"')", target_window))
            .arg(&filename)
            .output()
            .await?;
        
        if !output.status.success() {
            return Ok(MCPToolResult::error("Failed to capture window"));
        }
        
        let image_data = tokio::fs::read(&filename).await?;
        let base64_data = base64::encode(&image_data);
        
        Ok(MCPToolResult {
            content: vec![MCPContent::Image {
                data: base64_data,
                mime_type: format!("image/{}", format),
            }],
            is_error: None,
            metadata: Some(json!({
                "filename": filename,
                "window_id": target_window,
                "timestamp": timestamp
            })),
        })
    }
    
    async fn handle_start_recording(&self, args: ToolArgs) -> Result<MCPToolResult> {
        self.ensure_capture_dir().await?;
        
        let output = args.args.get("output")
            .and_then(|v| v.as_str());
        
        let audio = args.args.get("audio")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        let format = args.args.get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("mp4");
        
        let timestamp = chrono::Utc::now().timestamp();
        let filename = format!("{}/recording_{}.{}", self.capture_dir, timestamp, format);
        
        // Use wf-recorder for screen recording
        let mut cmd = Command::new("wf-recorder");
        
        if let Some(output_name) = output {
            cmd.arg("-o").arg(output_name);
        }
        
        if audio {
            cmd.arg("-a");
        }
        
        cmd.arg("-f").arg(&filename);
        
        // Start recording in background
        let child = cmd
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        
        // Save PID for later stopping
        let pid_file = format!("{}/recording.pid", self.capture_dir);
        tokio::fs::write(&pid_file, child.id().unwrap().to_string()).await?;
        
        Ok(MCPToolResult::text(format!(
            "Started recording to {}. Use stop_recording to finish.",
            filename
        )))
    }
    
    async fn handle_stop_recording(&self, _args: ToolArgs) -> Result<MCPToolResult> {
        let pid_file = format!("{}/recording.pid", self.capture_dir);
        
        // Read PID
        let pid_str = tokio::fs::read_to_string(&pid_file).await
            .map_err(|_| MCPError::Other(anyhow::anyhow!("No active recording")))?;
        
        let pid: u32 = pid_str.trim().parse()
            .map_err(|_| MCPError::Other(anyhow::anyhow!("Invalid PID")))?;
        
        // Send SIGINT to stop recording gracefully
        Command::new("kill")
            .args(&["-INT", &pid.to_string()])
            .output()
            .await?;
        
        // Clean up PID file
        tokio::fs::remove_file(&pid_file).await.ok();
        
        Ok(MCPToolResult::text("Recording stopped"))
    }
    
    async fn get_last_capture(&self) -> Result<String> {
        let mut entries = tokio::fs::read_dir(&self.capture_dir).await?;
        let mut latest_file = None;
        let mut latest_time = None;
        
        while let Some(entry) = entries.next_entry().await? {
            let metadata = entry.metadata().await?;
            if metadata.is_file() {
                let modified = metadata.modified()?;
                if latest_time.is_none() || modified > latest_time.unwrap() {
                    latest_time = Some(modified);
                    latest_file = Some(entry.path());
                }
            }
        }
        
        if let Some(path) = latest_file {
            let data = tokio::fs::read(&path).await?;
            Ok(base64::encode(&data))
        } else {
            Err(MCPError::Other(anyhow::anyhow!("No captures found")))
        }
    }
    
    async fn get_capture_list(&self) -> Result<String> {
        let mut entries = tokio::fs::read_dir(&self.capture_dir).await?;
        let mut captures = Vec::new();
        
        while let Some(entry) = entries.next_entry().await? {
            let metadata = entry.metadata().await?;
            if metadata.is_file() {
                captures.push(json!({
                    "filename": entry.file_name().to_string_lossy(),
                    "size": metadata.len(),
                    "modified": metadata.modified()?.elapsed().unwrap().as_secs(),
                }));
            }
        }
        
        Ok(serde_json::to_string_pretty(&captures)?)
    }
}