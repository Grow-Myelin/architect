use super::MCPPlugin;
use crate::{Result, MCPError};
use crate::mcp::{Tool, Resource, MCPToolResult, ToolArgs, MCPContent};
use crate::system::disk::DiskManager;
use crate::system::package::PackageManager;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::Path;
use tracing::{info, warn, error};

pub struct ArchInstallPlugin {
    disk_manager: DiskManager,
    package_manager: PackageManager,
}

impl ArchInstallPlugin {
    pub fn new() -> Self {
        Self {
            disk_manager: DiskManager::new(),
            package_manager: PackageManager::new(),
        }
    }
}

#[async_trait]
impl MCPPlugin for ArchInstallPlugin {
    fn name(&self) -> &str {
        "arch_install"
    }
    
    fn tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                name: "arch_install_partition".to_string(),
                description: "Partition a disk for Arch Linux installation".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "device": {
                            "type": "string",
                            "description": "Device path (e.g., /dev/sda)"
                        },
                        "scheme": {
                            "type": "string",
                            "enum": ["uefi", "bios"],
                            "description": "Partition scheme"
                        },
                        "swap_size": {
                            "type": "string",
                            "description": "Swap partition size (e.g., 4G)",
                            "default": "4G"
                        }
                    },
                    "required": ["device", "scheme"]
                }),
            },
            Tool {
                name: "arch_install_base".to_string(),
                description: "Install Arch Linux base system".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "target": {
                            "type": "string",
                            "description": "Mount point for installation",
                            "default": "/mnt"
                        },
                        "packages": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Additional packages to install",
                            "default": []
                        }
                    }
                }),
            },
            Tool {
                name: "arch_install_configure".to_string(),
                description: "Configure Arch Linux system".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "hostname": {
                            "type": "string",
                            "description": "System hostname"
                        },
                        "timezone": {
                            "type": "string",
                            "description": "Timezone (e.g., Europe/London)"
                        },
                        "locale": {
                            "type": "string",
                            "description": "System locale",
                            "default": "en_US.UTF-8"
                        },
                        "root_password": {
                            "type": "string",
                            "description": "Root password (will be hashed)"
                        }
                    },
                    "required": ["hostname", "timezone"]
                }),
            },
            Tool {
                name: "arch_install_bootloader".to_string(),
                description: "Install and configure bootloader".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "type": {
                            "type": "string",
                            "enum": ["grub", "systemd-boot"],
                            "description": "Bootloader type"
                        },
                        "device": {
                            "type": "string",
                            "description": "Device for bootloader (required for GRUB on BIOS)"
                        }
                    },
                    "required": ["type"]
                }),
            },
        ]
    }
    
    fn resources(&self) -> Vec<Resource> {
        vec![
            Resource {
                uri: "arch://installation/status".to_string(),
                name: "Installation Status".to_string(),
                description: Some("Current Arch Linux installation status".to_string()),
                mime_type: Some("application/json".to_string()),
            },
            Resource {
                uri: "arch://installation/log".to_string(),
                name: "Installation Log".to_string(),
                description: Some("Arch Linux installation log".to_string()),
                mime_type: Some("text/plain".to_string()),
            },
        ]
    }
    
    async fn handle_tool_call(&self, tool: &str, args: ToolArgs) -> Result<MCPToolResult> {
        match tool {
            "arch_install_partition" => self.handle_partition(args).await,
            "arch_install_base" => self.handle_install_base(args).await,
            "arch_install_configure" => self.handle_configure(args).await,
            "arch_install_bootloader" => self.handle_bootloader(args).await,
            _ => Err(MCPError::Other(anyhow::anyhow!("Unknown tool: {}", tool))),
        }
    }
    
    async fn handle_resource_read(&self, uri: &str) -> Result<String> {
        match uri {
            "arch://installation/status" => self.get_installation_status().await,
            "arch://installation/log" => self.get_installation_log().await,
            _ => Err(MCPError::Other(anyhow::anyhow!("Unknown resource: {}", uri))),
        }
    }
}

impl ArchInstallPlugin {
    async fn handle_partition(&self, args: ToolArgs) -> Result<MCPToolResult> {
        let device = args.args.get("device")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MCPError::Other(anyhow::anyhow!("Missing device parameter")))?;
        
