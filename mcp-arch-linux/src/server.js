#!/usr/bin/env node

import { Command } from 'commander';
import { createServer } from './core/mcp-server.js';
import { Logger } from './core/logger.js';
import { Config } from './core/config.js';
import { SecurityManager } from './security/security-manager.js';

const program = new Command();

program
  .name('mcp-arch-server')
  .description('MCP server for Arch Linux system control')
  .version('1.0.0')
  .option('-c, --config <path>', 'config file path', './config/server.yaml')
  .option('-p, --port <number>', 'server port', '8080')
  .option('-h, --host <address>', 'server host', 'localhost')
  .option('--debug', 'enable debug logging')
  .option('--no-auth', 'disable authentication (development only)')
  .parse();

const options = program.opts();

async function main() {
  try {
    // Initialize configuration
    const config = new Config(options.config);
    await config.load();
    
    // Override config with CLI options
    if (options.port) config.set('server.port', parseInt(options.port));
    if (options.host) config.set('server.host', options.host);
    if (options.debug) config.set('logging.level', 'debug');
    if (options.noAuth) config.set('security.requireAuth', false);
    
    // Initialize logger
    const logger = new Logger(config.get('logging'));
    
    // Initialize security manager
    const security = new SecurityManager(config.get('security'), logger);
    await security.initialize();
    
    // Create and start server
    const server = await createServer(config, logger, security);
    
    const host = config.get('server.host');
    const port = config.get('server.port');
    
    logger.info(`Starting MCP Arch Linux Server v${program.version()}`);
    logger.info(`Listening on ${host}:${port}`);
    
    await server.listen({ host, port });
    
    // Graceful shutdown
    const shutdown = async (signal) => {
      logger.info(`Received ${signal}, shutting down gracefully...`);
      try {
        await server.close();
        await security.cleanup();
        logger.info('Server shut down successfully');
        process.exit(0);
      } catch (error) {
        logger.error('Error during shutdown:', error);
        process.exit(1);
      }
    };
    
    process.on('SIGTERM', () => shutdown('SIGTERM'));
    process.on('SIGINT', () => shutdown('SIGINT'));
    
  } catch (error) {
    console.error('Failed to start server:', error);
    process.exit(1);
  }
}

// Handle unhandled promise rejections
process.on('unhandledRejection', (reason, promise) => {
  console.error('Unhandled Rejection at:', promise, 'reason:', reason);
  process.exit(1);
});

// Handle uncaught exceptions
process.on('uncaughtException', (error) => {
  console.error('Uncaught Exception:', error);
  process.exit(1);
});

main().catch(console.error);