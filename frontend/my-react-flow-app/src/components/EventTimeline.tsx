import React, { useMemo } from 'react';
import {
  ReactFlow,
  Background,
  Controls,
  type Node,
  type Edge,
} from '@xyflow/react';
import type { McpLog } from '../types';
import { StreamDirection } from '../types';

export default function EventTimeline({ events }: { events: McpLog[] }) {
  const { nodes, edges } = useMemo(() => {
    const sorted = [...events].sort((a, b) => a.event_id - b.event_id);

    const nodes: Node[] = [];
    const edges: Edge[] = [];

    const spanMap = new Map<string, string>(); // span_id → nodeId

    sorted.forEach((e, i) => {
      const nodeId = `evt-${e.event_id}`;

      nodes.push({
        id: nodeId,
        position: { x: i * 160, y: e.direction === StreamDirection.Outbound ? 100 : 300 },
        data: {
          label: `${e.event_id}: ${e.direction}`,
        },
        style: {
          padding: 10,
          borderRadius: 8,
          background: e.direction === StreamDirection.Outbound ? '#2563eb' : '#16a34a',
          color: 'white',
        },
      });

      // timeline edge
      if (i > 0) {
        edges.push({
          id: `timeline-${i}`,
          source: `evt-${sorted[i - 1].event_id}`,
          target: nodeId,
          animated: true,
          style: { stroke: '#888' },
        });
      }

      // span edge
      if (spanMap.has(e.span_id)) {
        edges.push({
          id: `span-${e.span_id}`,
          source: spanMap.get(e.span_id)!,
          target: nodeId,
          type: 'smoothstep',
          style: { stroke: '#facc15', strokeDasharray: '4 2' },
        });
      } else {
        spanMap.set(e.span_id, nodeId);
      }
    });

    return { nodes, edges };
  }, [events]);

  return (
    <div style={{ width: '100%', height: '100%' }}>
      <ReactFlow nodes={nodes} edges={edges} fitView>
        <Background />
        <Controls />
      </ReactFlow>
    </div>
  );
}
