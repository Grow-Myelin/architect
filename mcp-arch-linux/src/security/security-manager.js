import crypto from 'crypto';
import fs from 'fs-extra';
import path from 'path';
import { v4 as uuidv4 } from 'uuid';

export class SecurityManager {
  constructor(config, logger) {
    this.config = config;
    this.logger = logger;
    this.operationCount = 0;
    this.maxConcurrentOperations = config.maxConcurrentOperations || 10;
    this.auditAll = config.auditAll || true;
    this.activeOperations = new Map();
  }

  async initialize() {
    this.logger.info('Security manager initialized', {
      requireAuth: this.config.requireAuth,
      maxConcurrentOperations: this.maxConcurrentOperations,
      auditAll: this.auditAll
    });
  }

  async executeWithAudit(operationType, context, operation) {
    // Check concurrent operation limit
    if (this.activeOperations.size >= this.maxConcurrentOperations) {
      throw new Error('Maximum concurrent operations exceeded');
    }

    const operationId = uuidv4();
    const startTime = Date.now();
    
    this.activeOperations.set(operationId, {
      type: operationType,
      context,
      startTime
    });

    try {
      // Log operation start
      if (this.auditAll) {
        this.logger.audit('operation_start', {
          operationId,
          type: operationType,
          context,
          timestamp: new Date().toISOString()
        });
      }

      // Execute operation
      const result = await operation();
      
      // Log successful completion
      if (this.auditAll) {
        this.logger.audit('operation_success', {
          operationId,
          type: operationType,
          context,
          duration: Date.now() - startTime,
          timestamp: new Date().toISOString()
        });
      }

      return result;

    } catch (error) {
      // Log failed operation
      if (this.auditAll) {
        this.logger.audit('operation_failure', {
          operationId,
          type: operationType,
          context,
          error: error.message,
          duration: Date.now() - startTime,
          timestamp: new Date().toISOString()
        });
      }
      
      throw error;
      
    } finally {
      this.activeOperations.delete(operationId);
    }
  }

  async createSnapshot(description, files = []) {
    const snapshotId = uuidv4();
    const timestamp = new Date().toISOString();
    
    this.logger.info(`Creating system snapshot: ${snapshotId}`);

    const snapshot = {
      id: snapshotId,
      description,
      timestamp,
      files: [],
      services: [],
      metadata: {
        hostname: await this.getHostname(),
        user: process.env.USER || 'unknown',
        node: process.version
      }
    };

    // Backup specified files
    for (const filePath of files) {
      try {
        if (await fs.pathExists(filePath)) {
          const stats = await fs.stat(filePath);
          const content = await fs.readFile(filePath, 'utf8');
          
          snapshot.files.push({
            path: filePath,
            content,
            mode: stats.mode,
            size: stats.size,
            mtime: stats.mtime
          });
        }
      } catch (error) {
        this.logger.warn(`Failed to backup file ${filePath}:`, error.message);
      }
    }

    // Get systemd service states
    try {
      const services = await this.getServiceStates();
      snapshot.services = services;
    } catch (error) {
      this.logger.warn('Failed to capture service states:', error.message);
    }

    // Save snapshot
    const snapshotPath = path.join(
      this.config.snapshotDir || '/var/lib/mcp-arch-linux/snapshots',
      `${snapshotId}.json`
    );
    
    await fs.ensureDir(path.dirname(snapshotPath));
    await fs.writeJson(snapshotPath, snapshot, { spaces: 2 });

    this.logger.audit('snapshot_created', {
      snapshotId,
      description,
      fileCount: snapshot.files.length,
      serviceCount: snapshot.services.length
    });

    return snapshotId;
  }

