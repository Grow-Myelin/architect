export class MCPProtocol {
  constructor(pluginManager, logger, security) {
    this.pluginManager = pluginManager;
    this.logger = logger;
    this.security = security;
    this.initialized = false;
    this.clientInfo = null;
  }

  async handleRequest(request) {
    try {
      // Validate JSON-RPC format
      if (!request || request.jsonrpc !== '2.0' || !request.method) {
        return this.createError(-32600, 'Invalid Request', request?.id || null);
      }

      const { method, params, id } = request;

      // Handle MCP protocol methods
      switch (method) {
        case 'initialize':
          return this.handleInitialize(params, id);
        
        case 'initialized':
          return this.handleInitialized(params, id);
        
        case 'tools/list':
          return this.handleToolsList(params, id);
        
        case 'tools/call':
          return this.handleToolCall(params, id);
        
        case 'resources/list':
          return this.handleResourcesList(params, id);
        
        case 'resources/read':
          return this.handleResourceRead(params, id);
        
        case 'completion/complete':
          return this.handleCompletion(params, id);
        
        default:
          return this.createError(-32601, 'Method not found', id);
      }
    } catch (error) {
      this.logger.error('Protocol error:', error);
      return this.createError(-32603, 'Internal error', request?.id || null);
    }
  }

  async handleInitialize(params, id) {
    try {
      this.clientInfo = params.clientInfo;
      this.logger.info(`Client initialized: ${this.clientInfo?.name} v${this.clientInfo?.version}`);

      const result = {
        protocolVersion: '2024-11-05',
        capabilities: {
          tools: { listChanged: true },
          resources: { 
            subscribe: true,
            listChanged: true 
          },
          prompts: { listChanged: true }
        },
        serverInfo: {
          name: 'mcp-arch-linux',
          version: '1.0.0'
        }
      };

      return this.createResponse(result, id);
    } catch (error) {
      this.logger.error('Initialize error:', error);
      return this.createError(-32603, 'Initialization failed', id);
    }
  }

  async handleInitialized(params, id) {
    this.initialized = true;
    this.logger.info('Client initialization complete');
    return this.createResponse({}, id);
  }

  async handleToolsList(params, id) {
    try {
      if (!this.initialized) {
        return this.createError(-32002, 'Server not initialized', id);
      }

      const tools = await this.pluginManager.getAllTools();
      return this.createResponse({ tools }, id);
    } catch (error) {
      this.logger.error('Tools list error:', error);
      return this.createError(-32603, 'Failed to list tools', id);
    }
  }

  async handleToolCall(params, id) {
    try {
      if (!this.initialized) {
        return this.createError(-32002, 'Server not initialized', id);
      }

      const { name, arguments: toolArgs } = params;
      if (!name) {
        return this.createError(-32602, 'Missing tool name', id);
      }

      // Execute tool with security audit
      const result = await this.security.executeWithAudit(
        'tool_call',
        { tool: name, arguments: toolArgs },
        async () => {
          return await this.pluginManager.executeTool(name, toolArgs || {});
        }
      );

      return this.createResponse(result, id);
    } catch (error) {
      this.logger.error('Tool call error:', error);
      return this.createError(-32603, error.message, id);
    }
  }

  async handleResourcesList(params, id) {
    try {
      if (!this.initialized) {
        return this.createError(-32002, 'Server not initialized', id);
      }

      const resources = await this.pluginManager.getAllResources();
      return this.createResponse({ resources }, id);
    } catch (error) {
      this.logger.error('Resources list error:', error);
      return this.createError(-32603, 'Failed to list resources', id);
    }
  }

  async handleResourceRead(params, id) {
    try {
      if (!this.initialized) {
        return this.createError(-32002, 'Server not initialized', id);
      }

      const { uri } = params;
      if (!uri) {
        return this.createError(-32602, 'Missing resource URI', id);
      }

      const result = await this.security.executeWithAudit(
        'resource_read',
        { uri },
        async () => {
          return await this.pluginManager.readResource(uri);
        }
      );

      return this.createResponse(result, id);
    } catch (error) {
      this.logger.error('Resource read error:', error);
      return this.createError(-32603, error.message, id);
    }
  }

  async handleCompletion(params, id) {
    // Simple completion implementation
    return this.createResponse({
      completion: {
        values: [],
        total: 0,
        hasMore: false
      }
    }, id);
  }

  createResponse(result, id) {
    return {
      jsonrpc: '2.0',
      result,
      id
    };
  }

  createError(code, message, id) {
    return {
      jsonrpc: '2.0',
      error: {
        code,
        message
      },
      id
    };
  }
}