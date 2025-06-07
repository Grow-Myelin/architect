use crate::{Result, MCPError};
use tokio::net::UnixStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, AsyncBufReadExt};
use std::env;
use tracing::{info, warn, error, debug};
use serde::{Serialize, Deserialize};
use serde_json::Value;

pub struct HyprlandIPC {
    control_socket: UnixStream,
    event_socket: Option<UnixStream>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyprlandWindow {
    pub address: String,
    pub at: [i32; 2],
    pub size: [i32; 2],
    pub workspace: WorkspaceInfo,
    pub floating: bool,
    pub monitor: i32,
    pub class: String,
    pub title: String,
    pub pid: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyprlandMonitor {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub refresh_rate: f32,
    pub scale: f32,
    pub transform: i32,
    pub focused: bool,
    pub active_workspace: WorkspaceInfo,
}

#[derive(Debug, Clone)]
pub enum HyprlandEvent {
    Workspace(String),
    ActiveWindow(String, String),
    Fullscreen(bool),
    MonitorAdded(String),
    MonitorRemoved(String),
    CreateWorkspace(i32),
    DestroyWorkspace(i32),
    MoveWorkspace(i32, String),
    OpenWindow(String, String, String, String),
    CloseWindow(String),
    MoveWindow(String, String),
    Urgent(String),
    Minimize(String, bool),
    Other(String, String),
}

impl HyprlandIPC {
    pub async fn connect() -> Result<Self> {
        let runtime_dir = env::var("XDG_RUNTIME_DIR")
            .map_err(|_| MCPError::Configuration("XDG_RUNTIME_DIR not set".to_string()))?;
        
        let instance = env::var("HYPRLAND_INSTANCE_SIGNATURE")
            .map_err(|_| MCPError::Configuration("HYPRLAND_INSTANCE_SIGNATURE not set".to_string()))?;
        
        let control_path = format!("{}/hypr/{}/.socket.sock", runtime_dir, instance);
        let event_path = format!("{}/hypr/{}/.socket2.sock", runtime_dir, instance);
        
        debug!("Connecting to Hyprland sockets: control={}, event={}", control_path, event_path);
        
        let control_socket = UnixStream::connect(&control_path).await
            .map_err(|e| MCPError::Other(anyhow::anyhow!("Failed to connect to Hyprland control socket: {}", e)))?;
        
        // Event socket is optional
        let event_socket = UnixStream::connect(&event_path).await.ok();
        
        Ok(Self {
            control_socket,
            event_socket,
        })
    }
    
    pub async fn send_command(&mut self, command: &str) -> Result<String> {
        debug!("Sending Hyprland command: {}", command);
        
        // Send command
        self.control_socket.write_all(command.as_bytes()).await?;
        
        // Read response
        let mut buffer = vec![0; 8192];
        let n = self.control_socket.read(&mut buffer).await?;
        buffer.truncate(n);
        
        let response = String::from_utf8(buffer)
            .map_err(|e| MCPError::Other(anyhow::anyhow!("Invalid UTF-8 in response: {}", e)))?;
        
        debug!("Hyprland response: {}", response);
        
        // Reconnect for next command (Hyprland closes connection after each command)
        self.reconnect_control().await?;
        
        Ok(response)
    }
    
    async fn reconnect_control(&mut self) -> Result<()> {
        let runtime_dir = env::var("XDG_RUNTIME_DIR").unwrap();
        let instance = env::var("HYPRLAND_INSTANCE_SIGNATURE").unwrap();
        let control_path = format!("{}/hypr/{}/.socket.sock", runtime_dir, instance);
        
        self.control_socket = UnixStream::connect(&control_path).await
            .map_err(|e| MCPError::Other(anyhow::anyhow!("Failed to reconnect to Hyprland: {}", e)))?;
        
        Ok(())
    }
    
    pub async fn get_windows(&mut self) -> Result<Vec<HyprlandWindow>> {
        let response = self.send_command("j/clients").await?;
        let windows: Vec<HyprlandWindow> = serde_json::from_str(&response)
            .map_err(|e| MCPError::Other(anyhow::anyhow!("Failed to parse windows: {}", e)))?;
        Ok(windows)
    }
    
    pub async fn get_workspaces(&mut self) -> Result<Vec<Value>> {
        let response = self.send_command("j/workspaces").await?;
        let workspaces: Vec<Value> = serde_json::from_str(&response)
            .map_err(|e| MCPError::Other(anyhow::anyhow!("Failed to parse workspaces: {}", e)))?;
        Ok(workspaces)
    }
    
    pub async fn get_monitors(&mut self) -> Result<Vec<HyprlandMonitor>> {
        let response = self.send_command("j/monitors").await?;
        let monitors: Vec<HyprlandMonitor> = serde_json::from_str(&response)
            .map_err(|e| MCPError::Other(anyhow::anyhow!("Failed to parse monitors: {}", e)))?;
        Ok(monitors)
    }
    
    pub async fn get_active_window(&mut self) -> Result<HyprlandWindow> {
        let response = self.send_command("j/activewindow").await?;
        let window: HyprlandWindow = serde_json::from_str(&response)
            .map_err(|e| MCPError::Other(anyhow::anyhow!("Failed to parse active window: {}", e)))?;
        Ok(window)
    }
    
    pub async fn dispatch(&mut self, dispatcher: &str, params: &str) -> Result<String> {
        let command = if params.is_empty() {
            format!("dispatch {}", dispatcher)
        } else {
            format!("dispatch {} {}", dispatcher, params)
        };
        self.send_command(&command).await
    }
    
    pub async fn set_keyword(&mut self, keyword: &str, value: &str) -> Result<String> {
        let command = format!("keyword {} {}", keyword, value);
        self.send_command(&command).await
    }
    
    pub async fn reload_config(&mut self) -> Result<String> {
        self.send_command("reload").await
    }
    
    pub async fn kill_active(&mut self) -> Result<String> {
        self.dispatch("killactive", "").await
    }
    
    pub async fn workspace(&mut self, id: i32) -> Result<String> {
        self.dispatch("workspace", &id.to_string()).await
    }
    
    pub async fn move_to_workspace(&mut self, id: i32) -> Result<String> {
        self.dispatch("movetoworkspace", &id.to_string()).await
    }
    
    pub async fn toggle_floating(&mut self) -> Result<String> {
        self.dispatch("togglefloating", "").await
    }
    
    pub async fn toggle_fullscreen(&mut self) -> Result<String> {
        self.dispatch("fullscreen", "0").await
    }
    
    pub async fn focus_window(&mut self, direction: &str) -> Result<String> {
        self.dispatch("movefocus", direction).await
    }
    
    pub async fn resize_active(&mut self, x: i32, y: i32) -> Result<String> {
        self.dispatch("resizeactive", &format!("{} {}", x, y)).await
    }
    
    pub async fn move_active(&mut self, x: i32, y: i32) -> Result<String> {
        self.dispatch("moveactive", &format!("{} {}", x, y)).await
    }
}

impl HyprlandEvent {
    pub fn parse(line: &str) -> Option<Self> {
        let parts: Vec<&str> = line.splitn(2, ">>").collect();
        if parts.len() != 2 {
            return None;
        }
        
        let event_type = parts[0];
        let data = parts[1];
        
        match event_type {
            "workspace" => Some(HyprlandEvent::Workspace(data.to_string())),
            "activewindow" => {
                let parts: Vec<&str> = data.splitn(2, ',').collect();
                if parts.len() == 2 {
                    Some(HyprlandEvent::ActiveWindow(
                        parts[0].to_string(),
                        parts[1].to_string(),
                    ))
                } else {
                    None
                }
            }
            "fullscreen" => {
                let is_fullscreen = data == "1";
                Some(HyprlandEvent::Fullscreen(is_fullscreen))
            }
            "monitoradded" => Some(HyprlandEvent::MonitorAdded(data.to_string())),
            "monitorremoved" => Some(HyprlandEvent::MonitorRemoved(data.to_string())),
            "createworkspace" => {
                data.parse::<i32>()
                    .ok()
                    .map(HyprlandEvent::CreateWorkspace)
            }
            "destroyworkspace" => {
                data.parse::<i32>()
                    .ok()
                    .map(HyprlandEvent::DestroyWorkspace)
            }
            "moveworkspace" => {
                let parts: Vec<&str> = data.splitn(2, ',').collect();
                if parts.len() == 2 {
                    if let Ok(id) = parts[0].parse::<i32>() {
                        Some(HyprlandEvent::MoveWorkspace(id, parts[1].to_string()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            "openwindow" => {
                let parts: Vec<&str> = data.splitn(4, ',').collect();
                if parts.len() == 4 {
                    Some(HyprlandEvent::OpenWindow(
                        parts[0].to_string(),
                        parts[1].to_string(),
                        parts[2].to_string(),
                        parts[3].to_string(),
                    ))
                } else {
                    None
                }
            }
            "closewindow" => Some(HyprlandEvent::CloseWindow(data.to_string())),
            "movewindow" => {
                let parts: Vec<&str> = data.splitn(2, ',').collect();
                if parts.len() == 2 {
                    Some(HyprlandEvent::MoveWindow(
                        parts[0].to_string(),
                        parts[1].to_string(),
                    ))
                } else {
                    None
                }
            }
            "urgent" => Some(HyprlandEvent::Urgent(data.to_string())),
            "minimize" => {
                let parts: Vec<&str> = data.splitn(2, ',').collect();
                if parts.len() == 2 {
                    let minimized = parts[1] == "1";
                    Some(HyprlandEvent::Minimize(parts[0].to_string(), minimized))
                } else {
                    None
                }
            }
            _ => Some(HyprlandEvent::Other(event_type.to_string(), data.to_string())),
        }
    }
}