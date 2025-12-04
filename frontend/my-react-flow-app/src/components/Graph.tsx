import React, { useEffect, useCallback } from 'react';
import {
  ReactFlow,
  Background,
  Controls,
  Handle,
  Position,
  type Node,
  type Edge,
  useNodesState,
  useEdgesState,
  type NodeTypes,
  type NodeProps,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import type { McpLog } from '../types';

interface GraphProps {
  events: McpLog[];
  onNodeClick: (nodeId: string | null) => void;
  selectedNode: string | null; // requestId as string
}

type CustomNodeData = {
  label: string;
  method?: string;
  status: 'pending' | 'success' | 'error';
  requestId?: number;
  selectedId?: string | null;
};

// ------------ Custom node renderers ------------

const AgentNode: React.FC<NodeProps> = (props) => {
  const data = props.data as CustomNodeData;

  return (
    <div
      style={{
        position: 'relative',
        padding: '10px 24px',
        background: '#6366f1',
        color: 'white',
        borderRadius: '999px',
        fontSize: '16px',
        fontWeight: 700,
        boxShadow: '0 6px 14px rgba(0, 0, 0, 0.35)',
        border: '2px solid rgba(255,255,255,0.35)',
      }}
    >
      {/* allow edges in and out of Agent */}
      <Handle
        type="source"
        position={Position.Right}
        style={{ background: '#fff', width: 8, height: 8, borderRadius: '50%' }}
      />
      <Handle
        type="target"
        position={Position.Left}
        style={{ background: '#fff', width: 8, height: 8, borderRadius: '50%' }}
      />
      {data.label}
    </div>
  );
};

const ToolNode: React.FC<NodeProps> = (props) => {
  const data = props.data as CustomNodeData;

  const colorMap: Record<'pending' | 'success' | 'error', string> = {
    pending: '#eab308',
    success: '#22c55e',
    error: '#ef4444',
  };

  const status = (data.status ?? 'pending') as keyof typeof colorMap;

  const isSelected =
    data.requestId !== undefined &&
    data.selectedId != null &&
    data.requestId.toString() === data.selectedId;

  return (
    <div
      style={{
        position: 'relative',
        padding: '10px 22px',
        background: colorMap[status],
        color: 'white',
        borderRadius: '999px',
        fontSize: '13px',
        fontWeight: 700,
        boxShadow: isSelected
          ? '0 0 18px rgba(255,255,255,0.45)'
          : '0 6px 14px rgba(0, 0, 0, 0.35)',
        border: isSelected ? '3px solid white' : 'none',
        transition: 'transform 0.15s ease, box-shadow 0.15s ease',
        transform: isSelected ? 'scale(1.07)' : 'scale(1)',
      }}
    >
      {/* let tools receive from Agent and potentially send back */}
      <Handle
        type="target"
        position={Position.Left}
        style={{ background: '#fff', width: 8, height: 8, borderRadius: '50%' }}
      />
      <Handle
        type="source"
        position={Position.Right}
        style={{ background: '#fff', width: 8, height: 8, borderRadius: '50%' }}
      />

      {data.label}
      {data.method && (
        <div style={{ fontSize: '11px', marginTop: 4, opacity: 0.9 }}>
          {data.method}
        </div>
      )}
    </div>
  );
};

const nodeTypes: NodeTypes = {
  agent: AgentNode,
  tool: ToolNode,
};

export default function Graph({ events, onNodeClick, selectedNode }: GraphProps) {
  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);

  useEffect(() => {
    const nodeMap = new Map<string, Node>();
    const edgeList: Edge[] = [];

    // ----- Central agent node -----
    const centerX = 400;
    const centerY = 300;

    const agentNode: Node = {
      id: 'agent',
      type: 'agent',
      position: { x: centerX, y: centerY },
      data: { label: 'Agent', status: 'pending', selectedId: null } as CustomNodeData,
      draggable: false,
    };
    nodeMap.set('agent', agentNode);

    // ----- Unique tool methods → radial layout -----
    const toolMethods = Array.from(
      new Set(
        events
          .filter((e) => e.method && e.request_id !== undefined)
          .map((e) => e.method as string),
      ),
    );

    const radius = 220;

    toolMethods.forEach((method, index) => {
      const angle = (2 * Math.PI * index) / toolMethods.length;
      const x = centerX + radius * Math.cos(angle);
      const y = centerY + radius * Math.sin(angle);

      const nodeId = `tool-${method}`;

      const toolNode: Node = {
        id: nodeId,
        type: 'tool',
        position: { x, y },
        data: {
          label: method.split('/').pop() || method,
          method,
          status: 'success',
          requestId: events.find((e) => e.method === method && e.request_id != null)
            ?.request_id,
          selectedId: selectedNode,
        } as CustomNodeData,
      };

      nodeMap.set(nodeId, toolNode);

      // Basic edge Agent -> Tool
      edgeList.push({
        id: `edge-${nodeId}`,
        source: 'agent',
        target: nodeId,
        animated: true,
        style: {
          stroke: '#ffffff',
          strokeWidth: 3,
        },
      });
    });

    console.log('Graph nodes:', Array.from(nodeMap.values()));
    console.log('Graph edges:', edgeList);

    setNodes(Array.from(nodeMap.values()));
    setEdges(edgeList);
  }, [events, selectedNode, setNodes, setEdges]);

  const onNodeClickHandler = useCallback(
    (_: React.MouseEvent, node: Node) => {
      const data = node.data as CustomNodeData;
      if (data?.requestId !== undefined) {
        onNodeClick(data.requestId.toString());
      } else {
        onNodeClick(null);
      }
    },
    [onNodeClick],
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
