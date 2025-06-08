import { BasePlugin } from './base-plugin.js';
import { CommandExecutor } from '../system/command-executor.js';
import si from 'systeminformation';

export class SystemPlugin extends BasePlugin {
  constructor(config, logger, security) {
    super('system', config, logger, security);
    this.description = 'System management and monitoring plugin';
    this.commandExecutor = new CommandExecutor(
      config.security || {},
      logger,
      security
    );
    
    this.initializeTools();
    this.initializeResources();
  }

  initializeTools() {
    this.tools = [
      this.createTool(
        'system_exec',
        'Execute a system command with proper security controls',
        {
          type: 'object',
          properties: {
            command: {
              type: 'string',
              description: 'Command to execute'
            },
            args: {
              type: 'array',
              items: { type: 'string' },
              description: 'Command arguments',
              default: []
            },
            requireRoot: {
              type: 'boolean',
              description: 'Whether command requires root privileges',
              default: false
            },
            timeout: {
              type: 'number',
              description: 'Timeout in milliseconds',
              default: 300000
            }
          },
          required: ['command']
        }
      ),

      this.createTool(
        'system_info',
        'Get comprehensive system information',
        {
          type: 'object',
          properties: {
            detailed: {
              type: 'boolean',
              description: 'Include detailed hardware information',
              default: false
            }
          }
        }
      ),

      this.createTool(
        'system_services',
        'Manage systemd services',
        {
          type: 'object',
          properties: {
            action: {
              type: 'string',
              enum: ['list', 'status', 'start', 'stop', 'restart', 'enable', 'disable'],
              description: 'Action to perform'
            },
            service: {
              type: 'string',
              description: 'Service name (required for actions other than list)'
            }
          },
          required: ['action']
        }
      ),

      this.createTool(
        'system_package',
        'Manage system packages using pacman',
        {
          type: 'object',
          properties: {
            action: {
              type: 'string',
              enum: ['update', 'upgrade', 'install', 'remove', 'search', 'info'],
              description: 'Package action to perform'
            },
            packages: {
              type: 'array',
              items: { type: 'string' },
              description: 'Package names'
            },
            noconfirm: {
              type: 'boolean',
              description: 'Skip confirmation prompts',
              default: false
            }
          },
          required: ['action']
        }
      ),

      this.createTool(
        'system_snapshot',
        'Create a system state snapshot for rollback',
        {
          type: 'object',
          properties: {
            description: {
              type: 'string',
              description: 'Snapshot description'
            },
            files: {
              type: 'array',
              items: { type: 'string' },
              description: 'Files to include in snapshot',
              default: [
                '/etc/pacman.conf',
                '/etc/fstab',
                '/etc/hostname',
                '/etc/hosts',
                '/etc/locale.conf'
              ]
            }
          },
          required: ['description']
        }
      ),

      this.createTool(
        'system_rollback',
        'Rollback to a previous system snapshot',
        {
          type: 'object',
          properties: {
            snapshotId: {
              type: 'string',
              description: 'Snapshot ID to rollback to'
            }
          },
          required: ['snapshotId']
        }
      ),

      this.createTool(
        'system_process',
        'Manage system processes',
        {
          type: 'object',
          properties: {
            action: {
              type: 'string',
              enum: ['list', 'kill', 'info'],
              description: 'Process action to perform'
            },
            pid: {
              type: 'number',
              description: 'Process ID (for kill/info actions)'
            },
            signal: {
              type: 'string',
              description: 'Signal to send (for kill action)',
              default: 'TERM'
            },
            filter: {
              type: 'string',
              description: 'Filter processes by name (for list action)'
            }
          },
          required: ['action']
        }
      )
    ];
  }

