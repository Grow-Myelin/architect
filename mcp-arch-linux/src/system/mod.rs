pub mod command;
pub mod disk;
pub mod package;
pub mod hyprland;

use crate::{Result, MCPError};
use std::process::Stdio;
use tokio::process::Command;
use tracing::{info, warn, error};

pub async fn execute_privileged_command(
    command: &str,
    args: &[&str],
    require_root: bool,
) -> Result<String> {
    if require_root && !is_root() {
        return Err(MCPError::PermissionDenied(
            "This operation requires root privileges".to_string()
        ));
    }
    
    info!("Executing privileged command: {} {:?}", command, args);
    
    let output = Command::new(command)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("Command failed: {}", stderr);
        return Err(MCPError::SystemCommand(format!(
            "Command failed: {}",
            stderr
        )));
    }
    
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

pub async fn check_command_exists(command: &str) -> bool {
    Command::new("which")
        .arg(command)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|status| status.success())
        .unwrap_or(false)
}