  async restoreSnapshot(snapshotId) {
    this.logger.info(`Restoring system snapshot: ${snapshotId}`);

    const snapshotPath = path.join(
      this.config.snapshotDir || '/var/lib/mcp-arch-linux/snapshots',
      `${snapshotId}.json`
    );

    if (!await fs.pathExists(snapshotPath)) {
      throw new Error(`Snapshot not found: ${snapshotId}`);
    }

    const snapshot = await fs.readJson(snapshotPath);

    // Restore files
    for (const file of snapshot.files) {
      try {
        await fs.ensureDir(path.dirname(file.path));
        await fs.writeFile(file.path, file.content);
        await fs.chmod(file.path, file.mode);
        this.logger.debug(`Restored file: ${file.path}`);
      } catch (error) {
        this.logger.error(`Failed to restore file ${file.path}:`, error.message);
      }
    }

    // Restore services (basic implementation)
    for (const service of snapshot.services) {
      try {
        if (service.enabled !== service.currentEnabled) {
          const action = service.enabled ? 'enable' : 'disable';
          // This would need CommandExecutor integration
          this.logger.debug(`Would ${action} service: ${service.name}`);
        }
        
        if (service.active !== service.currentActive) {
          const action = service.active ? 'start' : 'stop';
          this.logger.debug(`Would ${action} service: ${service.name}`);
        }
      } catch (error) {
        this.logger.error(`Failed to restore service ${service.name}:`, error.message);
      }
    }

    this.logger.audit('snapshot_restored', {
      snapshotId,
      fileCount: snapshot.files.length,
      serviceCount: snapshot.services.length
    });

    return true;
  }

  async listSnapshots() {
    const snapshotDir = this.config.snapshotDir || '/var/lib/mcp-arch-linux/snapshots';
    
    if (!await fs.pathExists(snapshotDir)) {
      return [];
    }

    const files = await fs.readdir(snapshotDir);
    const snapshots = [];

    for (const file of files) {
      if (path.extname(file) === '.json') {
        try {
          const snapshotPath = path.join(snapshotDir, file);
          const snapshot = await fs.readJson(snapshotPath);
          snapshots.push({
            id: snapshot.id,
            description: snapshot.description,
            timestamp: snapshot.timestamp,
            fileCount: snapshot.files.length,
            serviceCount: snapshot.services.length
          });
        } catch (error) {
          this.logger.warn(`Failed to read snapshot ${file}:`, error.message);
        }
      }
    }

    return snapshots.sort((a, b) => new Date(b.timestamp) - new Date(a.timestamp));
  }

  async deleteSnapshot(snapshotId) {
    const snapshotPath = path.join(
      this.config.snapshotDir || '/var/lib/mcp-arch-linux/snapshots',
      `${snapshotId}.json`
    );

    if (!await fs.pathExists(snapshotPath)) {
      throw new Error(`Snapshot not found: ${snapshotId}`);
    }

    await fs.remove(snapshotPath);
    
    this.logger.audit('snapshot_deleted', { snapshotId });
    
    return true;
  }

  async validateInput(data, rules) {
    // Basic input validation
    if (typeof data !== 'object' || data === null) {
      throw new Error('Invalid input data');
    }

    for (const [key, rule] of Object.entries(rules)) {
      const value = data[key];

      if (rule.required && (value === undefined || value === null)) {
        throw new Error(`Required field missing: ${key}`);
      }

      if (value !== undefined && rule.type && typeof value !== rule.type) {
        throw new Error(`Invalid type for field ${key}: expected ${rule.type}, got ${typeof value}`);
      }

      if (rule.pattern && typeof value === 'string' && !rule.pattern.test(value)) {
        throw new Error(`Invalid format for field ${key}`);
      }

      if (rule.maxLength && typeof value === 'string' && value.length > rule.maxLength) {
        throw new Error(`Field ${key} exceeds maximum length of ${rule.maxLength}`);
      }
    }

    return true;
  }

  async sanitizePath(inputPath) {
    // Resolve and normalize path
    const normalized = path.resolve(inputPath);
    
    // Check for path traversal
    if (normalized.includes('..')) {
      throw new Error('Path traversal detected');
    }

    return normalized;
  }

  generateHash(data) {
    return crypto.createHash('sha256').update(JSON.stringify(data)).digest('hex');
  }

  async getHostname() {
    try {
      const { CommandExecutor } = await import('./command-executor.js');
      const executor = new CommandExecutor(this.config, this.logger, this);
      const result = await executor.execute('hostname');
      return result.stdout;
    } catch {
      return 'unknown';
    }
  }

  async getServiceStates() {
    // This would integrate with CommandExecutor to get actual service states
    // For now, return empty array
    return [];
  }

  getActiveOperations() {
    return Array.from(this.activeOperations.values());
  }

  async cleanup() {
    // Cancel any active operations if needed
    this.activeOperations.clear();
    this.logger.info('Security manager cleaned up');
  }
}