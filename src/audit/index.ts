export { TamperEvidenceLogger } from './tamper-evident-logger';
export { AppendOnlyStore } from './append-only-store';
export { SIEMForwarder } from './siem-forwarder';
export { PIITokenizer } from './pii-tokenizer';
export { OpenTelemetryTracer } from './opentelemetry-tracing';
export { CryptoEngine } from './crypto';
export { AnomalyDetector } from './anomaly-detector';
export { AUDIT_CONFIG } from './config';
export type { TamperEvidenceLogEntry } from './tamper-evident-logger';
export type { EncryptedField } from './crypto';
export type { AnomalyEvent } from './anomaly-detector';

import { TamperEvidenceLogger } from './tamper-evident-logger';
import { AppendOnlyStore } from './append-only-store';
import { SIEMForwarder } from './siem-forwarder';
import { PIITokenizer } from './pii-tokenizer';
import { OpenTelemetryTracer } from './opentelemetry-tracing';
import { AnomalyDetector } from './anomaly-detector';

export interface AuditPipeline {
  logger: TamperEvidenceLogger;
  store: AppendOnlyStore;
  forwarder: SIEMForwarder;
  piiTokenizer: PIITokenizer;
  tracer: OpenTelemetryTracer;
  anomalyDetector: AnomalyDetector;
}

export function initAuditPipeline(): AuditPipeline {
  const logger = new TamperEvidenceLogger();
  const store = new AppendOnlyStore();
  const forwarder = new SIEMForwarder();
  const piiTokenizer = new PIITokenizer();
  const tracer = new OpenTelemetryTracer();
  const anomalyDetector = new AnomalyDetector();

  return { logger, store, forwarder, piiTokenizer, tracer, anomalyDetector };
}