        let scheme = args.args.get("scheme")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MCPError::Other(anyhow::anyhow!("Missing scheme parameter")))?;
        
        let swap_size = args.args.get("swap_size")
            .and_then(|v| v.as_str())
            .unwrap_or("4G");
        
        info!("Partitioning disk {} with {} scheme", device, scheme);
        
        // Validate device exists
        if !Path::new(device).exists() {
            return Ok(MCPToolResult::error(format!("Device {} not found", device)));
        }
        
        // Create partitions based on scheme
        match scheme {
            "uefi" => {
                self.disk_manager.partition_uefi(device, swap_size).await?;
            }
            "bios" => {
                self.disk_manager.partition_bios(device, swap_size).await?;
            }
            _ => {
                return Ok(MCPToolResult::error(format!("Invalid partition scheme: {}", scheme)));
            }
        }
        
        Ok(MCPToolResult::text(format!(
            "Successfully partitioned {} with {} scheme and {} swap",
            device, scheme, swap_size
        )))
    }
    
    async fn handle_install_base(&self, args: ToolArgs) -> Result<MCPToolResult> {
        let target = args.args.get("target")
            .and_then(|v| v.as_str())
            .unwrap_or("/mnt");
        
        let additional_packages = args.args.get("packages")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(String::from)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        
        info!("Installing Arch Linux base system to {}", target);
        
        // Install base packages
        let mut packages = vec![
            "base".to_string(),
            "base-devel".to_string(),
            "linux".to_string(),
            "linux-firmware".to_string(),
            "networkmanager".to_string(),
            "vim".to_string(),
        ];
        packages.extend(additional_packages);
        
        self.package_manager.pacstrap(target, &packages).await?;
        
        // Generate fstab
        self.package_manager.genfstab(target).await?;
        
        Ok(MCPToolResult::text(format!(
            "Successfully installed Arch Linux base system with {} packages",
            packages.len()
        )))
    }
    
    async fn handle_configure(&self, args: ToolArgs) -> Result<MCPToolResult> {
        let hostname = args.args.get("hostname")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MCPError::Other(anyhow::anyhow!("Missing hostname parameter")))?;
        
        let timezone = args.args.get("timezone")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MCPError::Other(anyhow::anyhow!("Missing timezone parameter")))?;
        
        let locale = args.args.get("locale")
            .and_then(|v| v.as_str())
            .unwrap_or("en_US.UTF-8");
        
        info!("Configuring system: hostname={}, timezone={}, locale={}", 
              hostname, timezone, locale);
        
        // Configure in chroot
        let config_result = self.package_manager.configure_system(
            hostname,
            timezone,
            locale,
            args.args.get("root_password").and_then(|v| v.as_str()),
        ).await?;
        
        Ok(MCPToolResult::text(config_result))
    }
    
    async fn handle_bootloader(&self, args: ToolArgs) -> Result<MCPToolResult> {
        let bootloader_type = args.args.get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MCPError::Other(anyhow::anyhow!("Missing type parameter")))?;
        
        let device = args.args.get("device")
            .and_then(|v| v.as_str());
        
        info!("Installing {} bootloader", bootloader_type);
        
        match bootloader_type {
            "grub" => {
                if device.is_none() {
                    return Ok(MCPToolResult::error("Device parameter required for GRUB"));
                }
                self.package_manager.install_grub(device.unwrap()).await?;
            }
            "systemd-boot" => {
                self.package_manager.install_systemd_boot().await?;
            }
            _ => {
                return Ok(MCPToolResult::error(format!("Invalid bootloader type: {}", bootloader_type)));
            }
        }
        
        Ok(MCPToolResult::text(format!(
            "Successfully installed {} bootloader",
            bootloader_type
        )))
    }
    
    async fn get_installation_status(&self) -> Result<String> {
        let status = json!({
            "mounted": self.disk_manager.is_target_mounted("/mnt").await,
            "base_installed": self.package_manager.is_base_installed("/mnt").await,
            "configured": self.package_manager.is_configured("/mnt").await,
        });
        
        Ok(serde_json::to_string_pretty(&status)?)
    }
    
    async fn get_installation_log(&self) -> Result<String> {
        // Read installation log if it exists
        tokio::fs::read_to_string("/var/log/arch-install.log")
            .await
            .unwrap_or_else(|_| "No installation log available".to_string())
    }
}