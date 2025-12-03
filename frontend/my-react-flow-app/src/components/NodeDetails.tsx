import React from 'react';
import type { McpLog } from '../types';

interface NodeDetailsProps {
  event: McpLog;
  onClose: () => void;
}

export default function NodeDetails({ event, onClose }: NodeDetailsProps) {
  return (
    <div style={{ padding: '20px', height: '100%', background: '#1a1a1a' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '20px' }}>
        <h2 style={{ fontSize: '18px', fontWeight: 'bold' }}>Event Details</h2>
        <button
          onClick={onClose}
          style={{
            background: 'none',
            border: 'none',
            color: '#fff',
            cursor: 'pointer',
            fontSize: '20px',
          }}
        >
          ×
        </button>
      </div>

      <div style={{ marginBottom: '15px' }}>
        <div style={{ fontSize: '12px', color: '#888', marginBottom: '5px' }}>Timestamp</div>
        <div style={{ fontSize: '14px' }}>
          {new Date(event.timestamp).toLocaleString()}
        </div>
      </div>

      {event.method && (
        <div style={{ marginBottom: '15px' }}>
          <div style={{ fontSize: '12px', color: '#888', marginBottom: '5px' }}>Method</div>
          <div style={{ fontSize: '14px', fontFamily: 'monospace' }}>{event.method}</div>
        </div>
      )}

      {event.latency_ms !== undefined && (
        <div style={{ marginBottom: '15px' }}>
          <div style={{ fontSize: '12px', color: '#888', marginBottom: '5px' }}>Latency</div>
          <div style={{ fontSize: '14px' }}>{event.latency_ms} ms</div>
        </div>
      )}

      <div style={{ marginBottom: '15px' }}>
        <div style={{ fontSize: '12px', color: '#888', marginBottom: '5px' }}>Direction</div>
        <div style={{ fontSize: '14px' }}>{event.direction}</div>
      </div>

      <div style={{ marginBottom: '15px' }}>
        <div style={{ fontSize: '12px', color: '#888', marginBottom: '10px' }}>Payload</div>
        <pre
          style={{
            background: '#0a0a0a',
            padding: '15px',
            borderRadius: '4px',
            overflow: 'auto',
            fontSize: '12px',
            fontFamily: 'monospace',
            color: '#fff',
            maxHeight: 'calc(100vh - 300px)',
          }}
        >
          {JSON.stringify(event.payload, null, 2)}
        </pre>
      </div>
    </div>
  );
}

