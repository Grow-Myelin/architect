import { BasePlugin } from './base-plugin.js';
import { CommandExecutor } from '../system/command-executor.js';
import fs from 'fs-extra';
import path from 'path';

export class ScreenCapturePlugin extends BasePlugin {
  constructor(config, logger, security) {
    super('screen-capture', config, logger, security);
    this.description = 'Screen capture and recording functionality';
    this.captureDir = config.plugins?.screenCapture?.captureDir || '/var/lib/mcp-arch-linux/captures';
    this.maxFileSize = config.plugins?.screenCapture?.maxFileSize || '50MB';
    this.allowRecording = config.plugins?.screenCapture?.allowRecording || true;
    
    this.commandExecutor = new CommandExecutor(
      config.security || {},
      logger,
      security
    );
    
    this.availableTools = {
      grim: false,
      wfRecorder: false,
      slurp: false
    };
    
    this.initializeTools();
    this.initializeResources();
  }

  async initialize() {
    await super.initialize();
    
    // Ensure capture directory exists
    await fs.ensureDir(this.captureDir);
    
    // Check available tools
    this.availableTools.grim = await this.commandExecutor.checkCommandExists('grim');
    this.availableTools.wfRecorder = await this.commandExecutor.checkCommandExists('wf-recorder');
    this.availableTools.slurp = await this.commandExecutor.checkCommandExists('slurp');
    
    this.logger.info('Screen capture plugin initialized', {
      captureDir: this.captureDir,
      availableTools: this.availableTools
    });
  }

  initializeTools() {
    this.tools = [
      this.createTool(
        'capture_screenshot',
        'Capture a screenshot of the screen or specific area',
        {
          type: 'object',
          properties: {
            output: {
              type: 'string',
              description: 'Output name (monitor) to capture, or "all" for all outputs'
            },
            region: {
              type: 'object',
              properties: {
                x: { type: 'integer' },
                y: { type: 'integer' },
                width: { type: 'integer' },
                height: { type: 'integer' }
              },
              description: 'Specific region to capture (x,y,width,height)'
            },
            format: {
              type: 'string',
              enum: ['png', 'jpg', 'webp'],
              description: 'Image format',
              default: 'png'
            },
            quality: {
              type: 'integer',
              minimum: 1,
              maximum: 100,
              description: 'Image quality for lossy formats',
              default: 90
            },
            filename: {
              type: 'string',
              description: 'Custom filename (without extension)'
            }
          }
        }
      ),

      this.createTool(
        'capture_window',
        'Capture a screenshot of a specific window',
        {
          type: 'object',
          properties: {
            selector: {
              type: 'string',
              description: 'Window selector (class, title, or "active" for current window)',
              default: 'active'
            },
            format: {
              type: 'string',
              enum: ['png', 'jpg', 'webp'],
              description: 'Image format',
              default: 'png'
            },
            filename: {
              type: 'string',
              description: 'Custom filename (without extension)'
            }
          }
        }
      ),

      this.createTool(
        'capture_selection',
        'Capture a user-selected area of the screen',
        {
          type: 'object',
          properties: {
            format: {
              type: 'string',
              enum: ['png', 'jpg', 'webp'],
              description: 'Image format',
              default: 'png'
            },
            filename: {
              type: 'string',
              description: 'Custom filename (without extension)'
            }
          }
        }
      ),

      this.createTool(
        'start_recording',
        'Start screen recording',
        {
          type: 'object',
          properties: {
            output: {
              type: 'string',
              description: 'Output name to record'
            },
            audio: {
              type: 'boolean',
              description: 'Include audio in recording',
              default: false
            },
            format: {
              type: 'string',
              enum: ['mp4', 'webm', 'mkv'],
              description: 'Video format',
              default: 'mp4'
            },
            fps: {
              type: 'integer',
              minimum: 1,
              maximum: 60,
              description: 'Frames per second',
              default: 30
            },
            filename: {
              type: 'string',
              description: 'Custom filename (without extension)'
            }
          }
        }
      ),

      this.createTool(
        'stop_recording',
        'Stop current screen recording',
        {
          type: 'object',
          properties: {}
        }
      ),

      this.createTool(
        'list_captures',
        'List all captured files',
        {
          type: 'object',
          properties: {
            type: {
              type: 'string',
              enum: ['all', 'images', 'videos'],
              description: 'Type of captures to list',
              default: 'all'
            },
            limit: {
              type: 'integer',
              minimum: 1,
              description: 'Maximum number of files to return',
              default: 50
            }
          }
        }
      ),

      this.createTool(
        'delete_capture',
        'Delete a captured file',
        {
          type: 'object',
          properties: {
            filename: {
              type: 'string',
              description: 'Filename to delete'
            }
          },
          required: ['filename']
        }
      ),

      this.createTool(
        'get_capture',
        'Get a captured file as base64 data',
        {
          type: 'object',
          properties: {
            filename: {
              type: 'string',
              description: 'Filename to retrieve'
            }
          },
          required: ['filename']
        }
      )
    ];
  }

