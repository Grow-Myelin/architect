import { BasePlugin } from './base-plugin.js';
import { CommandExecutor } from '../system/command-executor.js';
import fs from 'fs-extra';
import path from 'path';

export class ArchInstallPlugin extends BasePlugin {
  constructor(config, logger, security) {
    super('arch-install', config, logger, security);
    this.description = 'Arch Linux installation automation';
    this.allowDiskOperations = config.plugins?.archInstall?.allowDiskOperations || true;
    
    this.commandExecutor = new CommandExecutor(
      config.security || {},
      logger,
      security
    );
    
    this.installState = {
      currentStep: null,
      targetMount: '/mnt',
      lastSnapshot: null
    };
    
    this.initializeTools();
    this.initializeResources();
  }

  async initialize() {
    await super.initialize();
    
    if (!this.allowDiskOperations) {
      this.logger.warn('Disk operations are disabled for Arch install plugin');
    }
  }

  initializeTools() {
    this.tools = [
      this.createTool(
        'arch_partition_disk',
        'Partition a disk for Arch Linux installation',
        {
          type: 'object',
          properties: {
            device: {
              type: 'string',
              description: 'Device path (e.g., /dev/sda)',
              pattern: '^/dev/[a-z]+$'
            },
            scheme: {
              type: 'string',
              enum: ['uefi', 'bios'],
              description: 'Partition scheme'
            },
            swapSize: {
              type: 'string',
              description: 'Swap partition size (e.g., 4G, 8G)',
              default: '4G'
            },
            rootSize: {
              type: 'string',
              description: 'Root partition size (e.g., 50G, or "remaining" for all space)',
              default: 'remaining'
            },
            dryRun: {
              type: 'boolean',
              description: 'Preview operations without executing',
              default: false
            }
          },
          required: ['device', 'scheme']
        }
      ),

      this.createTool(
        'arch_install_base',
        'Install Arch Linux base system',
        {
          type: 'object',
          properties: {
            target: {
              type: 'string',
              description: 'Mount point for installation',
              default: '/mnt'
            },
            packages: {
              type: 'array',
              items: { type: 'string' },
              description: 'Additional packages to install',
              default: ['base', 'base-devel', 'linux', 'linux-firmware', 'networkmanager', 'vim']
            },
            mirror: {
              type: 'string',
              description: 'Pacman mirror URL'
            }
          }
        }
      ),

      this.createTool(
        'arch_configure_system',
        'Configure the installed Arch Linux system',
        {
          type: 'object',
          properties: {
            hostname: {
              type: 'string',
              description: 'System hostname',
              pattern: '^[a-zA-Z0-9][a-zA-Z0-9-]*[a-zA-Z0-9]$'
            },
            timezone: {
              type: 'string',
              description: 'Timezone (e.g., Europe/London, America/New_York)'
            },
            locale: {
              type: 'string',
              description: 'System locale',
              default: 'en_US.UTF-8'
            },
            keymap: {
              type: 'string',
              description: 'Console keymap',
              default: 'us'
            },
            users: {
              type: 'array',
              items: {
                type: 'object',
                properties: {
                  username: { type: 'string' },
                  groups: { 
                    type: 'array', 
                    items: { type: 'string' },
                    default: ['wheel']
                  },
                  shell: { type: 'string', default: '/bin/bash' }
                },
                required: ['username']
              },
              description: 'Users to create'
            }
          },
          required: ['hostname', 'timezone']
        }
      ),

      this.createTool(
        'arch_install_bootloader',
        'Install and configure bootloader',
        {
          type: 'object',
          properties: {
            type: {
              type: 'string',
              enum: ['grub', 'systemd-boot'],
              description: 'Bootloader type'
            },
            device: {
              type: 'string',
              description: 'Device for GRUB installation (required for BIOS)'
            },
            target: {
              type: 'string',
              description: 'Target mount point',
              default: '/mnt'
            }
          },
          required: ['type']
        }
      ),

      this.createTool(
        'arch_mount_system',
        'Mount partitions for Arch installation',
        {
          type: 'object',
          properties: {
            device: {
              type: 'string',
              description: 'Base device (e.g., /dev/sda)'
            },
            scheme: {
              type: 'string',
              enum: ['uefi', 'bios'],
              description: 'Partition scheme'
            },
            target: {
              type: 'string',
              description: 'Mount point',
              default: '/mnt'
            }
          },
          required: ['device', 'scheme']
        }
      ),

      this.createTool(
        'arch_list_disks',
        'List available disks for installation',
        {
          type: 'object',
          properties: {
            detailed: {
              type: 'boolean',
              description: 'Include detailed disk information',
              default: false
            }
          }
        }
      ),

      this.createTool(
        'arch_installation_status',
        'Get current installation status and next steps',
        {
          type: 'object',
          properties: {}
        }
      ),

      this.createTool(
        'arch_complete_installation',
        'Finalize Arch Linux installation',
        {
          type: 'object',
          properties: {
            target: {
              type: 'string',
              description: 'Installation target',
              default: '/mnt'
            },
            reboot: {
              type: 'boolean',
              description: 'Reboot after completion',
              default: false
            }
          }
        }
      )
    ];
  }

