/**
 * Integration example: Using the audit pipeline in MCP interceptor
 * 
 * This demonstrates how to integrate the audit system into your MCP proxy
 * to log all tool invocations with cryptographic integrity.
 */

import { initAuditPipeline, AuditPipeline } from './index';

// Initialize audit pipeline
const audit: AuditPipeline = initAuditPipeline();

// Set up anomaly detection callback
audit.anomalyDetector.setAnomalyCallback((event) => {
  audit.logger.error('anomaly_detected', event.message, {
    anomaly_type: event.type,
    severity: event.severity,
    metadata: event.metadata,
  });
});

/**
 * Example: Intercept MCP tool call with full audit trail
 */
export class MCPInterceptor {
  private audit: AuditPipeline;

  constructor() {
    this.audit = initAuditPipeline();
  }

  async interceptCall(
    toolName: string,
    params: any,
    context: {
      sessionId: string;
      agentId: string;
      correlationId?: string;
    }
  ): Promise<any> {
    // Set session context
    this.audit.logger.setSessionId(context.sessionId, context.agentId);

    // Start trace
    const correlationId = context.correlationId || this.audit.logger['_generateCorrelationId']();
    this.audit.tracer.startTrace(correlationId);
    this.audit.tracer.startSpan('mcp_call_started');

    // Log invocation with tamper evidence
    this.audit.logger.info('mcp_call_started', 'MCP tool invocation', {
      tool_invoked: toolName,
      parameters_used: params,
      agent_id: context.agentId,
    });

    // Record request for anomaly detection
    this.audit.anomalyDetector.recordRequest(true, 'mcp_call');

    try {
      // Execute tool call
      this.audit.tracer.startSpan('tool_execution');
      const response = await this.executeCall(toolName, params);
      this.audit.tracer.endSpan();

      // Log success
      this.audit.logger.info('mcp_call_completed', 'Tool succeeded', {
        tool_invoked: toolName,
        response_summary: typeof response === 'object' 
          ? JSON.stringify(response).substring(0, 500) 
          : String(response).substring(0, 500),
        duration_ms: Date.now(), // Would calculate actual duration
      });

      // Store in append-only format
      await this.audit.store.append({
        event_type: 'mcp_call_completed',
        tool_invoked: toolName,
        timestamp: new Date().toISOString(),
        session_id: context.sessionId,
        agent_id: context.agentId,
      });

      // Forward to SIEM
      await this.audit.forwarder.forward({
        event_type: 'mcp_call_completed',
        tool_invoked: toolName,
        timestamp: new Date().toISOString(),
        session_id: context.sessionId,
        agent_id: context.agentId,
      });

      this.audit.tracer.endSpan();
      return response;

    } catch (error: any) {
      // Log error
      this.audit.logger.error('mcp_call_failed', String(error), {
        tool_invoked: toolName,
        error: String(error),
        error_type: error?.constructor?.name || 'Unknown',
      });

      // Record failure for anomaly detection
      this.audit.anomalyDetector.recordRequest(false, 'mcp_call');

      // Store error
      await this.audit.store.append({
        event_type: 'mcp_call_failed',
        tool_invoked: toolName,
        error: String(error),
        timestamp: new Date().toISOString(),
        session_id: context.sessionId,
        agent_id: context.agentId,
      });

      this.audit.tracer.endSpan();
      throw error;
    }
  }

  /**
   * Example: Handle authentication with PII tokenization
   */
  async handleAuth(credentials: { username: string; password: string; email?: string }): Promise<boolean> {
    // Tokenize PII before logging
    const tokenizedEmail = credentials.email 
      ? this.audit.piiTokenizer.tokenizePII(credentials.email, 'email')
      : undefined;

    this.audit.logger.info('auth_attempt', 'Authentication attempt', {
      username: credentials.username,
      email: tokenizedEmail, // Tokenized, not plain email
      // Password will be encrypted if FIELD_ENCRYPTION is enabled
      password: credentials.password,
    });

    try {
      const success = await this.authenticate(credentials);
      
      if (!success) {
        this.audit.anomalyDetector.recordAuthFailure(credentials.username);
      }

      return success;
    } catch (error: any) {
      this.audit.logger.error('auth_error', String(error), {
        username: credentials.username,
      });
      throw error;
    }
  }

  /**
   * Example: Query audit logs
   */
  async queryAuditLogs(filters: {
    event_type?: string;
    session_id?: string;
    user_id?: string;
    level?: string;
  }): Promise<any[]> {
    return await this.audit.store.query(filters);
  }

  /**
   * Example: Verify log file integrity
   */
  async verifyLogIntegrity(filePath: string): Promise<boolean> {
    return await this.audit.store.verifyIntegrity(filePath);
  }

  /**
   * Flush SIEM queue (call on shutdown)
   */
  async flushSIEM(): Promise<void> {
    await this.audit.forwarder.flush();
  }

  // Placeholder methods
  private async executeCall(toolName: string, params: any): Promise<any> {
    // Your actual tool execution logic
    return { result: 'success' };
  }

  private async authenticate(credentials: any): Promise<boolean> {
    // Your actual authentication logic
    return true;
  }
}

/**
 * Usage example:
 * 
 * const interceptor = new MCPInterceptor();
 * 
 * await interceptor.interceptCall('filesystem_read', {
 *   path: '/etc/passwd'
 * }, {
 *   sessionId: 'sess-123',
 *   agentId: 'agent-456'
 * });
 */