  initializeResources() {
    this.resources = [
      this.createResource(
        'capture://list',
        'Capture List',
        'List of all captured files',
        'application/json'
      ),
      this.createResource(
        'capture://latest',
        'Latest Capture',
        'The most recent capture',
        'application/json'
      ),
      this.createResource(
        'capture://status',
        'Capture Status',
        'Current capture system status',
        'application/json'
      )
    ];
  }

  async executeTool(toolName, args) {
    return this.withErrorHandling(async () => {
      switch (toolName) {
        case 'capture_screenshot':
          return this.handleScreenshot(args);
        case 'capture_window':
          return this.handleWindowCapture(args);
        case 'capture_selection':
          return this.handleSelectionCapture(args);
        case 'start_recording':
          return this.handleStartRecording(args);
        case 'stop_recording':
          return this.handleStopRecording(args);
        case 'list_captures':
          return this.handleListCaptures(args);
        case 'delete_capture':
          return this.handleDeleteCapture(args);
        case 'get_capture':
          return this.handleGetCapture(args);
        default:
          throw new Error(`Unknown tool: ${toolName}`);
      }
    }, toolName);
  }

  async readResource(uri) {
    return this.withErrorHandling(async () => {
      switch (uri) {
        case 'capture://list':
          return this.getCaptureList();
        case 'capture://latest':
          return this.getLatestCapture();
        case 'capture://status':
          return this.getCaptureStatus();
        default:
          throw new Error(`Unknown resource: ${uri}`);
      }
    }, 'readResource');
  }

  generateFilename(prefix, format) {
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    return `${prefix}_${timestamp}.${format}`;
  }

  async handleScreenshot(args) {
    if (!this.availableTools.grim) {
      throw new Error('grim is not available - install with: sudo pacman -S grim');
    }

    await this.validateArgs(args, this.tools[0].inputSchema);
    
    const { 
      output, 
      region, 
      format = 'png', 
      quality = 90, 
      filename 
    } = args;
    
    const finalFilename = filename ? 
      `${filename}.${format}` : 
      this.generateFilename('screenshot', format);
    const filepath = path.join(this.captureDir, finalFilename);
    
    const grimArgs = [];
    
    // Add output selection
    if (output && output !== 'all') {
      grimArgs.push('-o', output);
    }
    
    // Add region selection
    if (region) {
      const { x, y, width, height } = region;
      grimArgs.push('-g', `${x},${y} ${width}x${height}`);
    }
    
    // Add quality for JPEG
    if (format === 'jpg' && quality !== 90) {
      grimArgs.push('-q', quality.toString());
    }
    
    grimArgs.push(filepath);
    
    const result = await this.commandExecutor.execute('grim', grimArgs);
    
    if (!result.success) {
      throw new Error(`Screenshot failed: ${result.stderr}`);
    }
    
    // Read and return image as base64
    const imageData = await fs.readFile(filepath);
    const base64Data = imageData.toString('base64');
    
    return this.createImageResult(base64Data, `image/${format}`, {
      filename: finalFilename,
      size: imageData.length,
      format
    });
  }

  async handleWindowCapture(args) {
    if (!this.availableTools.grim) {
      throw new Error('grim is not available');
    }

    await this.validateArgs(args, this.tools[1].inputSchema);
    
    const { selector = 'active', format = 'png', filename } = args;
    
    const finalFilename = filename ? 
      `${filename}.${format}` : 
      this.generateFilename('window', format);
    const filepath = path.join(this.captureDir, finalFilename);
    
    let grimArgs;
    
    if (selector === 'active') {
      // Capture active window using hyprctl
      try {
        const windowInfo = await this.commandExecutor.execute('hyprctl', ['activewindow', '-j']);
        const window = JSON.parse(windowInfo.stdout);
        const { at, size } = window;
        grimArgs = ['-g', `${at[0]},${at[1]} ${size[0]}x${size[1]}`, filepath];
      } catch {
        throw new Error('Failed to get active window information');
      }
    } else {
      // Use window class or title selector
      grimArgs = ['-w', selector, filepath];
    }
    
    const result = await this.commandExecutor.execute('grim', grimArgs);
    
    if (!result.success) {
      throw new Error(`Window capture failed: ${result.stderr}`);
    }
    
    const imageData = await fs.readFile(filepath);
    const base64Data = imageData.toString('base64');
    
    return this.createImageResult(base64Data, `image/${format}`, {
      filename: finalFilename,
      size: imageData.length,
      selector
    });
  }

