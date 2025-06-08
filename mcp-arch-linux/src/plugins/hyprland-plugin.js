import { BasePlugin } from './base-plugin.js';
import { Socket } from 'net';
import fs from 'fs-extra';
import path from 'path';

export class HyprlandPlugin extends BasePlugin {
  constructor(config, logger, security) {
    super('hyprland', config, logger, security);
    this.description = 'Hyprland window manager integration';
    this.socketPath = config.plugins?.hyprland?.socketPath || null;
    this.isAvailable = false;
    
    this.initializeTools();
    this.initializeResources();
  }

  async initialize() {
    await super.initialize();
    
    // Auto-detect Hyprland socket if not configured
    if (!this.socketPath) {
      this.socketPath = await this.detectHyprlandSocket();
    }
    
    this.isAvailable = await this.checkHyprlandAvailable();
    
    if (!this.isAvailable) {
      this.logger.warn('Hyprland is not available - plugin will have limited functionality');
    } else {
      this.logger.info(`Hyprland plugin initialized with socket: ${this.socketPath}`);
    }
  }

  async detectHyprlandSocket() {
    const runtimeDir = process.env.XDG_RUNTIME_DIR;
    const instance = process.env.HYPRLAND_INSTANCE_SIGNATURE;
    
    if (runtimeDir && instance) {
      return path.join(runtimeDir, 'hypr', instance, '.socket.sock');
    }
    
    return null;
  }

  async checkHyprlandAvailable() {
    if (!this.socketPath) return false;
    
    try {
      await fs.access(this.socketPath);
      // Try a simple command to verify the socket works
      await this.sendHyprlandCommand('version');
      return true;
    } catch {
      return false;
    }
  }

