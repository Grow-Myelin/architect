use crate::{Result, MCPError};
use crate::system::execute_privileged_command;
use std::path::Path;
use tracing::{info, warn, error};

pub struct PackageManager {
    pacman_conf: Option<String>,
}

impl PackageManager {
    pub fn new() -> Self {
        Self { pacman_conf: None }
    }
    
    pub async fn pacstrap(&self, target: &str, packages: &[String]) -> Result<()> {
        info!("Installing packages to {}: {:?}", target, packages);
        
        let mut args = vec![target];
        for pkg in packages {
            args.push(pkg);
        }
        
        execute_privileged_command("pacstrap", &args, true).await?;
        Ok(())
    }
    
    pub async fn genfstab(&self, target: &str) -> Result<()> {
        info!("Generating fstab for {}", target);
        
        let output = execute_privileged_command("genfstab", &["-U", target], true).await?;
        
        // Write fstab to target
        let fstab_path = format!("{}/etc/fstab", target);
        tokio::fs::write(&fstab_path, output).await?;
        
        Ok(())
    }
    
    pub async fn arch_chroot(&self, target: &str, command: &str) -> Result<String> {
        info!("Executing in chroot: {}", command);
        
        execute_privileged_command("arch-chroot", &[target, "bash", "-c", command], true).await
    }
    
    pub async fn configure_system(
        &self,
        hostname: &str,
        timezone: &str,
        locale: &str,
        root_password: Option<&str>,
    ) -> Result<String> {
        let target = "/mnt";
        let mut results = Vec::new();
        
        // Set timezone
        self.arch_chroot(
            target,
            &format!("ln -sf /usr/share/zoneinfo/{} /etc/localtime", timezone)
        ).await?;
        results.push(format!("Set timezone to {}", timezone));
        
        // Generate /etc/adjtime
        self.arch_chroot(target, "hwclock --systohc").await?;
        results.push("Generated /etc/adjtime".to_string());
        
        // Configure locale
        self.arch_chroot(
            target,
            &format!("echo '{} UTF-8' >> /etc/locale.gen", locale)
        ).await?;
        self.arch_chroot(target, "locale-gen").await?;
        self.arch_chroot(
            target,
            &format!("echo 'LANG={}' > /etc/locale.conf", locale)
        ).await?;
        results.push(format!("Configured locale: {}", locale));
        
        // Set hostname
        self.arch_chroot(
            target,
            &format!("echo '{}' > /etc/hostname", hostname)
        ).await?;
        
        // Configure hosts file
        let hosts_content = format!(
            "127.0.0.1\tlocalhost\n::1\t\tlocalhost\n127.0.1.1\t{}.localdomain\t{}",
            hostname, hostname
        );
        self.arch_chroot(
            target,
            &format!("echo '{}' > /etc/hosts", hosts_content)
        ).await?;
        results.push(format!("Set hostname: {}", hostname));
        
        // Set root password if provided
        if let Some(password) = root_password {
            self.arch_chroot(
                target,
                &format!("echo 'root:{}' | chpasswd", password)
            ).await?;
            results.push("Set root password".to_string());
        }
        
        // Enable essential services
        self.arch_chroot(target, "systemctl enable NetworkManager").await?;
        results.push("Enabled NetworkManager".to_string());
        
        Ok(results.join("\n"))
    }
    
    pub async fn install_grub(&self, device: &str) -> Result<()> {
        let target = "/mnt";
        
        // Install GRUB packages
        self.arch_chroot(target, "pacman -S --noconfirm grub").await?;
        
        // Install GRUB to device
        self.arch_chroot(
            target,
            &format!("grub-install --target=i386-pc {}", device)
        ).await?;
        
        // Generate GRUB configuration
        self.arch_chroot(target, "grub-mkconfig -o /boot/grub/grub.cfg").await?;
        
        Ok(())
    }
    
    pub async fn install_systemd_boot(&self) -> Result<()> {
        let target = "/mnt";
        
        // Install systemd-boot
        self.arch_chroot(target, "bootctl --path=/boot/efi install").await?;
        
        // Create loader configuration
        let loader_conf = "default arch\ntimeout 5\nconsole-mode max\neditor no";
        self.arch_chroot(
            target,
            &format!("echo '{}' > /boot/efi/loader/loader.conf", loader_conf)
        ).await?;
        
        // Create arch entry
        let root_uuid = self.get_root_uuid(target).await?;
        let arch_conf = format!(
            "title Arch Linux\nlinux /vmlinuz-linux\ninitrd /initramfs-linux.img\noptions root=UUID={} rw",
            root_uuid
        );
        
        self.arch_chroot(
            target,
            &format!("mkdir -p /boot/efi/loader/entries && echo '{}' > /boot/efi/loader/entries/arch.conf", arch_conf)
        ).await?;
        
        Ok(())
    }
    
    async fn get_root_uuid(&self, target: &str) -> Result<String> {
        let output = execute_privileged_command(
            "blkid",
            &["-s", "UUID", "-o", "value", "/dev/disk/by-label/root"],
            true
        ).await?;
        
        Ok(output.trim().to_string())
    }
    
    pub async fn is_base_installed(&self, target: &str) -> bool {
        Path::new(&format!("{}/usr/bin/pacman", target)).exists()
    }
    
    pub async fn is_configured(&self, target: &str) -> bool {
        Path::new(&format!("{}/etc/hostname", target)).exists() &&
        Path::new(&format!("{}/etc/locale.conf", target)).exists()
    }
}