  initializeResources() {
    this.resources = [
      this.createResource(
        'arch://installation/status',
        'Installation Status',
        'Current Arch Linux installation progress',
        'application/json'
      ),
      this.createResource(
        'arch://installation/log',
        'Installation Log',
        'Detailed installation log',
        'text/plain'
      ),
      this.createResource(
        'arch://disks',
        'Available Disks',
        'List of available disks for installation',
        'application/json'
      )
    ];
  }

  async executeTool(toolName, args) {
    return this.withErrorHandling(async () => {
      switch (toolName) {
        case 'arch_partition_disk':
          return this.handlePartitionDisk(args);
        case 'arch_install_base':
          return this.handleInstallBase(args);
        case 'arch_configure_system':
          return this.handleConfigureSystem(args);
        case 'arch_install_bootloader':
          return this.handleInstallBootloader(args);
        case 'arch_mount_system':
          return this.handleMountSystem(args);
        case 'arch_list_disks':
          return this.handleListDisks(args);
        case 'arch_installation_status':
          return this.handleInstallationStatus(args);
        case 'arch_complete_installation':
          return this.handleCompleteInstallation(args);
        default:
          throw new Error(`Unknown tool: ${toolName}`);
      }
    }, toolName);
  }

  async readResource(uri) {
    return this.withErrorHandling(async () => {
      switch (uri) {
        case 'arch://installation/status':
          return this.getInstallationStatus();
        case 'arch://installation/log':
          return this.getInstallationLog();
        case 'arch://disks':
          return this.getAvailableDisks();
        default:
          throw new Error(`Unknown resource: ${uri}`);
      }
    }, 'readResource');
  }

  async handlePartitionDisk(args) {
    if (!this.allowDiskOperations) {
      throw new Error('Disk operations are disabled');
    }

    await this.validateArgs(args, this.tools[0].inputSchema);
    
    const { device, scheme, swapSize = '4G', rootSize = 'remaining', dryRun = false } = args;
    
    // Safety checks
    await this.validateDevice(device);
    
    if (dryRun) {
      return this.createTextResult(this.previewPartitionOperations(device, scheme, swapSize, rootSize));
    }
    
    // Create snapshot before partitioning
    const snapshotId = await this.security.createSnapshot(
      `Before partitioning ${device}`,
      ['/etc/fstab']
    );
    this.installState.lastSnapshot = snapshotId;
    
    this.installState.currentStep = 'partitioning';
    
    // Wipe existing partition table
    await this.commandExecutor.executeWithSudo('wipefs', ['-a', device]);
    
    if (scheme === 'uefi') {
      await this.createUEFIPartitions(device, swapSize, rootSize);
    } else {
      await this.createBIOSPartitions(device, swapSize, rootSize);
    }
    
    // Format partitions
    await this.formatPartitions(device, scheme);
    
    this.installState.currentStep = 'partitioned';
    
    return this.createTextResult(`Successfully partitioned ${device} with ${scheme} scheme`, {
      device,
      scheme,
      swapSize,
      snapshotId
    });
  }

  async createUEFIPartitions(device, swapSize, rootSize) {
    // Create GPT partition table
    await this.commandExecutor.executeWithSudo('sgdisk', ['-o', device]);
    
    // EFI partition (512MB)
    await this.commandExecutor.executeWithSudo('sgdisk', [
      '-n', '1:0:+512M',
      '-t', '1:ef00',
      '-c', '1:EFI',
      device
    ]);
    
    // Swap partition
    await this.commandExecutor.executeWithSudo('sgdisk', [
      '-n', `2:0:+${swapSize}`,
      '-t', '2:8200',
      '-c', '2:swap',
      device
    ]);
    
    // Root partition
    const rootArgs = rootSize === 'remaining' ? 
      ['-n', '3:0:0'] : 
      ['-n', `3:0:+${rootSize}`];
    
    await this.commandExecutor.executeWithSudo('sgdisk', [
      ...rootArgs,
      '-t', '3:8300',
      '-c', '3:root',
      device
    ]);
  }

