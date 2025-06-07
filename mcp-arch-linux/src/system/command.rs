use crate::{Result, MCPError};
use crate::security::AuditableOperation;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{info, warn, error, debug};
use command_group::AsyncCommandGroup;

pub struct CommandExecutor {
    timeout_duration: Duration,
    max_output_size: usize,
}

impl CommandExecutor {
    pub fn new() -> Self {
        Self {
            timeout_duration: Duration::from_secs(300), // 5 minutes default
            max_output_size: 10 * 1024 * 1024, // 10MB
        }
    }
    
    pub async fn execute(&self, cmd: &str, args: &[&str]) -> Result<CommandResult> {
        info!("Executing command: {} {:?}", cmd, args);
        
        let mut command = Command::new(cmd);
        command.args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        
        let mut child = command.group_spawn()
            .map_err(|e| MCPError::SystemCommand(format!("Failed to spawn command: {}", e)))?;
        
        // Execute with timeout
        let output = match timeout(self.timeout_duration, child.wait_with_output()).await {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => return Err(MCPError::SystemCommand(format!("Command error: {}", e))),
            Err(_) => {
                // Timeout occurred, kill the process group
                child.kill().await.ok();
                return Err(MCPError::SystemCommand("Command timed out".to_string()));
            }
        };
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        // Check output size
        if stdout.len() + stderr.len() > self.max_output_size {
            warn!("Command output exceeded size limit");
            return Ok(CommandResult {
                success: output.status.success(),
                stdout: stdout.chars().take(self.max_output_size / 2).collect(),
                stderr: stderr.chars().take(self.max_output_size / 2).collect(),
                exit_code: output.status.code(),
                truncated: true,
            });
        }
        
        Ok(CommandResult {
            success: output.status.success(),
            stdout: stdout.to_string(),
            stderr: stderr.to_string(),
            exit_code: output.status.code(),
            truncated: false,
        })
    }
    
    pub async fn execute_script(&self, script: &str) -> Result<CommandResult> {
        self.execute("bash", &["-c", script]).await
    }
    
    pub async fn execute_with_env(
        &self,
        cmd: &str,
        args: &[&str],
        env: &[(String, String)],
    ) -> Result<CommandResult> {
        info!("Executing command with env: {} {:?}", cmd, args);
        
        let mut command = Command::new(cmd);
        command.args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        
        // Set environment variables
        for (key, value) in env {
            command.env(key, value);
        }
        
        let output = timeout(self.timeout_duration, command.output()).await
            .map_err(|_| MCPError::SystemCommand("Command timed out".to_string()))?
            .map_err(|e| MCPError::SystemCommand(format!("Command error: {}", e)))?;
        
        Ok(CommandResult {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code(),
            truncated: false,
        })
    }
}

#[derive(Debug, Clone)]
pub struct CommandResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub truncated: bool,
}

impl CommandResult {
    pub fn to_string(&self) -> String {
        if self.success {
            self.stdout.clone()
        } else {
            format!("Error: {}", self.stderr)
        }
    }
}

pub struct SandboxedExecutor {
    base_executor: CommandExecutor,
    allowed_commands: Vec<String>,
}

impl SandboxedExecutor {
    pub fn new(allowed_commands: Vec<String>) -> Self {
        Self {
            base_executor: CommandExecutor::new(),
            allowed_commands,
        }
    }
    
    pub async fn execute(&self, cmd: &str, args: &[&str]) -> Result<CommandResult> {
        // Check if command is allowed
        if !self.allowed_commands.iter().any(|allowed| allowed == cmd) {
            return Err(MCPError::PermissionDenied(
                format!("Command '{}' is not allowed", cmd)
            ));
        }
        
        // Validate arguments for potential security issues
        for arg in args {
            if arg.contains("..") || arg.contains("~") {
                return Err(MCPError::PermissionDenied(
                    "Path traversal in arguments not allowed".to_string()
                ));
            }
        }
        
        self.base_executor.execute(cmd, args).await
    }
}