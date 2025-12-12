import { AUDIT_CONFIG } from './config';

export class OpenTelemetryTracer {
  private traceId: string = '';
  private spanStack: Array<{ name: string; startTime: number }> = [];

  startTrace(correlationId: string): void {
    this.traceId = correlationId;
    if (AUDIT_CONFIG.features.OTLP_EXPORT) {
      console.log('[TRACE] Started:', this.traceId);
    }
  }

  startSpan(name: string): void {
    this.spanStack.push({
      name,
      startTime: Date.now(),
    });
  }

  endSpan(): void {
    const span = this.spanStack.pop();
    if (span) {
      const duration = Date.now() - span.startTime;
      this._exportSpan(span.name, duration);
    }
  }

  getTraceId(): string {
    return this.traceId;
  }

  private async _exportSpan(spanName: string, duration: number): Promise<void> {
    if (!AUDIT_CONFIG.features.OTLP_EXPORT) return;

    const payload = {
      resourceSpans: [{
        resource: {
          attributes: [
            { key: 'service.name', value: { stringValue: 'sentinel-mcp' } },
          ],
        },
        scopeSpans: [{
          scope: { name: 'sentinel' },
          spans: [{
            traceId: this.traceId,
            spanId: `span_${Date.now()}`,
            name: spanName,
            startTimeUnixNano: Date.now() * 1_000_000,
            endTimeUnixNano: (Date.now() + duration) * 1_000_000,
            attributes: [
              { key: 'duration_ms', value: { intValue: duration } },
            ],
          }],
        }],
      }],
    };

    try {
      await fetch(AUDIT_CONFIG.otlp.endpoint, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          ...AUDIT_CONFIG.otlp.headers,
        },
        body: JSON.stringify(payload),
      }).catch(() => {
        // Silently fail if OTLP endpoint is not available
      });
    } catch (err) {
      console.error('[OTEL] Export failed:', err);
    }
  }
}
