import type { McpLog } from '../types';
import { StreamDirection } from '../types';

interface NodeDetailsProps {
  event: McpLog;
  onClose: () => void;
}

const COLORS = {
  bgPrimary: '#0d1117',
  bgSecondary: '#161b22',
  bgCard: '#1c2128',
  border: '#30363d',
  textPrimary: '#f0f6fc',
  textSecondary: '#8b949e',
  neonGreen: '#22c55e',
  neonRed: '#ef4444',
  neonPurple: '#8b5cf6',
  neonCyan: '#06b6d4',
};

export default function NodeDetails({ event, onClose }: NodeDetailsProps) {
  const isError = (event.payload as any)?.error;
  const accentColor = isError ? COLORS.neonRed : COLORS.neonGreen;

  return (
    <div
      style={{
        padding: '20px',
        height: '100%',
        background: COLORS.bgSecondary,
      }}
    >
      {/* Header */}
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          marginBottom: '24px',
          paddingBottom: '16px',
          borderBottom: `1px solid ${COLORS.border}`,
        }}
      >
        <h2
          style={{
            fontSize: '16px',
            fontWeight: 700,
            color: COLORS.neonPurple,
            textShadow: `0 0 10px rgba(139, 92, 246, 0.5)`,
            margin: 0,
          }}
        >
          Event Details
        </h2>
        <button
          onClick={onClose}
          style={{
            background: COLORS.bgCard,
            border: `1px solid ${COLORS.border}`,
            borderRadius: '6px',
            color: COLORS.textSecondary,
            cursor: 'pointer',
            fontSize: '16px',
            width: '28px',
            height: '28px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            transition: 'all 0.2s ease',
          }}
        >
          ×
        </button>
      </div>

      {/* Timestamp */}
      <div style={{ marginBottom: '16px' }}>
        <div
          style={{
            fontSize: '11px',
            color: COLORS.textSecondary,
            marginBottom: '4px',
            textTransform: 'uppercase',
            letterSpacing: '0.5px',
          }}
        >
          Timestamp
        </div>
        <div style={{ fontSize: '14px', color: COLORS.textPrimary }}>
          {new Date(event.timestamp).toLocaleString()}
        </div>
      </div>

      {/* Method */}
      {event.method && (
        <div style={{ marginBottom: '16px' }}>
          <div
            style={{
              fontSize: '11px',
              color: COLORS.textSecondary,
              marginBottom: '4px',
              textTransform: 'uppercase',
              letterSpacing: '0.5px',
            }}
          >
            Method
          </div>
          <div
            style={{
              fontSize: '14px',
              fontFamily: 'monospace',
              color: accentColor,
              textShadow: `0 0 8px ${accentColor}40`,
            }}
          >
            {event.method}
          </div>
        </div>
      )}

      {/* Latency */}
      {event.latency_ms !== undefined && (
        <div style={{ marginBottom: '16px' }}>
          <div
            style={{
              fontSize: '11px',
              color: COLORS.textSecondary,
              marginBottom: '4px',
              textTransform: 'uppercase',
              letterSpacing: '0.5px',
            }}
          >
            Latency
          </div>
          <div
            style={{
              fontSize: '14px',
              color: COLORS.neonCyan,
              textShadow: `0 0 8px ${COLORS.neonCyan}40`,
            }}
          >
            {event.latency_ms} ms
          </div>
        </div>
      )}

      {/* Direction */}
      <div style={{ marginBottom: '16px' }}>
        <div
          style={{
            fontSize: '11px',
            color: COLORS.textSecondary,
            marginBottom: '4px',
            textTransform: 'uppercase',
            letterSpacing: '0.5px',
          }}
        >
          Direction
        </div>
        <div
          style={{
            fontSize: '14px',
            color:
              event.direction === StreamDirection.Outbound
                ? '#eab308'
                : COLORS.neonGreen,
          }}
        >
          {event.direction}
        </div>
      </div>

      {/* Payload */}
      <div style={{ marginBottom: '16px' }}>
        <div
          style={{
            fontSize: '11px',
            color: COLORS.textSecondary,
            marginBottom: '8px',
            textTransform: 'uppercase',
            letterSpacing: '0.5px',
          }}
        >
          Payload
        </div>
        <pre
          style={{
            background: COLORS.bgPrimary,
            padding: '16px',
            borderRadius: '8px',
            border: `1px solid ${COLORS.border}`,
            overflow: 'auto',
            fontSize: '12px',
            fontFamily: 'monospace',
            color: COLORS.textPrimary,
            maxHeight: 'calc(100vh - 380px)',
            margin: 0,
            lineHeight: 1.5,
          }}
        >
          {JSON.stringify(event.payload, null, 2)}
        </pre>
      </div>
    </div>
  );
}
