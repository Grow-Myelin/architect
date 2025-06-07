use crate::{Result, MCPError};
use crate::system::execute_privileged_command;
use std::path::Path;
use tracing::{info, warn, error};

pub struct DiskManager {
    dry_run: bool,
}

impl DiskManager {
    pub fn new() -> Self {
        Self { dry_run: false }
    }
    
    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }
    
    pub async fn partition_uefi(&self, device: &str, swap_size: &str) -> Result<()> {
        info!("Creating UEFI partition scheme on {}", device);
        
        if self.dry_run {
            info!("DRY RUN: Would create UEFI partitions on {}", device);
            return Ok(());
        }
        
        // Wipe existing partition table
        execute_privileged_command("sgdisk", &["--zap-all", device], true).await?;
        
        // Create GPT partition table
        execute_privileged_command("sgdisk", &["-o", device], true).await?;
        
        // Create EFI partition (512MB)
        execute_privileged_command(
            "sgdisk",
            &["-n", "1:0:+512M", "-t", "1:ef00", "-c", "1:EFI", device],
            true
        ).await?;
        
        // Create swap partition
        execute_privileged_command(
            "sgdisk",
            &["-n", &format!("2:0:+{}", swap_size), "-t", "2:8200", "-c", "2:swap", device],
            true
        ).await?;
        
        // Create root partition (remaining space)
        execute_privileged_command(
            "sgdisk",
            &["-n", "3:0:0", "-t", "3:8300", "-c", "3:root", device],
            true
        ).await?;
        
        // Format partitions
        self.format_partitions_uefi(device).await?;
        
        Ok(())
    }
    
    pub async fn partition_bios(&self, device: &str, swap_size: &str) -> Result<()> {
        info!("Creating BIOS partition scheme on {}", device);
        
        if self.dry_run {
            info!("DRY RUN: Would create BIOS partitions on {}", device);
            return Ok(());
        }
        
        // Create MBR partition table using fdisk
        let fdisk_cmds = format!(
            "o\nn\np\n1\n\n+{}\nt\n82\nn\np\n2\n\n\nw\n",
            swap_size
        );
        
        execute_privileged_command(
            "sh",
            &["-c", &format!("echo '{}' | fdisk {}", fdisk_cmds, device)],
            true
        ).await?;
        
        // Format partitions
        self.format_partitions_bios(device).await?;
        
        Ok(())
    }
    
    async fn format_partitions_uefi(&self, device: &str) -> Result<()> {
        // Format EFI partition
        let efi_part = format!("{}1", device);
        execute_privileged_command("mkfs.fat", &["-F32", &efi_part], true).await?;
        
        // Format swap partition
        let swap_part = format!("{}2", device);
        execute_privileged_command("mkswap", &[&swap_part], true).await?;
        
        // Format root partition
        let root_part = format!("{}3", device);
        execute_privileged_command("mkfs.ext4", &["-F", &root_part], true).await?;
        
        Ok(())
    }
    
    async fn format_partitions_bios(&self, device: &str) -> Result<()> {
        // Format swap partition
        let swap_part = format!("{}1", device);
        execute_privileged_command("mkswap", &[&swap_part], true).await?;
        
        // Format root partition
        let root_part = format!("{}2", device);
        execute_privileged_command("mkfs.ext4", &["-F", &root_part], true).await?;
        
        Ok(())
    }
    
    pub async fn mount_partitions(&self, device: &str, target: &str, uefi: bool) -> Result<()> {
        info!("Mounting partitions to {}", target);
        
        // Create mount point
        tokio::fs::create_dir_all(target).await?;
        
        if uefi {
            // Mount root partition
            let root_part = format!("{}3", device);
            execute_privileged_command("mount", &[&root_part, target], true).await?;
            
            // Create and mount EFI partition
            let efi_mount = format!("{}/boot/efi", target);
            tokio::fs::create_dir_all(&efi_mount).await?;
            let efi_part = format!("{}1", device);
            execute_privileged_command("mount", &[&efi_part, &efi_mount], true).await?;
            
            // Enable swap
            let swap_part = format!("{}2", device);
            execute_privileged_command("swapon", &[&swap_part], true).await?;
        } else {
            // Mount root partition
            let root_part = format!("{}2", device);
            execute_privileged_command("mount", &[&root_part, target], true).await?;
            
            // Enable swap
            let swap_part = format!("{}1", device);
            execute_privileged_command("swapon", &[&swap_part], true).await?;
        }
        
        Ok(())
    }
    
    pub async fn is_target_mounted(&self, target: &str) -> bool {
        let output = execute_privileged_command("mountpoint", &["-q", target], false)
            .await
            .map(|_| true)
            .unwrap_or(false);
        output
    }
    
    pub async fn unmount_all(&self, target: &str) -> Result<()> {
        info!("Unmounting all partitions from {}", target);
        
        // Disable swap
        execute_privileged_command("swapoff", &["-a"], true).await.ok();
        
        // Unmount recursively
        execute_privileged_command("umount", &["-R", target], true).await?;
        
        Ok(())
    }
}