  initializeTools() {
    this.tools = [
      this.createTool(
        'hyprland_dispatch',
        'Execute Hyprland dispatcher command',
        {
          type: 'object',
          properties: {
            dispatcher: {
              type: 'string',
              description: 'Dispatcher command (e.g., workspace, movewindow, killactive)'
            },
            args: {
              type: 'string',
              description: 'Dispatcher arguments',
              default: ''
            }
          },
          required: ['dispatcher']
        }
      ),

      this.createTool(
        'hyprland_keyword',
        'Set Hyprland configuration keyword',
        {
          type: 'object',
          properties: {
            keyword: {
              type: 'string',
              description: 'Configuration keyword'
            },
            value: {
              type: 'string',
              description: 'Value to set'
            }
          },
          required: ['keyword', 'value']
        }
      ),

      this.createTool(
        'hyprland_windows',
        'Get information about windows',
        {
          type: 'object',
          properties: {
            format: {
              type: 'string',
              enum: ['json', 'text'],
              description: 'Output format',
              default: 'json'
            }
          }
        }
      ),

      this.createTool(
        'hyprland_workspaces',
        'Get information about workspaces',
        {
          type: 'object',
          properties: {
            format: {
              type: 'string',
              enum: ['json', 'text'],
              description: 'Output format',
              default: 'json'
            }
          }
        }
      ),

      this.createTool(
        'hyprland_monitors',
        'Get information about monitors',
        {
          type: 'object',
          properties: {
            format: {
              type: 'string',
              enum: ['json', 'text'],
              description: 'Output format',
              default: 'json'
            }
          }
        }
      ),

      this.createTool(
        'hyprland_reload',
        'Reload Hyprland configuration',
        {
          type: 'object',
          properties: {}
        }
      ),

      this.createTool(
        'hyprland_layout',
        'Control window layouts',
        {
          type: 'object',
          properties: {
            action: {
              type: 'string',
              enum: ['toggle', 'set'],
              description: 'Layout action'
            },
            layout: {
              type: 'string',
              description: 'Layout name (for set action)'
            }
          },
          required: ['action']
        }
      ),

      this.createTool(
        'hyprland_window_control',
        'Control specific windows',
        {
          type: 'object',
          properties: {
            action: {
              type: 'string',
              enum: ['focus', 'move', 'resize', 'close', 'float', 'fullscreen'],
              description: 'Window action'
            },
            target: {
              type: 'string',
              description: 'Window identifier or direction'
            },
            args: {
              type: 'string',
              description: 'Additional arguments for the action',
              default: ''
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
        'hyprland://config',
        'Hyprland Configuration',
        'Current Hyprland configuration file',
        'text/plain'
      ),
      this.createResource(
        'hyprland://status',
        'Hyprland Status',
        'Current Hyprland status and information',
        'application/json'
      ),
      this.createResource(
        'hyprland://layout',
        'Window Layout',
        'Current window layout and arrangement',
        'application/json'
      )
    ];
  }

  async executeTool(toolName, args) {
    if (!this.isAvailable) {
      throw new Error('Hyprland is not available');
    }

    return this.withErrorHandling(async () => {
      switch (toolName) {
        case 'hyprland_dispatch':
          return this.handleDispatch(args);
        case 'hyprland_keyword':
          return this.handleKeyword(args);
        case 'hyprland_windows':
          return this.handleWindows(args);
        case 'hyprland_workspaces':
          return this.handleWorkspaces(args);
        case 'hyprland_monitors':
          return this.handleMonitors(args);
        case 'hyprland_reload':
          return this.handleReload(args);
        case 'hyprland_layout':
          return this.handleLayout(args);
        case 'hyprland_window_control':
          return this.handleWindowControl(args);
        default:
          throw new Error(`Unknown tool: ${toolName}`);
      }
    }, toolName);
  }

  async readResource(uri) {
    return this.withErrorHandling(async () => {
      switch (uri) {
        case 'hyprland://config':
          return this.getConfig();
        case 'hyprland://status':
          return this.getStatus();
        case 'hyprland://layout':
          return this.getLayout();
        default:
          throw new Error(`Unknown resource: ${uri}`);
      }
    }, 'readResource');
  }

  async sendHyprlandCommand(command) {
    return new Promise((resolve, reject) => {
      const socket = new Socket();
      let data = '';

      socket.connect(this.socketPath, () => {
        socket.write(command);
      });

      socket.on('data', (chunk) => {
        data += chunk.toString();
      });

      socket.on('end', () => {
        resolve(data.trim());
      });

      socket.on('error', (error) => {
        reject(new Error(`Hyprland socket error: ${error.message}`));
      });

      socket.setTimeout(5000, () => {
        socket.destroy();
        reject(new Error('Hyprland command timeout'));
      });
    });
  }

  async handleDispatch(args) {
    await this.validateArgs(args, this.tools[0].inputSchema);
    
    const { dispatcher, args: dispatcherArgs = '' } = args;
    const command = dispatcherArgs ? `dispatch ${dispatcher} ${dispatcherArgs}` : `dispatch ${dispatcher}`;
    
    const result = await this.sendHyprlandCommand(command);
    return this.createTextResult(result || 'Command executed successfully');
  }

  async handleKeyword(args) {
    await this.validateArgs(args, this.tools[1].inputSchema);
    
    const { keyword, value } = args;
    const command = `keyword ${keyword} ${value}`;
    
    const result = await this.sendHyprlandCommand(command);
    return this.createTextResult(result || `Set ${keyword} = ${value}`);
  }

  async handleWindows(args) {
    await this.validateArgs(args, this.tools[2].inputSchema);
    
    const { format = 'json' } = args;
    const command = format === 'json' ? 'j/clients' : 'clients';
    
    const result = await this.sendHyprlandCommand(command);
    
    if (format === 'json') {
      try {
        const windows = JSON.parse(result);
        return this.createTextResult(JSON.stringify(windows, null, 2));
      } catch {
        return this.createTextResult(result);
      }
    } else {
      return this.createTextResult(result);
    }
  }

  async handleWorkspaces(args) {
    await this.validateArgs(args, this.tools[3].inputSchema);
    
    const { format = 'json' } = args;
    const command = format === 'json' ? 'j/workspaces' : 'workspaces';
    
    const result = await this.sendHyprlandCommand(command);
    
    if (format === 'json') {
      try {
        const workspaces = JSON.parse(result);
        return this.createTextResult(JSON.stringify(workspaces, null, 2));
      } catch {
        return this.createTextResult(result);
      }
    } else {
      return this.createTextResult(result);
    }
  }

  async handleMonitors(args) {
    await this.validateArgs(args, this.tools[4].inputSchema);
    
    const { format = 'json' } = args;
    const command = format === 'json' ? 'j/monitors' : 'monitors';
    
    const result = await this.sendHyprlandCommand(command);
    
    if (format === 'json') {
      try {
        const monitors = JSON.parse(result);
        return this.createTextResult(JSON.stringify(monitors, null, 2));
      } catch {
        return this.createTextResult(result);
      }
    } else {
      return this.createTextResult(result);
    }
  }

  async handleReload(args) {
    const result = await this.sendHyprlandCommand('reload');
    return this.createTextResult('Hyprland configuration reloaded');
  }

  async handleLayout(args) {
    await this.validateArgs(args, this.tools[6].inputSchema);
    
    const { action, layout } = args;
    let command;
    
    switch (action) {
      case 'toggle':
        command = 'dispatch togglelayout';
        break;
      case 'set':
        if (!layout) throw new Error('Layout name required for set action');
        command = `dispatch exec hyprctl keyword general:layout ${layout}`;
        break;
      default:
        throw new Error(`Unknown layout action: ${action}`);
    }
    
    const result = await this.sendHyprlandCommand(command);
    return this.createTextResult(result || 'Layout command executed');
  }

  async handleWindowControl(args) {
    await this.validateArgs(args, this.tools[7].inputSchema);
    
    const { action, target = '', args: actionArgs = '' } = args;
    let command;
    
    switch (action) {
      case 'focus':
        command = target ? `dispatch focuswindow ${target}` : 'dispatch focuswindow';
        break;
      case 'move':
        command = target ? `dispatch movewindow ${target}` : 'dispatch movewindow';
        break;
      case 'resize':
        command = actionArgs ? `dispatch resizeactive ${actionArgs}` : 'dispatch resizeactive 10 10';
        break;
      case 'close':
        command = 'dispatch killactive';
        break;
      case 'float':
        command = 'dispatch togglefloating';
        break;
      case 'fullscreen':
        command = 'dispatch fullscreen';
        break;
      default:
        throw new Error(`Unknown window action: ${action}`);
    }
    
    const result = await this.sendHyprlandCommand(command);
    return this.createTextResult(result || `Window ${action} executed`);
  }

  async getConfig() {
    const configPath = process.env.HOME ? 
      path.join(process.env.HOME, '.config', 'hypr', 'hyprland.conf') :
      '/etc/hypr/hyprland.conf';
    
    try {
      const content = await fs.readFile(configPath, 'utf8');
      return { content };
    } catch (error) {
      return { content: `Error reading config: ${error.message}` };
    }
  }

  async getStatus() {
    if (!this.isAvailable) {
      return { 
        content: JSON.stringify({
          available: false,
          error: 'Hyprland not available'
        }, null, 2)
      };
    }

    try {
      const [version, activeWindow, workspaces] = await Promise.all([
        this.sendHyprlandCommand('version'),
        this.sendHyprlandCommand('j/activewindow'),
        this.sendHyprlandCommand('j/workspaces')
      ]);

      const status = {
        available: true,
        version,
        activeWindow: JSON.parse(activeWindow || '{}'),
        workspaceCount: JSON.parse(workspaces || '[]').length,
        socketPath: this.socketPath
      };

      return { content: JSON.stringify(status, null, 2) };
    } catch (error) {
      return {
        content: JSON.stringify({
          available: false,
          error: error.message
        }, null, 2)
      };
    }
  }

  async getLayout() {
    try {
      const [windows, workspaces, monitors] = await Promise.all([
        this.sendHyprlandCommand('j/clients'),
        this.sendHyprlandCommand('j/workspaces'),
        this.sendHyprlandCommand('j/monitors')
      ]);

      const layout = {
        windows: JSON.parse(windows || '[]'),
        workspaces: JSON.parse(workspaces || '[]'),
        monitors: JSON.parse(monitors || '[]')
      };

      return { content: JSON.stringify(layout, null, 2) };
    } catch (error) {
      return { content: `Error getting layout: ${error.message}` };
    }
  }
}