  async handleSelectionCapture(args) {
    if (!this.availableTools.grim || !this.availableTools.slurp) {
      throw new Error('grim and slurp are required - install with: sudo pacman -S grim slurp');
    }

    await this.validateArgs(args, this.tools[2].inputSchema);
    
    const { format = 'png', filename } = args;
    
    const finalFilename = filename ? 
      `${filename}.${format}` : 
      this.generateFilename('selection', format);
    const filepath = path.join(this.captureDir, finalFilename);
    
    // Use slurp to get selection, then grim to capture
    const slurpResult = await this.commandExecutor.execute('slurp');
    
    if (!slurpResult.success) {
      throw new Error('Selection cancelled or failed');
    }
    
    const selection = slurpResult.stdout.trim();
    const result = await this.commandExecutor.execute('grim', ['-g', selection, filepath]);
    
    if (!result.success) {
      throw new Error(`Selection capture failed: ${result.stderr}`);
    }
    
    const imageData = await fs.readFile(filepath);
    const base64Data = imageData.toString('base64');
    
    return this.createImageResult(base64Data, `image/${format}`, {
      filename: finalFilename,
      size: imageData.length,
      selection
    });
  }

  async handleStartRecording(args) {
    if (!this.allowRecording) {
      throw new Error('Recording is disabled in configuration');
    }
    
    if (!this.availableTools.wfRecorder) {
      throw new Error('wf-recorder is not available - install with: sudo pacman -S wf-recorder');
    }

    await this.validateArgs(args, this.tools[3].inputSchema);
    
    const { 
      output, 
      audio = false, 
      format = 'mp4', 
      fps = 30, 
      filename 
    } = args;
    
    const finalFilename = filename ? 
      `${filename}.${format}` : 
      this.generateFilename('recording', format);
    const filepath = path.join(this.captureDir, finalFilename);
    
    const recordingArgs = [];
    
    if (output) {
      recordingArgs.push('-o', output);
    }
    
    if (audio) {
      recordingArgs.push('-a');
    }
    
    recordingArgs.push('-r', fps.toString());
    recordingArgs.push('-f', filepath);
    
    // Start recording in background
    const child = await this.commandExecutor.execute('wf-recorder', recordingArgs, {
      captureOutput: false
    });
    
    // Save recording info
    const recordingInfo = {
      filename: finalFilename,
      filepath,
      startTime: new Date().toISOString(),
      pid: child.pid
    };
    
    const recordingInfoPath = path.join(this.captureDir, '.recording.json');
    await fs.writeJson(recordingInfoPath, recordingInfo);
    
    return this.createTextResult(`Recording started: ${finalFilename}`, {
      filename: finalFilename,
      format,
      fps,
      audio
    });
  }

  async handleStopRecording(args) {
    const recordingInfoPath = path.join(this.captureDir, '.recording.json');
    
    if (!await fs.pathExists(recordingInfoPath)) {
      throw new Error('No active recording found');
    }
    
    const recordingInfo = await fs.readJson(recordingInfoPath);
    
    // Stop recording by sending SIGINT
    try {
      await this.commandExecutor.execute('kill', ['-INT', recordingInfo.pid.toString()]);
    } catch {
      // Process might have already stopped
    }
    
    // Clean up recording info
    await fs.remove(recordingInfoPath);
    
    // Check if file exists and get info
    const stats = await fs.stat(recordingInfo.filepath);
    
    return this.createTextResult(`Recording stopped: ${recordingInfo.filename}`, {
      filename: recordingInfo.filename,
      duration: new Date() - new Date(recordingInfo.startTime),
      size: stats.size
    });
  }

