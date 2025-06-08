import fs from 'fs-extra';
import path from 'path';
import YAML from 'yaml';
import Joi from 'joi';

const configSchema = Joi.object({
  server: Joi.object({
    host: Joi.string().default('localhost'),
    port: Joi.number().integer().min(1).max(65535).default(8080),
    cors: Joi.object({
      origin: Joi.alternatives().try(Joi.boolean(), Joi.string(), Joi.array().items(Joi.string())).default(true),
      credentials: Joi.boolean().default(true)
    }).default()
  }).default(),

  logging: Joi.object({
    level: Joi.string().valid('error', 'warn', 'info', 'debug').default('info'),
    logDir: Joi.string().default('/var/log/mcp-arch-linux'),
    maxFiles: Joi.string().default('14d'),
    maxSize: Joi.string().default('20m')
  }).default(),

  security: Joi.object({
    requireAuth: Joi.boolean().default(true),
    allowedCommands: Joi.array().items(Joi.string()).default([
      'pacman', 'systemctl', 'hyprctl', 'grim', 'wf-recorder', 
      'sgdisk', 'mkfs.ext4', 'mkfs.fat', 'mkswap', 'mount', 'umount',
      'arch-chroot', 'pacstrap', 'genfstab'
    ]),
    maxConcurrentOperations: Joi.number().integer().min(1).default(10),
    commandTimeout: Joi.number().integer().min(1000).default(300000), // 5 minutes
    auditAll: Joi.boolean().default(true)
  }).default(),

  plugins: Joi.object({
    system: Joi.object({
      enabled: Joi.boolean().default(true),
      snapshotDir: Joi.string().default('/var/lib/mcp-arch-linux/snapshots')
    }).default(),
    
    archInstall: Joi.object({
      enabled: Joi.boolean().default(true),
      allowDiskOperations: Joi.boolean().default(true)
    }).default(),
    
    hyprland: Joi.object({
      enabled: Joi.boolean().default(true),
      socketPath: Joi.string().allow(null).default(null) // Auto-detect
    }).default(),
    
    screenCapture: Joi.object({
      enabled: Joi.boolean().default(true),
      captureDir: Joi.string().default('/var/lib/mcp-arch-linux/captures'),
      maxFileSize: Joi.string().default('50MB'),
      allowRecording: Joi.boolean().default(true)
    }).default()
  }).default()
});

export class Config {
  constructor(configPath = './config/server.yaml') {
    this.configPath = configPath;
    this.config = {};
  }

  async load() {
    try {
      // Load default config
      this.config = {
        server: {
          host: 'localhost',
          port: 8080,
          cors: {
            origin: true,
            credentials: true
          }
        },
        logging: {
          level: 'info',
          logDir: '/var/log/mcp-arch-linux',
          maxFiles: '14d',
          maxSize: '20m'
        },
        security: {
          requireAuth: true,
          allowedCommands: [
            'pacman', 'systemctl', 'hyprctl', 'grim', 'wf-recorder',
            'sgdisk', 'mkfs.ext4', 'mkfs.fat', 'mkswap', 'mount', 'umount',
            'arch-chroot', 'pacstrap', 'genfstab'
          ],
          maxConcurrentOperations: 10,
          commandTimeout: 300000,
          auditAll: true
        },
        plugins: {
          system: {
            enabled: true,
            snapshotDir: '/var/lib/mcp-arch-linux/snapshots'
          },
          archInstall: {
            enabled: true,
            allowDiskOperations: true
          },
          hyprland: {
            enabled: true,
            socketPath: null
          },
          screenCapture: {
            enabled: true,
            captureDir: '/var/lib/mcp-arch-linux/captures',
            maxFileSize: '50MB',
            allowRecording: true
          }
        }
      };

      // Try to load config file if it exists
      if (await fs.pathExists(this.configPath)) {
        const configContent = await fs.readFile(this.configPath, 'utf8');
        const fileConfig = YAML.parse(configContent);
        
        // Merge with defaults
        this.config = this.mergeDeep(this.config, fileConfig);
      } else {
        // Create default config file
        await this.save();
      }

      // Validate config
      const { error, value } = configSchema.validate(this.config);
      if (error) {
        throw new Error(`Config validation error: ${error.message}`);
      }

      this.config = value;

      // Ensure directories exist
      await this.ensureDirectories();

    } catch (error) {
      throw new Error(`Failed to load config: ${error.message}`);
    }
  }

  async save() {
    try {
      await fs.ensureDir(path.dirname(this.configPath));
      const yamlContent = YAML.stringify(this.config, {
        indent: 2,
        lineWidth: 120
      });
      await fs.writeFile(this.configPath, yamlContent, 'utf8');
    } catch (error) {
      throw new Error(`Failed to save config: ${error.message}`);
    }
  }

  get(keyPath) {
    return this.getNestedValue(this.config, keyPath);
  }

  set(keyPath, value) {
    this.setNestedValue(this.config, keyPath, value);
  }

  getAll() {
    return { ...this.config };
  }

  async ensureDirectories() {
    const dirs = [
      this.get('logging.logDir'),
      this.get('plugins.system.snapshotDir'),
      this.get('plugins.screenCapture.captureDir')
    ];

    for (const dir of dirs) {
      if (dir) {
        await fs.ensureDir(dir);
      }
    }
  }

  getNestedValue(obj, keyPath) {
    return keyPath.split('.').reduce((current, key) => {
      return current && current[key] !== undefined ? current[key] : undefined;
    }, obj);
  }

  setNestedValue(obj, keyPath, value) {
    const keys = keyPath.split('.');
    const lastKey = keys.pop();
    const target = keys.reduce((current, key) => {
      if (!current[key] || typeof current[key] !== 'object') {
        current[key] = {};
      }
      return current[key];
    }, obj);
    target[lastKey] = value;
  }

  mergeDeep(target, source) {
    const output = { ...target };
    if (this.isObject(target) && this.isObject(source)) {
      Object.keys(source).forEach(key => {
        if (this.isObject(source[key])) {
          if (!(key in target)) {
            output[key] = source[key];
          } else {
            output[key] = this.mergeDeep(target[key], source[key]);
          }
        } else {
          output[key] = source[key];
        }
      });
    }
    return output;
  }

  isObject(item) {
    return item && typeof item === 'object' && !Array.isArray(item);
  }
}