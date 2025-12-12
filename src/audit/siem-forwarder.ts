import { AUDIT_CONFIG } from './config';

export class SIEMForwarder {
  private queue: Record<string, any>[] = [];
  private batchSize: number = AUDIT_CONFIG.siem.batchSize;

  async forward(entry: Record<string, any>): Promise<void> {
    if (!AUDIT_CONFIG.features.SIEM_FORWARDING) return;

    this.queue.push(entry);
    if (this.queue.length >= this.batchSize) {
      await this._flushBatch();
    }
  }

  async flush(): Promise<void> {
    await this._flushBatch();
  }

  private async _flushBatch(): Promise<void> {
    if (this.queue.length === 0) return;

    const batch = [...this.queue];
    this.queue = [];

    try {
      switch (AUDIT_CONFIG.siem.type) {
        case 'splunk':
          await this._forwardToSplunk(batch);
          break;
        case 'elk':
          await this._forwardToELK(batch);
          break;
        case 'sentinel':
          await this._forwardToSentinel(batch);
          break;
        case 'chronicle':
          await this._forwardToChronicle(batch);
          break;
        default:
          console.warn('[SIEM] Unknown SIEM type:', AUDIT_CONFIG.siem.type);
      }
    } catch (err) {
      console.error('[SIEM] Forwarding failed:', err);
      // Re-queue for retry
      this.queue = [...batch, ...this.queue];
    }
  }

  private async _forwardToSplunk(entries: Record<string, any>[]): Promise<void> {
    if (!AUDIT_CONFIG.siem.endpoint || !AUDIT_CONFIG.siem.token) {
      console.warn('[SIEM] Splunk endpoint or token not configured');
      return;
    }

    const payload = entries.map(e => ({ event: JSON.stringify(e) }));

    const response = await fetch(`${AUDIT_CONFIG.siem.endpoint}/services/collector`, {
      method: 'POST',
      headers: {
        'Authorization': `Splunk ${AUDIT_CONFIG.siem.token}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ event: payload }),
    });

    if (!response.ok) {
      throw new Error(`Splunk forwarding failed: ${response.statusText}`);
    }
  }

  private async _forwardToELK(entries: Record<string, any>[]): Promise<void> {
    if (!AUDIT_CONFIG.siem.endpoint || !AUDIT_CONFIG.siem.token) {
      console.warn('[SIEM] ELK endpoint or token not configured');
      return;
    }

    const bulkPayload = entries
      .map(e => JSON.stringify({ index: { _id: e.correlation_id } }) + '\n' + JSON.stringify(e))
      .join('\n') + '\n';

    const response = await fetch(`${AUDIT_CONFIG.siem.endpoint}/_bulk`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${AUDIT_CONFIG.siem.token}`,
        'Content-Type': 'application/x-ndjson',
      },
      body: bulkPayload,
    });

    if (!response.ok) {
      throw new Error(`ELK forwarding failed: ${response.statusText}`);
    }
  }

  private async _forwardToSentinel(entries: Record<string, any>[]): Promise<void> {
    if (!AUDIT_CONFIG.siem.endpoint || !AUDIT_CONFIG.siem.token) {
      console.warn('[SIEM] Azure Sentinel endpoint or token not configured');
      return;
    }

    // Azure Sentinel (Microsoft) format
    const payload = entries.map(e => ({
      TimeGenerated: e.timestamp,
      Message: e.payload,
      EventType: e.event_type,
      UserId: e.user_id,
      CorrelationId: e.correlation_id,
      SessionId: e.session_id,
      AgentId: e.agent_id,
      Level: e.level,
      Classification: e.classification,
    }));

    const response = await fetch(AUDIT_CONFIG.siem.endpoint, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${AUDIT_CONFIG.siem.token}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(payload),
    });

    if (!response.ok) {
      throw new Error(`Sentinel forwarding failed: ${response.statusText}`);
    }
  }

  private async _forwardToChronicle(entries: Record<string, any>[]): Promise<void> {
    if (!AUDIT_CONFIG.siem.endpoint || !AUDIT_CONFIG.siem.token) {
      console.warn('[SIEM] Chronicle endpoint or token not configured');
      return;
    }

    // Google Chronicle format
    const payload = {
      entries: entries.map(e => ({
        timestamp: e.timestamp,
        log_data: JSON.stringify(e),
        metadata: {
          event_type: e.event_type,
          user_id: e.user_id,
          session_id: e.session_id,
          agent_id: e.agent_id,
          classification: e.classification,
        },
      })),
    };

    const response = await fetch(AUDIT_CONFIG.siem.endpoint, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${AUDIT_CONFIG.siem.token}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(payload),
    });

    if (!response.ok) {
      throw new Error(`Chronicle forwarding failed: ${response.statusText}`);
    }
  }
}