  async handleListCaptures(args) {
    await this.validateArgs(args, this.tools[5].inputSchema);
    
    const { type = 'all', limit = 50 } = args;
    
    const files = await fs.readdir(this.captureDir);
    const captures = [];
    
    for (const file of files) {
      if (file.startsWith('.')) continue; // Skip hidden files
      
      const filepath = path.join(this.captureDir, file);
      const stats = await fs.stat(filepath);
      const ext = path.extname(file).toLowerCase();
      
      let fileType;
      if (['.png', '.jpg', '.jpeg', '.webp'].includes(ext)) {
        fileType = 'image';
      } else if (['.mp4', '.webm', '.mkv'].includes(ext)) {
        fileType = 'video';
      } else {
        continue; // Skip unknown file types
      }
      
      if (type !== 'all' && type !== `${fileType}s`) {
        continue;
      }
      
      captures.push({
        filename: file,
        type: fileType,
        size: stats.size,
        created: stats.birthtime,
        modified: stats.mtime
      });
    }
    
    // Sort by creation date, newest first
    captures.sort((a, b) => new Date(b.created) - new Date(a.created));
    
    return this.createTextResult(JSON.stringify(captures.slice(0, limit), null, 2));
  }

  async handleDeleteCapture(args) {
    await this.validateArgs(args, this.tools[6].inputSchema);
    
    const { filename } = args;
    const filepath = path.join(this.captureDir, filename);
    
    // Security check - ensure file is in capture directory
    const resolvedPath = path.resolve(filepath);
    const resolvedCaptureDir = path.resolve(this.captureDir);
    
    if (!resolvedPath.startsWith(resolvedCaptureDir)) {
      throw new Error('Invalid file path');
    }
    
    if (!await fs.pathExists(filepath)) {
      throw new Error(`File not found: ${filename}`);
    }
    
    await fs.remove(filepath);
    
    return this.createTextResult(`File deleted: ${filename}`);
  }

  async handleGetCapture(args) {
    await this.validateArgs(args, this.tools[7].inputSchema);
    
    const { filename } = args;
    const filepath = path.join(this.captureDir, filename);
    
    // Security check
    const resolvedPath = path.resolve(filepath);
    const resolvedCaptureDir = path.resolve(this.captureDir);
    
    if (!resolvedPath.startsWith(resolvedCaptureDir)) {
      throw new Error('Invalid file path');
    }
    
    if (!await fs.pathExists(filepath)) {
      throw new Error(`File not found: ${filename}`);
    }
    
    const data = await fs.readFile(filepath);
    const base64Data = data.toString('base64');
    const stats = await fs.stat(filepath);
    const ext = path.extname(filename).toLowerCase();
    
    let mimeType = 'application/octet-stream';
    if (['.png'].includes(ext)) mimeType = 'image/png';
    else if (['.jpg', '.jpeg'].includes(ext)) mimeType = 'image/jpeg';
    else if (['.webp'].includes(ext)) mimeType = 'image/webp';
    else if (['.mp4'].includes(ext)) mimeType = 'video/mp4';
    else if (['.webm'].includes(ext)) mimeType = 'video/webm';
    
    return this.createImageResult(base64Data, mimeType, {
      filename,
      size: stats.size,
      created: stats.birthtime
    });
  }

  async getCaptureList() {
    const result = await this.handleListCaptures({ type: 'all', limit: 100 });
    return { content: result.content[0].text };
  }

  async getLatestCapture() {
    try {
      const files = await fs.readdir(this.captureDir);
      let latestFile = null;
      let latestTime = 0;
      
      for (const file of files) {
        if (file.startsWith('.')) continue;
        
        const filepath = path.join(this.captureDir, file);
        const stats = await fs.stat(filepath);
        
        if (stats.birthtime.getTime() > latestTime) {
          latestTime = stats.birthtime.getTime();
          latestFile = {
            filename: file,
            size: stats.size,
            created: stats.birthtime
          };
        }
      }
      
      return { content: JSON.stringify(latestFile, null, 2) };
    } catch (error) {
      return { content: JSON.stringify({ error: error.message }, null, 2) };
    }
  }

  async getCaptureStatus() {
    const status = {
      captureDir: this.captureDir,
      availableTools: this.availableTools,
      allowRecording: this.allowRecording,
      maxFileSize: this.maxFileSize
    };
    
    // Check for active recording
    const recordingInfoPath = path.join(this.captureDir, '.recording.json');
    if (await fs.pathExists(recordingInfoPath)) {
      const recordingInfo = await fs.readJson(recordingInfoPath);
      status.activeRecording = recordingInfo;
    }
    
    return { content: JSON.stringify(status, null, 2) };
  }
}