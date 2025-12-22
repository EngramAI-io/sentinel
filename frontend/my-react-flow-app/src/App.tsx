import React, { useState } from 'react';
import Graph from './components/Graph';
import EventTimeline from './components/EventTimeline';
import NodeDetails from './components/NodeDetails';
import { useWebSocket } from './hooks/useWebSocket';
import type { McpLog } from './types';

type ViewMode = 'tools' | 'timeline';

function App() {
  const [selectedNode, setSelectedNode] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<ViewMode>('tools');

  const events: McpLog[] = useWebSocket('ws://localhost:3000/ws');

  const selectedEvent =
    selectedNode != null
      ? events.find((e) => e.request_id?.toString() === selectedNode) ?? null
      : null;

  return (
    <div style={{ display: 'flex', height: '100vh', width: '100vw' }}>
      {/* Main graph area */}
      <div style={{ flex: 1, position: 'relative' }}>
        {/* Top control bar */}
        <div
          style={{
            position: 'absolute',
            top: 12,
            left: 12,
            zIndex: 20,
            display: 'flex',
            gap: 8,
          }}
        >
          <button
            onClick={() =>
              setViewMode((m) => (m === 'tools' ? 'timeline' : 'tools'))
            }
            style={{
              padding: '6px 12px',
              fontSize: 12,
              fontWeight: 600,
              background: '#111',
              color: '#fff',
              border: '1px solid #333',
              borderRadius: 6,
              cursor: 'pointer',
            }}
          >
            {viewMode === 'tools' ? 'Timeline View' : 'Tool View'}
          </button>

          {viewMode === 'timeline' && (
            <span
              style={{
                fontSize: 11,
                color: '#aaa',
                alignSelf: 'center',
              }}
            >
              ordered by event_id • span edges = causality
            </span>
          )}
        </div>

        {/* Graph content */}
        {viewMode === 'tools' ? (
          <Graph
            events={events}
            onNodeClick={setSelectedNode}
            selectedNode={selectedNode}
          />
        ) : (
          <EventTimeline events={events} />
        )}
      </div>

      {/* Right-side details panel */}
      {selectedEvent && viewMode === 'tools' && (
        <div
          style={{
            width: 400,
            borderLeft: '1px solid #333',
            overflowY: 'auto',
          }}
        >
          <NodeDetails
            event={selectedEvent}
            onClose={() => setSelectedNode(null)}
          />
        </div>
      )}
    </div>
  );
}

export default App;
