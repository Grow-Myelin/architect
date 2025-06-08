import { spawn } from 'child_process';
import { promisify } from 'util';
import fs from 'fs-extra';
import path from 'path';

export class CommandExecutor {
  constructor(config, logger, security) {
    this.config = config;
    this.logger = logger;
    this.security = security;
    this.allowedCommands = config.allowedCommands || [];
    this.timeout = config.commandTimeout || 300000; // 5 minutes
    this.runningProcesses = new Map();
  }

  async execute(command, args = [], options = {}) {
    const {
      cwd = process.cwd(),
      env = process.env,
      requireRoot = false,
      timeout = this.timeout,
      input = null,
      captureOutput = true
    } = options;

    // Security checks
    this.validateCommand(command);
    
    if (requireRoot && process.getuid && process.getuid() !== 0) {
      throw new Error('Root privileges required for this operation');
    }

    const processId = this.generateProcessId();
    
    try {
      this.logger.debug(`Executing command: ${command} ${args.join(' ')}`, {
        processId,
        cwd,
        requireRoot
      });

      const result = await this.spawnProcess(command, args, {
        cwd,
        env,
        timeout,
        input,
        captureOutput,
        processId
      });

      this.logger.debug(`Command completed: ${command}`, {
        processId,
        exitCode: result.exitCode,
        duration: result.duration
      });

      return result;

    } catch (error) {
      this.logger.error(`Command failed: ${command}`, {
        processId,
        error: error.message
      });
      throw error;
    } finally {
      this.runningProcesses.delete(processId);
    }
  }

  async executeScript(script, options = {}) {
    return this.execute('bash', ['-c', script], options);
  }

  async executeWithSudo(command, args = [], options = {}) {
    // Check if we're already root
    if (process.getuid && process.getuid() === 0) {
      return this.execute(command, args, options);
    }

    // Use sudo
    const sudoArgs = ['-n', command, ...args]; // -n for non-interactive
    return this.execute('sudo', sudoArgs, { ...options, requireRoot: false });
  }

  validateCommand(command) {
    // Check if command is in allowed list
    if (this.allowedCommands.length > 0 && !this.allowedCommands.includes(command)) {
      throw new Error(`Command not allowed: ${command}`);
    }

    // Prevent command injection
    if (command.includes(';') || command.includes('&&') || command.includes('||') || command.includes('|')) {
      throw new Error('Command injection detected');
    }

    // Check for path traversal
    if (command.includes('..') || command.includes('~')) {
      throw new Error('Path traversal detected');
    }
  }

  async spawnProcess(command, args, options) {
    return new Promise((resolve, reject) => {
      const startTime = Date.now();
      const child = spawn(command, args, {
        cwd: options.cwd,
        env: options.env,
        stdio: options.captureOutput ? ['pipe', 'pipe', 'pipe'] : 'inherit'
      });

      this.runningProcesses.set(options.processId, child);

      let stdout = '';
      let stderr = '';

      if (options.captureOutput) {
        child.stdout.on('data', (data) => {
          stdout += data.toString();
        });

        child.stderr.on('data', (data) => {
          stderr += data.toString();
        });
      }

      // Handle input
      if (options.input) {
        child.stdin.write(options.input);
        child.stdin.end();
      }

      // Set timeout
      const timeoutHandle = setTimeout(() => {
        child.kill('SIGTERM');
        setTimeout(() => {
          if (!child.killed) {
            child.kill('SIGKILL');
          }
        }, 5000); // Give 5 seconds for graceful shutdown
      }, options.timeout);

      child.on('close', (code, signal) => {
        clearTimeout(timeoutHandle);
        const duration = Date.now() - startTime;

        if (signal) {
          reject(new Error(`Process killed with signal ${signal}`));
        } else {
          resolve({
            exitCode: code,
            stdout: stdout.trim(),
            stderr: stderr.trim(),
            duration,
            success: code === 0
          });
        }
      });

      child.on('error', (error) => {
        clearTimeout(timeoutHandle);
        reject(new Error(`Failed to spawn process: ${error.message}`));
      });
    });
  }

  async killProcess(processId) {
    const process = this.runningProcesses.get(processId);
    if (process) {
      process.kill('SIGTERM');
      return true;
    }
    return false;
  }

  async killAllProcesses() {
    const promises = [];
    for (const [id, process] of this.runningProcesses.entries()) {
      promises.push(this.killProcess(id));
    }
    await Promise.all(promises);
  }

  getRunningProcesses() {
    return Array.from(this.runningProcesses.keys());
  }

  generateProcessId() {
    return `proc_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }

  // Convenience methods for common operations
  async checkCommandExists(command) {
    try {
      const result = await this.execute('which', [command]);
      return result.success;
    } catch {
      return false;
    }
  }

  async getSystemInfo() {
    try {
      const [hostname, uptime, memory, disk] = await Promise.all([
        this.execute('hostname').then(r => r.stdout).catch(() => 'unknown'),
        this.execute('uptime', ['-p']).then(r => r.stdout).catch(() => 'unknown'),
        this.execute('free', ['-h']).then(r => r.stdout).catch(() => 'unknown'),
        this.execute('df', ['-h', '/']).then(r => r.stdout).catch(() => 'unknown')
      ]);

      return {
        hostname,
        uptime,
        memory,
        disk
      };
    } catch (error) {
      throw new Error(`Failed to get system info: ${error.message}`);
    }
  }

  async isRoot() {
    return process.getuid ? process.getuid() === 0 : false;
  }

  async canSudo() {
    try {
      const result = await this.execute('sudo', ['-n', 'true']);
      return result.success;
    } catch {
      return false;
    }
  }
}