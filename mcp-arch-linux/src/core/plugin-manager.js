export class PluginManager {
  constructor(logger, security) {
    this.logger = logger;
    this.security = security;
    this.plugins = new Map();
    this.tools = new Map();
    this.resources = new Map();
  }

  async register(plugin) {
    try {
      if (!plugin.name) {
        throw new Error('Plugin must have a name');
      }

      if (this.plugins.has(plugin.name)) {
        throw new Error(`Plugin ${plugin.name} is already registered`);
      }

      // Initialize plugin
      if (plugin.initialize) {
        await plugin.initialize();
      }

      // Register plugin tools
      const tools = await plugin.getTools();
      for (const tool of tools) {
        if (this.tools.has(tool.name)) {
          throw new Error(`Tool ${tool.name} is already registered by another plugin`);
        }
        this.tools.set(tool.name, { tool, plugin });
      }

      // Register plugin resources
      const resources = await plugin.getResources();
      for (const resource of resources) {
        if (this.resources.has(resource.uri)) {
          throw new Error(`Resource ${resource.uri} is already registered by another plugin`);
        }
        this.resources.set(resource.uri, { resource, plugin });
      }

      this.plugins.set(plugin.name, plugin);
      this.logger.info(`Registered plugin: ${plugin.name}`);

    } catch (error) {
      this.logger.error(`Failed to register plugin ${plugin.name}:`, error);
      throw error;
    }
  }

  async unregister(pluginName) {
    const plugin = this.plugins.get(pluginName);
    if (!plugin) {
      throw new Error(`Plugin ${pluginName} not found`);
    }

    // Remove tools
    for (const [toolName, { plugin: toolPlugin }] of this.tools.entries()) {
      if (toolPlugin === plugin) {
        this.tools.delete(toolName);
      }
    }

    // Remove resources
    for (const [uri, { plugin: resourcePlugin }] of this.resources.entries()) {
      if (resourcePlugin === plugin) {
        this.resources.delete(uri);
      }
    }

    // Cleanup plugin
    if (plugin.cleanup) {
      await plugin.cleanup();
    }

    this.plugins.delete(pluginName);
    this.logger.info(`Unregistered plugin: ${pluginName}`);
  }

  async getAllTools() {
    const tools = [];
    for (const [name, { tool }] of this.tools.entries()) {
      tools.push(tool);
    }
    return tools;
  }

  async getAllResources() {
    const resources = [];
    for (const [uri, { resource }] of this.resources.entries()) {
      resources.push(resource);
    }
    return resources;
  }

  async executeTool(toolName, args) {
    const toolData = this.tools.get(toolName);
    if (!toolData) {
      throw new Error(`Tool not found: ${toolName}`);
    }

    const { plugin } = toolData;
    
    try {
      this.logger.debug(`Executing tool: ${toolName}`, { args });
      const result = await plugin.executeTool(toolName, args);
      this.logger.debug(`Tool execution completed: ${toolName}`);
      return result;
    } catch (error) {
      this.logger.error(`Tool execution failed: ${toolName}`, error);
      throw new Error(`Tool execution failed: ${error.message}`);
    }
  }

  async readResource(uri) {
    const resourceData = this.resources.get(uri);
    if (!resourceData) {
      throw new Error(`Resource not found: ${uri}`);
    }

    const { plugin } = resourceData;
    
    try {
      this.logger.debug(`Reading resource: ${uri}`);
      const result = await plugin.readResource(uri);
      this.logger.debug(`Resource read completed: ${uri}`);
      return result;
    } catch (error) {
      this.logger.error(`Resource read failed: ${uri}`, error);
      throw new Error(`Resource read failed: ${error.message}`);
    }
  }

  async getPluginList() {
    const plugins = [];
    for (const [name, plugin] of this.plugins.entries()) {
      plugins.push({
        name,
        description: plugin.description || 'No description',
        version: plugin.version || '1.0.0',
        tools: (await plugin.getTools()).length,
        resources: (await plugin.getResources()).length
      });
    }
    return plugins;
  }

  getPlugin(name) {
    return this.plugins.get(name);
  }

  async cleanup() {
    for (const [name, plugin] of this.plugins.entries()) {
      try {
        if (plugin.cleanup) {
          await plugin.cleanup();
        }
      } catch (error) {
        this.logger.error(`Error cleaning up plugin ${name}:`, error);
      }
    }
    
    this.plugins.clear();
    this.tools.clear();
    this.resources.clear();
  }
}