  initializeResources() {
    this.resources = [
      this.createResource(
        'system://info',
        'System Information',
        'Current system status and hardware information',
        'application/json'
      ),
      this.createResource(
        'system://logs',
        'System Logs',
        'Recent system logs from journalctl',
        'text/plain'
      ),
      this.createResource(
        'system://services',
        'Service Status',
        'Status of all systemd services',
        'application/json'
      ),
      this.createResource(
        'system://snapshots',
        'System Snapshots',
        'List of available system snapshots',
        'application/json'
      ),
      this.createResource(
        'system://processes',
        'Running Processes',
        'List of currently running processes',
        'application/json'
      )
    ];
  }

  async executeTool(toolName, args) {
    return this.withErrorHandling(async () => {
      switch (toolName) {
        case 'system_exec':
          return this.handleSystemExec(args);
        case 'system_info':
          return this.handleSystemInfo(args);
        case 'system_services':
          return this.handleSystemServices(args);
        case 'system_package':
          return this.handleSystemPackage(args);
        case 'system_snapshot':
          return this.handleSystemSnapshot(args);
        case 'system_rollback':
          return this.handleSystemRollback(args);
        case 'system_process':
          return this.handleSystemProcess(args);
        default:
          throw new Error(`Unknown tool: ${toolName}`);
      }
    }, toolName);
  }

  async readResource(uri) {
    return this.withErrorHandling(async () => {
      switch (uri) {
        case 'system://info':
          return this.getSystemInfo();
        case 'system://logs':
          return this.getSystemLogs();
        case 'system://services':
          return this.getServiceStatus();
        case 'system://snapshots':
          return this.getSnapshots();
        case 'system://processes':
          return this.getProcesses();
        default:
          throw new Error(`Unknown resource: ${uri}`);
      }
    }, 'readResource');
  }

  async handleSystemExec(args) {
    await this.validateArgs(args, this.tools[0].inputSchema);
    
    const { command, args: cmdArgs = [], requireRoot = false, timeout = 300000 } = args;
    
    const result = await this.commandExecutor.execute(command, cmdArgs, {
      requireRoot,
      timeout
    });

    return this.createTextResult(
      result.success ? result.stdout : result.stderr,
      {
        exitCode: result.exitCode,
        success: result.success,
        duration: result.duration
      }
    );
  }

  async handleSystemInfo(args) {
    await this.validateArgs(args, this.tools[1].inputSchema);
    
    const { detailed = false } = args;
    
    const info = {
      basic: await si.osInfo(),
      cpu: await si.cpu(),
      memory: await si.mem(),
      uptime: si.time().uptime,
      load: await si.currentLoad()
    };

    if (detailed) {
      info.detailed = {
        system: await si.system(),
        motherboard: await si.baseboard(),
        bios: await si.bios(),
        disks: await si.diskLayout(),
        network: await si.networkInterfaces()
      };
    }

    return this.createTextResult(JSON.stringify(info, null, 2));
  }

  async handleSystemServices(args) {
    await this.validateArgs(args, this.tools[2].inputSchema);
    
    const { action, service } = args;
    
    let result;
    switch (action) {
      case 'list':
        result = await this.commandExecutor.execute('systemctl', ['list-units', '--type=service']);
        break;
      case 'status':
        if (!service) throw new Error('Service name required for status action');
        result = await this.commandExecutor.execute('systemctl', ['status', service]);
        break;
      case 'start':
      case 'stop':
      case 'restart':
        if (!service) throw new Error(`Service name required for ${action} action`);
        result = await this.commandExecutor.executeWithSudo('systemctl', [action, service]);
        break;
      case 'enable':
      case 'disable':
        if (!service) throw new Error(`Service name required for ${action} action`);
        result = await this.commandExecutor.executeWithSudo('systemctl', [action, service]);
        break;
      default:
        throw new Error(`Unknown action: ${action}`);
    }

    return this.createTextResult(result.stdout || result.stderr);
  }