  async createBIOSPartitions(device, swapSize, rootSize) {
    // Create MBR partition table using fdisk
    const commands = [
      'o', // Create new empty DOS partition table
      'n', 'p', '1', '', `+${swapSize}`, // Swap partition
      't', '82', // Set type to swap
      'n', 'p', '2', '', rootSize === 'remaining' ? '' : `+${rootSize}`, // Root partition
      'w' // Write changes
    ];
    
    const fdiskInput = commands.join('\n') + '\n';
    
    await this.commandExecutor.executeWithSudo('fdisk', [device], {
      input: fdiskInput
    });
  }

  async formatPartitions(device, scheme) {
    if (scheme === 'uefi') {
      // Format EFI partition
      await this.commandExecutor.executeWithSudo('mkfs.fat', ['-F32', `${device}1`]);
      
      // Format swap
      await this.commandExecutor.executeWithSudo('mkswap', [`${device}2`]);
      
      // Format root
      await this.commandExecutor.executeWithSudo('mkfs.ext4', ['-F', `${device}3`]);
    } else {
      // Format swap
      await this.commandExecutor.executeWithSudo('mkswap', [`${device}1`]);
      
      // Format root
      await this.commandExecutor.executeWithSudo('mkfs.ext4', ['-F', `${device}2`]);
    }
  }

  async handleMountSystem(args) {
    await this.validateArgs(args, this.tools[4].inputSchema);
    
    const { device, scheme, target = '/mnt' } = args;
    
    // Create mount point
    await this.commandExecutor.executeWithSudo('mkdir', ['-p', target]);
    
    if (scheme === 'uefi') {
      // Mount root
      await this.commandExecutor.executeWithSudo('mount', [`${device}3`, target]);
      
      // Create and mount EFI
      await this.commandExecutor.executeWithSudo('mkdir', ['-p', `${target}/boot/efi`]);
      await this.commandExecutor.executeWithSudo('mount', [`${device}1`, `${target}/boot/efi`]);
      
      // Enable swap
      await this.commandExecutor.executeWithSudo('swapon', [`${device}2`]);
    } else {
      // Mount root
      await this.commandExecutor.executeWithSudo('mount', [`${device}2`, target]);
      
      // Enable swap
      await this.commandExecutor.executeWithSudo('swapon', [`${device}1`]);
    }
    
    this.installState.targetMount = target;
    this.installState.currentStep = 'mounted';
    
    return this.createTextResult(`Successfully mounted ${device} to ${target}`);
  }

  async handleInstallBase(args) {
    await this.validateArgs(args, this.tools[1].inputSchema);
    
    const { 
      target = '/mnt', 
      packages = ['base', 'base-devel', 'linux', 'linux-firmware', 'networkmanager', 'vim'],
      mirror 
    } = args;
    
    this.installState.currentStep = 'installing_base';
    
    // Update pacman mirrors if specified
    if (mirror) {
      const mirrorlist = `Server = ${mirror}\n`;
      await fs.writeFile('/etc/pacman.d/mirrorlist', mirrorlist);
    }
    
    // Install base system
    await this.commandExecutor.executeWithSudo('pacstrap', [target, ...packages], {
      timeout: 1800000 // 30 minutes
    });
    
    // Generate fstab
    const fstabResult = await this.commandExecutor.executeWithSudo('genfstab', ['-U', target]);
    await fs.writeFile(`${target}/etc/fstab`, fstabResult.stdout);
    
    this.installState.currentStep = 'base_installed';
    
    return this.createTextResult(`Successfully installed base system with ${packages.length} packages`, {
      packages,
      target
    });
  }

