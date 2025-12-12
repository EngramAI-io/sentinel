function parseHeaders(headerString: string): Record<string, string> {
  if (!headerString) return {};
  return Object.fromEntries(
    headerString.split(',').map(h => {
      const [key, value] = h.split('=');
      return [key.trim(), value.trim()];
    })
  );
}

export const AUDIT_CONFIG = {
  // Feature flags (independent toggles)
  features: {
    STRUCTURED_LOGGING: process.env.SENTINEL_FEATURE_LOGGING === 'true',
    AUDIT_STORAGE: process.env.SENTINEL_FEATURE_STORAGE === 'true',
    SESSION_TRACKING: process.env.SENTINEL_FEATURE_SESSIONS === 'true',
    PRIVACY_REDACTION: process.env.SENTINEL_FEATURE_REDACTION === 'true',
    FIELD_ENCRYPTION: process.env.SENTINEL_FEATURE_ENCRYPTION === 'true',
    TAMPER_EVIDENT: process.env.SENTINEL_FEATURE_TAMPER_EVIDENT === 'true',
    ANOMALY_DETECTION: process.env.SENTINEL_FEATURE_ANOMALY === 'true',
    OTLP_EXPORT: process.env.SENTINEL_FEATURE_OTLP === 'true',
    SIEM_FORWARDING: process.env.SENTINEL_FEATURE_SIEM === 'true',
    COMPLIANCE_REPORTS: process.env.SENTINEL_FEATURE_COMPLIANCE === 'true',
  },

  // Cryptography
  crypto: {
    hmacKey: process.env.AUDIT_HMAC_KEY || '',
    encryptionKey: process.env.AUDIT_ENCRYPTION_KEY || '',
    algorithm: 'aes-256-gcm',
    encryptFields: (process.env.AUDIT_ENCRYPT_FIELDS || 'password,api_key,token,db_url').split(','),
  },

  // Logging
  logging: {
    dir: process.env.AUDIT_LOG_DIR || './logs/audit',
    level: process.env.AUDIT_LOG_LEVEL || 'INFO',
    rustEngine: process.env.USE_RUST_ENGINE === 'true',
  },

  // Storage (Append-only, WORM)
  storage: {
    retentionDays: parseInt(process.env.AUDIT_LOG_RETENTION_DAYS || '90', 10),
    maxFileSize: parseInt(process.env.AUDIT_LOG_MAX_FILE_SIZE || '104857600', 10),
    appendOnly: true,
    writeOnce: process.env.AUDIT_WRITE_ONCE === 'true',
    s3Lock: process.env.AUDIT_S3_OBJECT_LOCK === 'true', // AWS S3 Object Lock
  },

  // Field Classification (for SIEM tags)
  fieldClassification: {
    PUBLIC: 'public',
    CONFIDENTIAL: 'confidential',
    SECRET: 'secret',
    RESTRICTED: 'restricted',
  },

  // SIEM Forwarding
  siem: {
    enabled: process.env.SENTINEL_FEATURE_SIEM === 'true',
    type: process.env.SIEM_TYPE || 'splunk', // splunk, elk, sentinel, chronicle
    endpoint: process.env.SIEM_ENDPOINT || '',
    token: process.env.SIEM_TOKEN || '',
    batchSize: parseInt(process.env.SIEM_BATCH_SIZE || '100', 10),
  },

  // OpenTelemetry
  otlp: {
    endpoint: process.env.OTEL_EXPORTER_OTLP_ENDPOINT || 'http://localhost:4318/v1/traces',
    headers: parseHeaders(process.env.OTEL_EXPORTER_OTLP_HEADERS || ''),
    samplingRate: parseFloat(process.env.OTEL_SAMPLING_RATE || '1.0'),
  },

  // Privacy & PII
  privacy: {
    redactionMode: process.env.REDACTION_MODE || 'anonymize',
    tokenizePII: process.env.AUDIT_TOKENIZE_PII === 'true',
    maskEmails: process.env.AUDIT_MASK_EMAILS === 'true',
    maskPhones: process.env.AUDIT_MASK_PHONES === 'true',
  },

  // Anomaly Detection
  anomaly: {
    errorRateThreshold: parseInt(process.env.RULE_ERROR_RATE_THRESHOLD || '50', 10),
    authFailureLimit: parseInt(process.env.RULE_AUTH_FAILURES || '3', 10),
    rateLimit: parseInt(process.env.RULE_RATE_LIMIT || '100', 10),
  },
};
