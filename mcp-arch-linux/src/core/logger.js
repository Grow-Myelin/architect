import winston from 'winston';
import DailyRotateFile from 'winston-daily-rotate-file';
import path from 'path';
import fs from 'fs-extra';

export class Logger {
  constructor(config = {}) {
    this.config = {
      level: 'info',
      logDir: '/var/log/mcp-arch-linux',
      maxFiles: '14d',
      maxSize: '20m',
      ...config
    };

    this.logger = this.createLogger();
  }

  createLogger() {
    // Ensure log directory exists
    fs.ensureDirSync(this.config.logDir);

    const transports = [
      // Console transport
      new winston.transports.Console({
        format: winston.format.combine(
          winston.format.colorize(),
          winston.format.timestamp(),
          winston.format.printf(({ timestamp, level, message, ...meta }) => {
            const metaStr = Object.keys(meta).length ? ` ${JSON.stringify(meta)}` : '';
            return `${timestamp} [${level}]: ${message}${metaStr}`;
          })
        )
      }),

      // Rotating file transport for general logs
      new DailyRotateFile({
        filename: path.join(this.config.logDir, 'app-%DATE%.log'),
        datePattern: 'YYYY-MM-DD',
        maxFiles: this.config.maxFiles,
        maxSize: this.config.maxSize,
        format: winston.format.combine(
          winston.format.timestamp(),
          winston.format.json()
        )
      }),

      // Separate audit log
      new DailyRotateFile({
        filename: path.join(this.config.logDir, 'audit-%DATE%.log'),
        datePattern: 'YYYY-MM-DD',
        maxFiles: this.config.maxFiles,
        maxSize: this.config.maxSize,
        level: 'info',
        format: winston.format.combine(
          winston.format.timestamp(),
          winston.format.json()
        ),
        // Only log audit events
        filter: (info) => info.audit === true
      })
    ];

    return winston.createLogger({
      level: this.config.level,
      transports,
      exceptionHandlers: [
        new winston.transports.File({
          filename: path.join(this.config.logDir, 'exceptions.log')
        })
      ],
      rejectionHandlers: [
        new winston.transports.File({
          filename: path.join(this.config.logDir, 'rejections.log')
        })
      ]
    });
  }

  debug(message, meta = {}) {
    this.logger.debug(message, meta);
  }

  info(message, meta = {}) {
    this.logger.info(message, meta);
  }

  warn(message, meta = {}) {
    this.logger.warn(message, meta);
  }

  error(message, meta = {}) {
    this.logger.error(message, meta);
  }

  // Special method for audit logging
  audit(operation, details = {}) {
    this.logger.info(`AUDIT: ${operation}`, {
      ...details,
      audit: true,
      timestamp: new Date().toISOString(),
      pid: process.pid
    });
  }

  // Performance logging
  time(label) {
    console.time(label);
  }

  timeEnd(label) {
    console.timeEnd(label);
  }

  // Create child logger with additional context
  child(defaultMeta) {
    const childLogger = this.logger.child(defaultMeta);
    return {
      debug: (message, meta = {}) => childLogger.debug(message, meta),
      info: (message, meta = {}) => childLogger.info(message, meta),
      warn: (message, meta = {}) => childLogger.warn(message, meta),
      error: (message, meta = {}) => childLogger.error(message, meta),
      audit: (operation, details = {}) => {
        childLogger.info(`AUDIT: ${operation}`, {
          ...details,
          audit: true,
          timestamp: new Date().toISOString(),
          pid: process.pid
        });
      }
    };
  }
}