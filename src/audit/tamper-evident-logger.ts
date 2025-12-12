import { AUDIT_CONFIG } from './config';
import { CryptoEngine } from './crypto';

export interface TamperEvidenceLogEntry {
  timestamp: string;
  level: 'INFO' | 'WARN' | 'ERROR' | 'CRITICAL';
  event_type: string;
  session_id: string;
  agent_id: string;
  user_id: string;
  correlation_id: string;
  tool_invoked?: string;
  parameters_used?: Record<string, any>;
  response_summary?: any;
  schema_version: string;
  context_snapshot: Record<string, any>;
  classification: string;  // PUBLIC, CONFIDENTIAL, SECRET, RESTRICTED
  payload?: Record<string, any>;
  encrypted_fields: string[];  // List of encrypted field names
  hmac_signature: string;       // HMAC-SHA256 signature
  source_file: string;
  tags: string[];
}

export class TamperEvidenceLogger {
  private sessionId: string = '';
  private agentId: string = '';
  private crypto: CryptoEngine;

  constructor() {
    this.crypto = new CryptoEngine();
    if (!AUDIT_CONFIG.features.STRUCTURED_LOGGING) {
      console.log('[AUDIT] Structured logging disabled');
      return;
    }
  }

  setSessionId(sessionId: string, agentId: string = 'unknown'): void {
    this.sessionId = sessionId;
    this.agentId = agentId;
  }

  info(eventType: string, message: string, payload?: Record<string, any>): void {
    if (!AUDIT_CONFIG.features.STRUCTURED_LOGGING) return;
    this._log('INFO', eventType, message, payload);
  }

  warn(eventType: string, message: string, payload?: Record<string, any>): void {
    if (!AUDIT_CONFIG.features.STRUCTURED_LOGGING) return;
    this._log('WARN', eventType, message, payload);
  }

  error(eventType: string, message: string, payload?: Record<string, any>): void {
    if (!AUDIT_CONFIG.features.STRUCTURED_LOGGING) return;
    this._log('ERROR', eventType, message, payload);
  }

  critical(eventType: string, message: string, payload?: Record<string, any>): void {
    if (!AUDIT_CONFIG.features.STRUCTURED_LOGGING) return;
    this._log('CRITICAL', eventType, message, payload);
  }

  private _log(
    level: string,
    eventType: string,
    message: string,
    payload?: Record<string, any>
  ): void {
    // Build base entry
    let entry: any = {
      timestamp: new Date().toISOString(),
      level,
      event_type: eventType,
      session_id: this.sessionId || 'unknown',
      agent_id: this.agentId,
      user_id: process.env.SENTINEL_USER_ID || 'anonymous',
      correlation_id: this._generateCorrelationId(),
      schema_version: '1.0',
      context_snapshot: {
        environment: process.env.NODE_ENV || 'development',
        timestamp_ms: Date.now(),
      },
      classification: 'CONFIDENTIAL',
      source_file: this._getSourceFile(),
      tags: [eventType.split('_')[0]],
      payload: payload || {},
      encrypted_fields: [],
    };

    // Encrypt sensitive fields
    if (AUDIT_CONFIG.features.FIELD_ENCRYPTION && payload) {
      entry = this._encryptSensitiveFields(entry);
    }

    // Generate HMAC signature
    if (AUDIT_CONFIG.features.TAMPER_EVIDENT) {
      const entryForSignature = { ...entry };
      delete entryForSignature.hmac_signature;
      entry.hmac_signature = this.crypto.signLogEntry(entryForSignature);
    } else {
      entry.hmac_signature = '';
    }

    // Output JSON
    console.log(JSON.stringify(entry));
  }

  private _encryptSensitiveFields(entry: any): any {
    const fieldsToEncrypt = AUDIT_CONFIG.crypto.encryptFields;
    for (const fieldName of fieldsToEncrypt) {
      if (entry.payload && entry.payload[fieldName]) {
        const plaintext = String(entry.payload[fieldName]);
        entry.payload[fieldName] = this.crypto.encryptField(plaintext, entry.session_id);
        entry.encrypted_fields.push(fieldName);
      }
    }
    return entry;
  }

  private _generateCorrelationId(): string {
    return 'req-' + Math.random().toString(36).substr(2, 9);
  }

  private _getSourceFile(): string {
    const stack = new Error().stack || '';
    const lines = stack.split('\n');
    return lines[3]?.match(/at (.+):/)?.[1] || 'unknown.ts:0';
  }
}
