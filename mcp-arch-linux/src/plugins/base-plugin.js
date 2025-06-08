export class BasePlugin {
  constructor(name, config, logger, security) {
    this.name = name;
    this.config = config;
    this.logger = logger;
    this.security = security;
    this.version = '1.0.0';
    this.description = 'Base plugin';
    this.tools = [];
    this.resources = [];
  }

  async initialize() {
    this.logger.debug(`Initializing plugin: ${this.name}`);
  }

  async cleanup() {
    this.logger.debug(`Cleaning up plugin: ${this.name}`);
  }

  async getTools() {
    return this.tools;
  }

  async getResources() {
    return this.resources;
  }

  async executeTool(toolName, args) {
    throw new Error(`Tool not implemented: ${toolName}`);
  }

  async readResource(uri) {
    throw new Error(`Resource not implemented: ${uri}`);
  }

  createTool(name, description, inputSchema, handler) {
    return {
      name,
      description,
      inputSchema: inputSchema || {
        type: 'object',
        properties: {},
        required: []
      },
      handler
    };
  }

  createResource(uri, name, description, mimeType = 'text/plain') {
    return {
      uri,
      name,
      description,
      mimeType
    };
  }

  createContent(type, content, metadata = {}) {
    const baseContent = {
      type,
      ...metadata
    };

    switch (type) {
      case 'text':
        return { ...baseContent, text: content };
      case 'image':
        return { ...baseContent, data: content, mimeType: metadata.mimeType || 'image/png' };
      case 'resource':
        return { ...baseContent, uri: content };
      default:
        throw new Error(`Unknown content type: ${type}`);
    }
  }

  createResult(content, isError = false, metadata = {}) {
    return {
      content: Array.isArray(content) ? content : [content],
      isError,
      ...metadata
    };
  }

  createTextResult(text, metadata = {}) {
    return this.createResult(
      this.createContent('text', text),
      false,
      metadata
    );
  }

  createErrorResult(message, metadata = {}) {
    return this.createResult(
      this.createContent('text', message),
      true,
      metadata
    );
  }

  createImageResult(imageData, mimeType = 'image/png', metadata = {}) {
    return this.createResult(
      this.createContent('image', imageData, { mimeType }),
      false,
      metadata
    );
  }

  async validateArgs(args, schema) {
    // Basic validation
    if (schema.required) {
      for (const field of schema.required) {
        if (args[field] === undefined) {
          throw new Error(`Required argument missing: ${field}`);
        }
      }
    }

    if (schema.properties) {
      for (const [key, prop] of Object.entries(schema.properties)) {
        const value = args[key];
        
        if (value !== undefined) {
          if (prop.type && typeof value !== prop.type) {
            throw new Error(`Invalid type for argument ${key}: expected ${prop.type}, got ${typeof value}`);
          }

          if (prop.enum && !prop.enum.includes(value)) {
            throw new Error(`Invalid value for argument ${key}: must be one of ${prop.enum.join(', ')}`);
          }

          if (prop.pattern && typeof value === 'string' && !new RegExp(prop.pattern).test(value)) {
            throw new Error(`Invalid format for argument ${key}`);
          }
        }
      }
    }

    return true;
  }

  async withErrorHandling(operation, operationName) {
    try {
      const result = await operation();
      return result;
    } catch (error) {
      this.logger.error(`${this.name} plugin error in ${operationName}:`, error);
      throw error;
    }
  }
}