import Fastify from 'fastify';
import cors from '@fastify/cors';
import { MCPProtocol } from './mcp-protocol.js';
import { PluginManager } from './plugin-manager.js';

// Import plugins
import { SystemPlugin } from '../plugins/system-plugin.js';
import { ArchInstallPlugin } from '../plugins/arch-install-plugin.js';
import { HyprlandPlugin } from '../plugins/hyprland-plugin.js';
import { ScreenCapturePlugin } from '../plugins/screen-capture-plugin.js';

export async function createServer(config, logger, security) {
  const fastify = Fastify({
    logger: false, // We use our own logger
    trustProxy: true
  });

  // Register CORS
  await fastify.register(cors, {
    origin: true,
    credentials: true
  });

  // Initialize plugin manager
  const pluginManager = new PluginManager(logger, security);
  
  // Register plugins
  await pluginManager.register(new SystemPlugin(config, logger, security));
  await pluginManager.register(new ArchInstallPlugin(config, logger, security));
  await pluginManager.register(new HyprlandPlugin(config, logger, security));
  await pluginManager.register(new ScreenCapturePlugin(config, logger, security));

  // Initialize MCP protocol handler
  const mcpProtocol = new MCPProtocol(pluginManager, logger, security);

  // Health check endpoint
  fastify.get('/health', async (request, reply) => {
    return {
      status: 'healthy',
      version: '1.0.0',
      timestamp: new Date().toISOString(),
      plugins: await pluginManager.getPluginList()
    };
  });

  // MCP protocol endpoint (JSON-RPC over HTTP)
  fastify.post('/mcp', async (request, reply) => {
    try {
      const response = await mcpProtocol.handleRequest(request.body);
      reply.type('application/json');
      return response;
    } catch (error) {
      logger.error('MCP request error:', error);
      reply.code(500);
      return {
        jsonrpc: '2.0',
        error: {
          code: -32603,
          message: 'Internal error',
          data: error.message
        },
        id: request.body?.id || null
      };
    }
  });

  // WebSocket endpoint for MCP protocol
  fastify.register(async function (fastify) {
    fastify.get('/mcp/ws', { websocket: true }, (connection, request) => {
      logger.info('New WebSocket connection established');

      connection.on('message', async (message) => {
        try {
          const data = JSON.parse(message.toString());
          const response = await mcpProtocol.handleRequest(data);
          connection.send(JSON.stringify(response));
        } catch (error) {
          logger.error('WebSocket message error:', error);
          connection.send(JSON.stringify({
            jsonrpc: '2.0',
            error: {
              code: -32603,
              message: 'Internal error',
              data: error.message
            },
            id: null
          }));
        }
      });

      connection.on('close', () => {
        logger.info('WebSocket connection closed');
      });

      connection.on('error', (error) => {
        logger.error('WebSocket error:', error);
      });
    });
  });

  // Request logging middleware
  fastify.addHook('onRequest', async (request) => {
    logger.debug(`${request.method} ${request.url}`, {
      ip: request.ip,
      userAgent: request.headers['user-agent']
    });
  });

  // Error handler
  fastify.setErrorHandler(async (error, request, reply) => {
    logger.error('Request error:', error);
    reply.code(500).send({
      error: 'Internal Server Error',
      message: error.message
    });
  });

  return fastify;
}