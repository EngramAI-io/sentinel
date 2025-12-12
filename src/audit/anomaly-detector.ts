import { AUDIT_CONFIG } from './config';

export interface AnomalyEvent {
  type: 'error_rate' | 'auth_failure' | 'rate_limit' | 'unusual_pattern';
  severity: 'low' | 'medium' | 'high' | 'critical';
  message: string;
  timestamp: string;
  metadata: Record<string, any>;
}

export class AnomalyDetector {
  private errorCount: number = 0;
  private authFailureCount: number = 0;
  private requestCount: number = 0;
  private windowStart: number = Date.now();
  private windowSize: number = 60000; // 1 minute window

  private onAnomalyCallback?: (event: AnomalyEvent) => void;

  constructor() {
    if (!AUDIT_CONFIG.features.ANOMALY_DETECTION) {
      return;
    }
  }

  setAnomalyCallback(callback: (event: AnomalyEvent) => void): void {
    this.onAnomalyCallback = callback;
  }

  recordRequest(success: boolean, eventType?: string): void {
    if (!AUDIT_CONFIG.features.ANOMALY_DETECTION) return;

    this.requestCount++;
    if (!success) {
      this.errorCount++;
    }

    // Check if we need to reset the window
    const now = Date.now();
    if (now - this.windowStart > this.windowSize) {
      this._resetWindow();
    }

    // Check error rate threshold
    if (this.requestCount > 0) {
      const errorRate = (this.errorCount / this.requestCount) * 100;
      if (errorRate > AUDIT_CONFIG.anomaly.errorRateThreshold) {
        this._triggerAnomaly({
          type: 'error_rate',
          severity: errorRate > 80 ? 'critical' : errorRate > 60 ? 'high' : 'medium',
          message: `Error rate exceeded threshold: ${errorRate.toFixed(2)}%`,
          timestamp: new Date().toISOString(),
          metadata: {
            errorRate,
            errorCount: this.errorCount,
            requestCount: this.requestCount,
          },
        });
      }
    }

    // Check rate limit
    if (this.requestCount > AUDIT_CONFIG.anomaly.rateLimit) {
      this._triggerAnomaly({
        type: 'rate_limit',
        severity: 'high',
        message: `Rate limit exceeded: ${this.requestCount} requests in window`,
        timestamp: new Date().toISOString(),
        metadata: {
          requestCount: this.requestCount,
          windowSize: this.windowSize,
        },
      });
    }
  }

  recordAuthFailure(userId?: string): void {
    if (!AUDIT_CONFIG.features.ANOMALY_DETECTION) return;

    this.authFailureCount++;
    if (this.authFailureCount >= AUDIT_CONFIG.anomaly.authFailureLimit) {
      this._triggerAnomaly({
        type: 'auth_failure',
        severity: 'critical',
        message: `Multiple authentication failures detected: ${this.authFailureCount}`,
        timestamp: new Date().toISOString(),
        metadata: {
          failureCount: this.authFailureCount,
          userId,
        },
      });
    }
  }

  private _resetWindow(): void {
    this.errorCount = 0;
    this.authFailureCount = 0;
    this.requestCount = 0;
    this.windowStart = Date.now();
  }

  private _triggerAnomaly(event: AnomalyEvent): void {
    if (this.onAnomalyCallback) {
      this.onAnomalyCallback(event);
    } else {
      console.warn('[ANOMALY]', JSON.stringify(event));
    }
  }

  getStats(): {
    errorCount: number;
    authFailureCount: number;
    requestCount: number;
    errorRate: number;
  } {
    const errorRate = this.requestCount > 0 
      ? (this.errorCount / this.requestCount) * 100 
      : 0;

    return {
      errorCount: this.errorCount,
      authFailureCount: this.authFailureCount,
      requestCount: this.requestCount,
      errorRate,
    };
  }
}
