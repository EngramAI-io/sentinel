# Sentinel Audit & Telemetry System

Enterprise-grade OWASP MCP08 audit & telemetry with cryptographic integrity, field-level encryption, and SIEM integration.

## Features

✅ **Tamper-Evident Logging**: HMAC-SHA256 signature per log entry  
✅ **Field-Level Encryption**: AES-256-GCM for secrets, tokens, PII  
✅ **Append-Only Storage**: Immutable log files with integrity verification  
✅ **SIEM Integration**: Forward to Splunk, ELK, Azure Sentinel, Google Chronicle  
✅ **PII-Safe Logging**: Tokenization + masking of sensitive fields  
✅ **OpenTelemetry Tracing**: End-to-end request correlation  
✅ **Data Classification**: Labels for retention governance  
✅ **Modular Design**: 10 independent feature toggles  
✅ **Cross-Platform**: TypeScript, Rust, and Python implementations  

## Architecture

```
LOG → HMAC HASH → AES-256-GCM ENCRYPT → APPEND-ONLY FILE
     → VERIFY HASH → DECRYPT → READ
     → SIGN FILE → IMMUTABLE → FORWARD TO SIEM
```

## Quick Start

### 1. Install Dependencies

```bash
# TypeScript
cd src/audit
npm install

# Python (optional)
cd python
pip install -r requirements.txt
```

### 2. Configure Environment

Copy `.env.audit.example` to `.env.audit` and configure:

```bash
# Generate keys
openssl rand -hex 32 > hmac_key.txt
openssl rand -hex 32 > encryption_key.txt

# Set in .env.audit
AUDIT_HMAC_KEY=<from hmac_key.txt>
AUDIT_ENCRYPTION_KEY=<from encryption_key.txt>
```

### 3. Use in Your Code

```typescript
import { initAuditPipeline } from './audit';

const audit = initAuditPipeline();

// Set session context
audit.logger.setSessionId('sess-123', 'agent-456');

// Log with tamper evidence
audit.logger.info('tool_invoked', 'MCP tool called', {
  tool: 'filesystem_read',
  path: '/etc/passwd',
  // password field will be encrypted if FIELD_ENCRYPTION is enabled
});

// Store in append-only format
await audit.store.append({ /* log entry */ });

// Forward to SIEM
await audit.forwarder.forward({ /* log entry */ });
```

## TypeScript Modules

- **crypto.ts**: Cryptographic operations (HMAC, AES-256-GCM, hashing)
- **config.ts**: Central configuration with feature toggles
- **tamper-evident-logger.ts**: Structured logging with cryptographic integrity
- **append-only-store.ts**: Immutable WORM storage
- **siem-forwarder.ts**: SIEM forwarding (Splunk, ELK, Sentinel, Chronicle)
- **pii-tokenizer.ts**: PII tokenization and masking
- **opentelemetry-tracing.ts**: OpenTelemetry tracing
- **anomaly-detector.ts**: Anomaly detection
- **index.ts**: Main exports and initialization

## Rust Binaries

### crypto-engine

High-performance cryptographic operations:

```bash
cargo build --bin crypto-engine --release
./target/release/crypto-engine --mode sign --input log.jsonl
./target/release/crypto-engine --mode verify --input log.jsonl
```

### log-signer

Log file integrity signer:

```bash
cargo build --bin log-signer --release
./target/release/log-signer --file audit-2024-01-01.jsonl --key <hex-key>
./target/release/log-signer --file audit-2024-01-01.jsonl --key <hex-key> --verify
```

## Python Modules

Optional Python implementation:

```python
from audit_crypto import CryptoEngine
from audit_logger import TamperEvidenceLogger
from audit_store import AppendOnlyStore

logger = TamperEvidenceLogger()
logger.set_session_id('sess-123', 'agent-456')
logger.info('tool_invoked', 'MCP tool called', {'tool': 'filesystem_read'})
```

## Feature Toggles

All features can be enabled/disabled independently via environment variables:

- `SENTINEL_FEATURE_LOGGING`: Structured logging
- `SENTINEL_FEATURE_STORAGE`: Append-only storage
- `SENTINEL_FEATURE_ENCRYPTION`: Field-level encryption
- `SENTINEL_FEATURE_TAMPER_EVIDENT`: HMAC signing
- `SENTINEL_FEATURE_SIEM`: SIEM forwarding
- `SENTINEL_FEATURE_OTLP`: OpenTelemetry export
- `SENTINEL_FEATURE_ANOMALY`: Anomaly detection
- `SENTINEL_FEATURE_REDACTION`: PII redaction
- `SENTINEL_FEATURE_SESSIONS`: Session tracking
- `SENTINEL_FEATURE_COMPLIANCE`: Compliance reports

## SIEM Integration

### Splunk

```bash
SIEM_TYPE=splunk
SIEM_ENDPOINT=https://your-splunk.com:8088
SIEM_TOKEN=your-token
```

### ELK Stack

```bash
SIEM_TYPE=elk
SIEM_ENDPOINT=https://your-elasticsearch.com:9200
SIEM_TOKEN=your-token
```

### Azure Sentinel

```bash
SIEM_TYPE=sentinel
SIEM_ENDPOINT=https://your-workspace.azure.com/api/logs
SIEM_TOKEN=your-token
```

### Google Chronicle

```bash
SIEM_TYPE=chronicle
SIEM_ENDPOINT=https://your-chronicle.com/api/v1/ingest
SIEM_TOKEN=your-token
```

## Security Best Practices

1. **Key Management**: Store keys in secure key management systems (AWS KMS, HashiCorp Vault)
2. **Key Rotation**: Rotate HMAC and encryption keys periodically
3. **Access Control**: Restrict access to audit logs and keys
4. **Network Security**: Use TLS for SIEM forwarding
5. **Backup**: Regularly backup audit logs with integrity verification
6. **Monitoring**: Monitor for tamper detection alerts

## Compliance

Supports compliance frameworks:
- SOC 2
- HIPAA
- GDPR
- PCI-DSS

Configure via `COMPLIANCE_FRAMEWORK` environment variable.

## License

MIT