  async handleConfigureSystem(args) {
    await this.validateArgs(args, this.tools[2].inputSchema);
    
    const { 
      hostname, 
      timezone, 
      locale = 'en_US.UTF-8', 
      keymap = 'us',
      users = []
    } = args;
    
    const target = this.installState.targetMount;
    this.installState.currentStep = 'configuring';
    
    // Set timezone
    await this.archChroot(target, `ln -sf /usr/share/zoneinfo/${timezone} /etc/localtime`);
    await this.archChroot(target, 'hwclock --systohc');
    
    // Configure locale
    await this.archChroot(target, `echo '${locale} UTF-8' >> /etc/locale.gen`);
    await this.archChroot(target, 'locale-gen');
    await this.archChroot(target, `echo 'LANG=${locale}' > /etc/locale.conf`);
    
    // Set keymap
    await this.archChroot(target, `echo 'KEYMAP=${keymap}' > /etc/vconsole.conf`);
    
    // Set hostname
    await this.archChroot(target, `echo '${hostname}' > /etc/hostname`);
    
    // Configure hosts
    const hostsContent = `127.0.0.1\tlocalhost\n::1\t\tlocalhost\n127.0.1.1\t${hostname}.localdomain\t${hostname}`;
    await this.archChroot(target, `echo '${hostsContent}' > /etc/hosts`);
    
    // Enable NetworkManager
    await this.archChroot(target, 'systemctl enable NetworkManager');
    
    // Create users
    for (const user of users) {
      await this.archChroot(target, `useradd -m -G ${user.groups.join(',')} -s ${user.shell} ${user.username}`);
      this.logger.info(`Created user: ${user.username}`);
    }
    
    // Enable sudo for wheel group
    await this.archChroot(target, "sed -i 's/# %wheel ALL=(ALL:ALL) ALL/%wheel ALL=(ALL:ALL) ALL/' /etc/sudoers");
    
    this.installState.currentStep = 'configured';
    
    return this.createTextResult(`System configured: ${hostname}`, {
      hostname,
      timezone,
      locale,
      usersCreated: users.length
    });
  }

  async handleInstallBootloader(args) {
    await this.validateArgs(args, this.tools[3].inputSchema);
    
    const { type, device, target = '/mnt' } = args;
    
    this.installState.currentStep = 'installing_bootloader';
    
    if (type === 'grub') {
      await this.installGRUB(target, device);
    } else if (type === 'systemd-boot') {
      await this.installSystemdBoot(target);
    }
    
    this.installState.currentStep = 'bootloader_installed';
    
    return this.createTextResult(`Successfully installed ${type} bootloader`);
  }

  async installGRUB(target, device) {
    // Install GRUB package
    await this.archChroot(target, 'pacman -S --noconfirm grub');
    
    // Install GRUB (determine if UEFI or BIOS)
    const efiDir = `${target}/boot/efi`;
    if (await fs.pathExists(efiDir)) {
      // UEFI installation
      await this.archChroot(target, 'pacman -S --noconfirm efibootmgr');
      await this.archChroot(target, 'grub-install --target=x86_64-efi --efi-directory=/boot/efi --bootloader-id=GRUB');
    } else {
      // BIOS installation
      if (!device) {
        throw new Error('Device required for BIOS GRUB installation');
      }
      await this.archChroot(target, `grub-install --target=i386-pc ${device}`);
    }
    
    // Generate GRUB config
    await this.archChroot(target, 'grub-mkconfig -o /boot/grub/grub.cfg');
  }

  async installSystemdBoot(target) {
    // Install systemd-boot
    await this.archChroot(target, 'bootctl --path=/boot/efi install');
    
    // Create loader configuration
    const loaderConf = `default arch\ntimeout 5\nconsole-mode max\neditor no`;
    await this.archChroot(target, `echo '${loaderConf}' > /boot/efi/loader/loader.conf`);
    
    // Get root UUID
    const rootUuid = await this.getRootUUID(target);
    
    // Create Arch entry
    const archConf = `title Arch Linux\nlinux /vmlinuz-linux\ninitrd /initramfs-linux.img\noptions root=UUID=${rootUuid} rw`;
    await this.archChroot(target, `mkdir -p /boot/efi/loader/entries`);
    await this.archChroot(target, `echo '${archConf}' > /boot/efi/loader/entries/arch.conf`);
  }

  async handleListDisks(args) {
    const { detailed = false } = args;
    
    const result = await this.commandExecutor.execute('lsblk', ['-J', '-o', 'NAME,SIZE,TYPE,MOUNTPOINT,MODEL']);
    const disks = JSON.parse(result.stdout);
    
    if (detailed) {
      // Add additional disk information
      for (const disk of disks.blockdevices || []) {
        if (disk.type === 'disk') {
          try {
            const smartInfo = await this.commandExecutor.execute('smartctl', ['-i', `/dev/${disk.name}`]);
            disk.smart = smartInfo.stdout;
          } catch {
            // SMART info not available
          }
        }
      }
    }
    
    return this.createTextResult(JSON.stringify(disks, null, 2));
  }

