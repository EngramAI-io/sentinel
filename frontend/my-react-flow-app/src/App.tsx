import React, { useState } from 'react';
import Graph from './components/Graph';
import NodeDetails from './components/NodeDetails';
import { useWebSocket } from './hooks/useWebSocket';
import type { McpLog } from './types';

function App() {
  const [selectedNode, setSelectedNode] = useState<string | null>(null);

  const events: McpLog[] = useWebSocket('ws://localhost:3000/ws');

  const selectedEvent =
    selectedNode != null
      ? events.find((e) => e.request_id?.toString() === selectedNode) ?? null
      : null;

  return (
    <div style={{ display: 'flex', height: '100vh', width: '100vw' }}>
      {/* Main graph area */}
      <div style={{ flex: 1, position: 'relative' }}>
        <Graph
          events={events}
          onNodeClick={setSelectedNode}
          selectedNode={selectedNode}
        />
      </div>

      {/* Right-side details panel */}
      {selectedEvent && (
        <div
          style={{
            width: 400,
            borderLeft: '1px solid #30363d',
            overflowY: 'auto',
            background: '#0d1117',
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
