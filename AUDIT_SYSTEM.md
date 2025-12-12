# Enterprise-Grade OWASP MCP08 Audit & Telemetry System

## Overview

A comprehensive audit and telemetry system with cryptographic integrity, field-level encryption, and SIEM integration for the Sentinel MCP interceptor.

## Architecture

```
LOG → HMAC HASH → AES-256-GCM ENCRYPT → APPEND-ONLY FILE
     → VERIFY HASH → DECRYPT → READ
     → SIGN FILE → IMMUTABLE → FORWARD TO SIEM
```

## Components

### TypeScript Modules (10 modules)

Located in `src/audit/`:

1. **crypto.ts** - Cryptographic operations (HMAC-SHA256, AES-256-GCM, SHA-256)
2. **config.ts** - Central configuration with 10 feature toggles
3. **tamper-evident-logger.ts** - Structured logging with cryptographic integrity
4. **append-only-store.ts** - Immutable WORM storage with integrity verification
5. **siem-forwarder.ts** - SIEM forwarding (Splunk, ELK, Azure Sentinel, Google Chronicle)
6. **pii-tokenizer.ts** - PII tokenization and masking
7. **opentelemetry-tracing.ts** - OpenTelemetry tracing for end-to-end correlation
8. **anomaly-detector.ts** - Anomaly detection (error rates, auth failures, rate limits)
9. **index.ts** - Main exports and pipeline initialization
10. **integration-example.ts** - Usage examples

### Rust Binaries (2 binaries)

Located in `src/bin/`:

1. **crypto-engine.rs** - High-performance cryptographic operations
   - Sign log files
   - Verify log integrity
   - Encrypt log entries

2. **log-signer.rs** - Log file integrity signer
   - Sign log files with HMAC
   - Verify signatures
   - Batch processing

### Python Modules (3 modules)

Located in `python/`:

1. **audit_crypto.py** - Cryptographic operations (Python implementation)
2. **audit_logger.py** - Tamper-evident structured logging
3. **audit_store.py** - Append-only storage

## Key Features

### ✅ Tamper-Evident Logging
- HMAC-SHA256 signature per log entry
- Automatic integrity verification
- Tamper detection alerts

### ✅ Field-Level Encryption
- AES-256-GCM encryption for sensitive fields
- Configurable field list (password, api_key, token, etc.)
- Authenticated encryption with additional data

### ✅ Append-Only Storage
- Immutable log files (WORM - Write-Once-Read-Many)
- Automatic file rotation
- Integrity verification on read

### ✅ SIEM Integration
- Splunk HEC (HTTP Event Collector)
- ELK Stack (Elasticsearch bulk API)
- Azure Sentinel
- Google Chronicle

### ✅ PII-Safe Logging
- Tokenization of PII fields
- Email masking
- Phone number masking
- SSN masking

### ✅ OpenTelemetry Tracing
- End-to-end request correlation
- Span tracking
- OTLP export

### ✅ Anomaly Detection
- Error rate monitoring
- Authentication failure tracking
- Rate limit enforcement

### ✅ Data Classification
- Labels: PUBLIC, CONFIDENTIAL, SECRET, RESTRICTED
- Retention governance
- Compliance tagging

## Configuration

Copy `env.audit.example` to `.env.audit` and configure:

```bash
# Generate cryptographic keys
openssl rand -hex 32  # For AUDIT_HMAC_KEY
openssl rand -hex 32  # For AUDIT_ENCRYPTION_KEY
```

### Feature Toggles

All features can be enabled/disabled independently:

- `SENTINEL_FEATURE_LOGGING` - Structured logging
- `SENTINEL_FEATURE_STORAGE` - Append-only storage
- `SENTINEL_FEATURE_ENCRYPTION` - Field-level encryption
- `SENTINEL_FEATURE_TAMPER_EVIDENT` - HMAC signing
- `SENTINEL_FEATURE_SIEM` - SIEM forwarding
- `SENTINEL_FEATURE_OTLP` - OpenTelemetry export
- `SENTINEL_FEATURE_ANOMALY` - Anomaly detection
- `SENTINEL_FEATURE_REDACTION` - PII redaction
- `SENTINEL_FEATURE_SESSIONS` - Session tracking
- `SENTINEL_FEATURE_COMPLIANCE` - Compliance reports

## Usage

### TypeScript

```typescript
import { initAuditPipeline } from './audit';

const audit = initAuditPipeline();

// Set session context
audit.logger.setSessionId('sess-123', 'agent-456');

// Log with tamper evidence
audit.logger.info('tool_invoked', 'MCP tool called', {
  tool: 'filesystem_read',
  path: '/etc/passwd',
});

// Store in append-only format
await audit.store.append({ /* log entry */ });

// Forward to SIEM
await audit.forwarder.forward({ /* log entry */ });
```

### Rust

```bash
# Build binaries
cargo build --release

# Sign log file
./target/release/crypto-engine --mode sign --input log.jsonl

# Verify integrity
./target/release/crypto-engine --mode verify --input log.jsonl

# Sign with specific key
./target/release/log-signer --file audit-2024-01-01.jsonl --key <hex-key>
```

### Python

```python
from audit_logger import TamperEvidenceLogger
from audit_store import AppendOnlyStore

logger = TamperEvidenceLogger()
logger.set_session_id('sess-123', 'agent-456')
logger.info('tool_invoked', 'MCP tool called', {'tool': 'filesystem_read'})

store = AppendOnlyStore()
store.append({ /* log entry */ })
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

## File Structure

```
sentinel/
├── src/
│   ├── audit/                    # TypeScript audit modules
│   │   ├── crypto.ts
│   │   ├── config.ts
│   │   ├── tamper-evident-logger.ts
│   │   ├── append-only-store.ts
│   │   ├── siem-forwarder.ts
│   │   ├── pii-tokenizer.ts
│   │   ├── opentelemetry-tracing.ts
│   │   ├── anomaly-detector.ts
│   │   ├── index.ts
│   │   ├── integration-example.ts
│   │   ├── package.json
│   │   ├── tsconfig.json
│   │   └── README.md
│   └── bin/                      # Rust binaries
│       ├── crypto-engine.rs
│       └── log-signer.rs
├── python/                       # Python modules
│   ├── audit_crypto.py
│   ├── audit_logger.py
│   ├── audit_store.py
│   ├── requirements.txt
│   └── __init__.py
├── env.audit.example             # Configuration template
└── AUDIT_SYSTEM.md              # This file
```

## Next Steps

1. Copy `env.audit.example` to `.env.audit` and configure
2. Generate cryptographic keys using OpenSSL
3. Enable desired features via environment variables
4. Integrate into your MCP interceptor (see `src/audit/integration-example.ts`)
5. Configure SIEM endpoint and credentials
6. Set up monitoring for tamper detection alerts

## License

MIT