  async handleInstallationStatus(args) {
    const status = {
      currentStep: this.installState.currentStep,
      targetMount: this.installState.targetMount,
      lastSnapshot: this.installState.lastSnapshot,
      nextSteps: this.getNextSteps()
    };
    
    return this.createTextResult(JSON.stringify(status, null, 2));
  }

  async handleCompleteInstallation(args) {
    const { target = '/mnt', reboot = false } = args;
    
    this.installState.currentStep = 'finalizing';
    
    // Final steps
    await this.archChroot(target, 'systemctl enable NetworkManager');
    
    // Unmount filesystems
    await this.commandExecutor.executeWithSudo('umount', ['-R', target]);
    
    this.installState.currentStep = 'completed';
    
    let message = 'Arch Linux installation completed successfully!';
    
    if (reboot) {
      message += ' Rebooting system...';
      // Give some time for response before reboot
      setTimeout(() => {
        this.commandExecutor.executeWithSudo('reboot');
      }, 5000);
    }
    
    return this.createTextResult(message);
  }

  // Helper methods
  async archChroot(target, command) {
    return this.commandExecutor.executeWithSudo('arch-chroot', [target, 'bash', '-c', command]);
  }

  async validateDevice(device) {
    // Check if device exists
    if (!await fs.pathExists(device)) {
      throw new Error(`Device not found: ${device}`);
    }
    
    // Basic safety check - ensure it's a block device
    const result = await this.commandExecutor.execute('lsblk', [device]);
    if (!result.success) {
      throw new Error(`Invalid device: ${device}`);
    }
  }

  previewPartitionOperations(device, scheme, swapSize, rootSize) {
    let preview = `Partition operations for ${device} (${scheme}):\n\n`;
    
    if (scheme === 'uefi') {
      preview += `1. Create GPT partition table\n`;
      preview += `2. Create EFI partition (512MB)\n`;
      preview += `3. Create swap partition (${swapSize})\n`;
      preview += `4. Create root partition (${rootSize})\n`;
      preview += `5. Format EFI as FAT32\n`;
      preview += `6. Format swap\n`;
      preview += `7. Format root as ext4\n`;
    } else {
      preview += `1. Create MBR partition table\n`;
      preview += `2. Create swap partition (${swapSize})\n`;
      preview += `3. Create root partition (${rootSize})\n`;
      preview += `4. Format swap\n`;
      preview += `5. Format root as ext4\n`;
    }
    
    preview += `\nWARNING: This will destroy all data on ${device}`;
    
    return preview;
  }

  async getRootUUID(target) {
    const result = await this.commandExecutor.execute('findmnt', ['-n', '-o', 'UUID', target]);
    return result.stdout.trim();
  }

  getNextSteps() {
    switch (this.installState.currentStep) {
      case null:
        return ['List available disks', 'Partition disk'];
      case 'partitioned':
        return ['Mount system', 'Install base system'];
      case 'mounted':
        return ['Install base system'];
      case 'base_installed':
        return ['Configure system'];
      case 'configured':
        return ['Install bootloader'];
      case 'bootloader_installed':
        return ['Complete installation'];
      case 'completed':
        return ['Installation complete'];
      default:
        return ['Check installation status'];
    }
  }

  async getInstallationStatus() {
    const status = {
      currentStep: this.installState.currentStep,
      targetMount: this.installState.targetMount,
      nextSteps: this.getNextSteps(),
      timestamp: new Date().toISOString()
    };
    
    return { content: JSON.stringify(status, null, 2) };
  }

  async getInstallationLog() {
    // Return recent system logs related to installation
    try {
      const result = await this.commandExecutor.execute('journalctl', ['-n', '100', '--no-pager', '-u', 'pacstrap']);
      return { content: result.stdout };
    } catch {
      return { content: 'No installation logs available' };
    }
  }

  async getAvailableDisks() {
    const result = await this.commandExecutor.execute('lsblk', ['-J', '-o', 'NAME,SIZE,TYPE,MOUNTPOINT']);
    return { content: result.stdout };
  }
}