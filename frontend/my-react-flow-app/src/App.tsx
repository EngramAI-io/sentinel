import React, { useState } from 'react';
import Graph from './components/Graph';
import NodeDetails from './components/NodeDetails';
import { useWebSocket } from './hooks/useWebSocket';
// import type { McpLog } from './types';

function App() {
  const [selectedNode, setSelectedNode] = useState<string | null>(null);
  const events = useWebSocket('ws://localhost:3000/ws');

  const selectedEvent = selectedNode
    ? events.find((e) => e.request_id?.toString() === selectedNode)
    : null;

  return (
    <div style={{ display: 'flex', height: '100vh', width: '100vw' }}>
      <div style={{ flex: 1, position: 'relative' }}>
        <Graph events={events} onNodeClick={setSelectedNode} selectedNode={selectedNode} />
      </div>
      {selectedEvent && (
        <div style={{ width: '400px', borderLeft: '1px solid #333', overflowY: 'auto' }}>
          <NodeDetails event={selectedEvent} onClose={() => setSelectedNode(null)} />
        </div>
      )}
    </div>
  );
}

export default App;