  async handleSystemPackage(args) {
    await this.validateArgs(args, this.tools[3].inputSchema);
    
    const { action, packages = [], noconfirm = false } = args;
    
    const pacmanArgs = [];
    if (noconfirm) pacmanArgs.push('--noconfirm');

    let result;
    switch (action) {
      case 'update':
        result = await this.commandExecutor.executeWithSudo('pacman', ['-Sy', ...pacmanArgs]);
        break;
      case 'upgrade':
        result = await this.commandExecutor.executeWithSudo('pacman', ['-Syu', ...pacmanArgs]);
        break;
      case 'install':
        if (packages.length === 0) throw new Error('Package names required for install');
        result = await this.commandExecutor.executeWithSudo('pacman', ['-S', ...pacmanArgs, ...packages]);
        break;
      case 'remove':
        if (packages.length === 0) throw new Error('Package names required for remove');
        result = await this.commandExecutor.executeWithSudo('pacman', ['-R', ...pacmanArgs, ...packages]);
        break;
      case 'search':
        if (packages.length === 0) throw new Error('Search term required');
        result = await this.commandExecutor.execute('pacman', ['-Ss', packages[0]]);
        break;
      case 'info':
        if (packages.length === 0) throw new Error('Package name required for info');
        result = await this.commandExecutor.execute('pacman', ['-Si', packages[0]]);
        break;
      default:
        throw new Error(`Unknown action: ${action}`);
    }

    return this.createTextResult(result.stdout || result.stderr);
  }

  async handleSystemSnapshot(args) {
    await this.validateArgs(args, this.tools[4].inputSchema);
    
    const { description, files } = args;
    
    const snapshotId = await this.security.createSnapshot(description, files);
    
    return this.createTextResult(`Snapshot created successfully: ${snapshotId}`, {
      snapshotId
    });
  }

  async handleSystemRollback(args) {
    await this.validateArgs(args, this.tools[5].inputSchema);
    
    const { snapshotId } = args;
    
    await this.security.restoreSnapshot(snapshotId);
    
    return this.createTextResult(`Successfully rolled back to snapshot: ${snapshotId}`);
  }

  async handleSystemProcess(args) {
    await this.validateArgs(args, this.tools[6].inputSchema);
    
    const { action, pid, signal = 'TERM', filter } = args;
    
    let result;
    switch (action) {
      case 'list':
        const psArgs = ['aux'];
        if (filter) {
          result = await this.commandExecutor.execute('ps', psArgs);
          const lines = result.stdout.split('\n').filter(line => 
            line.toLowerCase().includes(filter.toLowerCase())
          );
          result.stdout = lines.join('\n');
        } else {
          result = await this.commandExecutor.execute('ps', psArgs);
        }
        break;
      case 'kill':
        if (!pid) throw new Error('PID required for kill action');
        result = await this.commandExecutor.execute('kill', [`-${signal}`, pid.toString()]);
        break;
      case 'info':
        if (!pid) throw new Error('PID required for info action');
        result = await this.commandExecutor.execute('ps', ['-p', pid.toString(), '-o', 'pid,ppid,user,comm,args']);
        break;
      default:
        throw new Error(`Unknown action: ${action}`);
    }

    return this.createTextResult(result.stdout || result.stderr);
  }

  async getSystemInfo() {
    const info = await si.osInfo();
    return { content: JSON.stringify(info, null, 2) };
  }

  async getSystemLogs() {
    const result = await this.commandExecutor.execute('journalctl', ['-n', '100', '--no-pager']);
    return { content: result.stdout };
  }

  async getServiceStatus() {
    const result = await this.commandExecutor.execute('systemctl', ['list-units', '--type=service', '--no-pager']);
    return { content: result.stdout };
  }

  async getSnapshots() {
    const snapshots = await this.security.listSnapshots();
    return { content: JSON.stringify(snapshots, null, 2) };
  }

  async getProcesses() {
    const result = await this.commandExecutor.execute('ps', ['aux']);
    return { content: result.stdout };
  }
}