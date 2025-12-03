import React, { useEffect, useCallback } from 'react';
import {
  ReactFlow,
  Background,
  Controls,
  type Node,
  type Edge,
  useNodesState,
  useEdgesState,
  type NodeTypes,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import type { McpLog, StreamDirection } from '../types';

interface GraphProps {
  events: McpLog[];
  onNodeClick: (nodeId: string | null) => void;
  selectedNode: string | null;
}

interface CustomNodeData {
  label: string;
  method?: string;
  status: 'pending' | 'success' | 'error';
  requestId?: number;
}

const nodeTypes: NodeTypes = {
  agent: ({ data }) => (
    <div
      style={{
        padding: '10px 20px',
        background: '#6366f1',
        color: 'white',
        borderRadius: '8px',
        fontSize: '14px',
        fontWeight: 'bold',
        boxShadow: '0 4px 6px rgba(0, 0, 0, 0.3)',
      }}
    >
      {data.label}
    </div>
  ),
  tool: ({ data }) => {
    const colorMap = {
      pending: '#eab308',
      success: '#22c55e',
      error: '#ef4444',
    };
    return (
      <div
        style={{
          padding: '10px 20px',
          background: colorMap[data.status],
          color: 'white',
          borderRadius: '8px',
          fontSize: '12px',
          fontWeight: 'bold',
          boxShadow: '0 4px 6px rgba(0, 0, 0, 0.3)',
          border: data.requestId?.toString() === data.selectedId ? '3px solid white' : 'none',
        }}
      >
        {data.label}
        {data.method && <div style={{ fontSize: '10px', marginTop: '4px', opacity: 0.9 }}>{data.method}</div>}
      </div>
    );
  },
};

export default function Graph({ events, onNodeClick, selectedNode }: GraphProps) {
  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);

  useEffect(() => {
    const nodeMap = new Map<string, Node>();
    const edgeMap = new Map<string, Edge>();
    
    // Create agent node (central)
    const agentNode: Node<CustomNodeData> = {
      id: 'agent',
      type: 'agent',
      position: { x: 400, y: 300 },
      data: { label: 'Agent', status: 'pending' },
      draggable: false,
    };
    nodeMap.set('agent', agentNode);

    // Process events
    events.forEach((event) => {
      if (event.method && event.request_id) {
        const nodeId = `tool-${event.method}`;
        const requestId = event.request_id.toString();

        // Create or update tool node
        if (!nodeMap.has(nodeId)) {
          const toolNode: Node<CustomNodeData> = {
            id: nodeId,
            type: 'tool',
            position: {
              x: 200 + Math.random() * 600,
              y: 100 + Math.random() * 400,
            },
            data: {
              label: event.method.split('/').pop() || event.method,
              method: event.method,
              status: event.direction === StreamDirection.Inbound && event.latency_ms !== undefined
                ? (event.payload.error ? 'error' : 'success')
                : 'pending',
              requestId: event.request_id,
              selectedId: selectedNode,
            },
          };
          nodeMap.set(nodeId, toolNode);
        }

        // Update node status based on response
        if (event.direction === StreamDirection.Inbound && event.latency_ms !== undefined) {
          const node = nodeMap.get(nodeId);
          if (node) {
            const hasError = event.payload.error !== undefined;
            const updatedNode: Node<CustomNodeData> = {
              ...node,
              data: {
                ...node.data,
                status: hasError ? 'error' : 'success',
                selectedId: selectedNode,
              },
            };
            nodeMap.set(nodeId, updatedNode);
          }
        }

        // Create edge from agent to tool
        const edgeId = `edge-${requestId}`;
        if (!edgeMap.has(edgeId)) {
          const edge: Edge = {
            id: edgeId,
            source: 'agent',
            target: nodeId,
            animated: event.direction === StreamDirection.Outbound,
            style: {
              stroke: event.direction === StreamDirection.Inbound ? '#22c55e' : '#eab308',
              strokeWidth: 2,
            },
          };
          edgeMap.set(edgeId, edge);
        }
      }
    });

    setNodes(Array.from(nodeMap.values()));
    setEdges(Array.from(edgeMap.values()));
  }, [events, selectedNode, setNodes, setEdges]);

  const onNodeClickHandler = useCallback(
    (_: React.MouseEvent, node: Node) => {
      const data = node.data as CustomNodeData;
      if (data.requestId) {
        onNodeClick(data.requestId.toString());
      }
    },
    [onNodeClick]
  );

  return (
    <div style={{ width: '100%', height: '100%' }}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onNodeClick={onNodeClickHandler}
        nodeTypes={nodeTypes}
        fitView
      >
        <Background />
        <Controls />
      </ReactFlow>
    </div>
  );
}

