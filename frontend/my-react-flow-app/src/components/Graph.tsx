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
import { StreamDirection } from '../types';

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
  calls?: number;
  outbound?: number;
  inbound?: number;
  errors?: number;
  avgLatencyMs?: number;
};

type ClusterNodeData = {
  label: string;
  color: string;
};

type ClusterStats = {
  id: string;
  label: string;
  color: string;
  xSum: number;
  ySum: number;
  count: number;
};

type ToolStats = {
  total: number;
  outbound: number;
  inbound: number;
  errors: number;
  lastRequestId?: number;
  totalLatency: number;
  maxLatency: number;
};

// ---------- helpers ----------

function getClusterInfo(method: string): { id: string; label: string; color: string } {
  if (method.startsWith('postgres.') || method.startsWith('redis.')) {
    return { id: 'db', label: 'Databases', color: 'rgba(56,189,248,0.25)' }; // cyan-ish
  }
  if (method.startsWith('github.')) {
    return { id: 'github', label: 'GitHub', color: 'rgba(251,113,133,0.28)' }; // pink
  }
  if (method.startsWith('slack.')) {
    return { id: 'slack', label: 'Slack', color: 'rgba(74,222,128,0.25)' }; // green
  }
  if (method.startsWith('kubernetes.')) {
    return { id: 'k8s', label: 'Kubernetes', color: 'rgba(129,140,248,0.25)' }; // indigo
  }
  if (method.startsWith('llm.')) {
    return { id: 'llm', label: 'LLM Tools', color: 'rgba(250,204,21,0.28)' }; // yellow
  }
  if (method.startsWith('fs.')) {
    return { id: 'fs', label: 'Filesystem', color: 'rgba(148,163,184,0.25)' }; // slate
  }
  if (method.startsWith('browser.')) {
    return { id: 'browser', label: 'Browser', color: 'rgba(251,146,60,0.25)' }; // orange
  }
  if (method.startsWith('billing.')) {
    return { id: 'billing', label: 'Billing', color: 'rgba(244,114,182,0.25)' }; // fuchsia
  }
  if (method.startsWith('monitoring.')) {
    return { id: 'monitoring', label: 'Monitoring', color: 'rgba(45,212,191,0.25)' }; // teal
  }
  return { id: 'other', label: 'Other Tools', color: 'rgba(107,114,128,0.2)' };
}

function getToolIcon(method?: string): string {
  if (!method) return '🧩';
  if (method.startsWith('github.')) return '🐙';
  if (method.startsWith('slack.')) return '💬';
  if (method.startsWith('postgres.')) return '🐘';
  if (method.startsWith('redis.')) return '🔥';
  if (method.startsWith('kubernetes.')) return '☸️';
  if (method.startsWith('vector.')) return '🧠';
  if (method.startsWith('llm.')) return '🤖';
  if (method.startsWith('browser.')) return '🌐';
  if (method.startsWith('fs.')) return '📁';
  if (method.startsWith('billing.')) return '💳';
  if (method.startsWith('monitoring.')) return '📈';
  return '🧩';
}

function getLatencyColor(avgLatencyMs: number, hasError: boolean): string {
  if (hasError) return '#ef4444'; // red

  if (avgLatencyMs <= 50) return '#22c55e'; // fast: green
  if (avgLatencyMs <= 150) return '#eab308'; // ok: yellow
  if (avgLatencyMs <= 350) return '#f97316'; // slow: orange

  return '#dc2626'; // very slow, deep red
}

// ------------ Custom node renderers ------------

const AgentNode: React.FC<NodeProps> = (props) => {
  const data = props.data as CustomNodeData;

  return (
    <div
      style={{
        position: 'relative',
        padding: '12px 28px',
        background: '#6366f1',
        color: 'white',
        borderRadius: '999px',
        fontSize: '18px',
        fontWeight: 700,
        boxShadow: '0 10px 30px rgba(0, 0, 0, 0.6)',
        border: '2px solid rgba(255,255,255,0.35)',
      }}
    >
      {/* Source handles only – left & right */}
      <Handle
        id="left"
        type="source"
        position={Position.Left}
        style={{ background: '#fff', width: 8, height: 8, borderRadius: '50%' }}
      />
      <Handle
        id="right"
        type="source"
        position={Position.Right}
        style={{ background: '#fff', width: 8, height: 8, borderRadius: '50%' }}
      />

      {data.label}
    </div>
  );
};

const ToolNode: React.FC<NodeProps> = (props) => {
  const data = props.data as CustomNodeData;

  const baseColor = data.status === 'error' ? '#ef4444' : '#22c55e';
  const icon = getToolIcon(data.method);

  const isSelected =
    data.requestId !== undefined &&
    data.selectedId != null &&
    data.requestId.toString() === data.selectedId;

  const latencyLabel =
    typeof data.avgLatencyMs === 'number'
      ? `${Math.round(data.avgLatencyMs)}ms`
      : '—';

  return (
    <div
      style={{
        position: 'relative',
        padding: '10px 22px',
        background: baseColor,
        color: 'white',
        borderRadius: '999px',
        fontSize: '13px',
        fontWeight: 700,
        boxShadow: isSelected
          ? '0 0 22px rgba(255,255,255,0.65)'
          : '0 10px 26px rgba(0, 0, 0, 0.6)',
        border: isSelected ? '3px solid white' : 'none',
        transition: 'transform 0.15s ease, box-shadow 0.15s ease',
        transform: isSelected ? 'scale(1.08)' : 'scale(1)',
        display: 'flex',
        alignItems: 'center',
        gap: 10,
      }}
    >
      {/* Target handles only – left & right */}
      <Handle
        id="left"
        type="target"
        position={Position.Left}
        style={{ background: '#fff', width: 7, height: 7, borderRadius: '50%' }}
      />
      <Handle
        id="right"
        type="target"
        position={Position.Right}
        style={{ background: '#fff', width: 7, height: 7, borderRadius: '50%' }}
      />

      <span style={{ fontSize: 18 }}>{icon}</span>

      <div style={{ display: 'flex', flexDirection: 'column' }}>
        <div>{data.label}</div>
        {data.method && (
          <div style={{ fontSize: '11px', marginTop: 2, opacity: 0.9 }}>
            {data.method}
          </div>
        )}
        <div
          style={{
            fontSize: '10px',
            marginTop: 4,
            opacity: 0.9,
            display: 'flex',
            gap: 6,
            flexWrap: 'wrap',
          }}
        >
          <span>calls: {data.calls ?? 0}</span>
          <span>out: {data.outbound ?? 0}</span>
          <span>in: {data.inbound ?? 0}</span>
          {typeof data.errors === 'number' && data.errors > 0 && (
            <span style={{ color: '#fee2e2' }}>errors: {data.errors}</span>
          )}
          <span>lat: {latencyLabel}</span>
        </div>
      </div>
    </div>
  );
};

const ClusterNode: React.FC<NodeProps> = (props) => {
  const data = props.data as ClusterNodeData;

  return (
    <div
      style={{
        position: 'relative',
        width: 260,
        height: 260,
        borderRadius: '999px',
        background: data.color,
        filter: 'blur(40px)',
        opacity: 0.9,
      }}
    >
      <div
        style={{
          position: 'absolute',
          top: 14,
          left: 20,
          fontSize: 11,
          fontWeight: 600,
          color: 'rgba(226,232,240,0.95)',
          textTransform: 'uppercase',
          letterSpacing: 0.06,
        }}
      >
        {data.label}
      </div>
    </div>
  );
};

const nodeTypes: NodeTypes = {
  agent: AgentNode,
  tool: ToolNode,
  cluster: ClusterNode,
};

export default function Graph({ events, onNodeClick, selectedNode }: GraphProps) {
  const [nodes, setNodes, onNodesChange] = useNodesState<Node[]>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge[]>([]);

  useEffect(() => {
    const nodeMap = new Map<string, Node>();
    const edgeList: Edge[] = [];

    // ----- aggregate stats per method -----
    const statsMap = new Map<string, ToolStats>();

    events.forEach((e) => {
      if (!e.method || e.request_id == null) return;
      const method = e.method;

      const stats: ToolStats =
        statsMap.get(method) ?? {
          total: 0,
          outbound: 0,
          inbound: 0,
          errors: 0,
          lastRequestId: undefined,
          totalLatency: 0,
          maxLatency: 0,
        };

      stats.total += 1;
      stats.lastRequestId = e.request_id;

      if (e.direction === StreamDirection.Outbound) {
        stats.outbound += 1;
      }

      if (e.direction === StreamDirection.Inbound) {
        stats.inbound += 1;
        if (typeof e.latency_ms === 'number') {
          stats.totalLatency += e.latency_ms;
          stats.maxLatency = Math.max(stats.maxLatency, e.latency_ms);
        }
      }

      if ((e.payload as any)?.error) {
        stats.errors += 1;
      }

      statsMap.set(method, stats);
    });

    // ----- central agent node -----
    const centerX = 600;
    const centerY = 360;

    const agentNode: Node = {
      id: 'agent',
      type: 'agent',
      position: { x: centerX, y: centerY },
      data: { label: 'Agent', status: 'pending', selectedId: null } as CustomNodeData,
      draggable: false,
      style: { zIndex: 2 },
    };
    nodeMap.set('agent', agentNode);

    // ----- unique tool methods → radial layout -----
    const toolMethods = Array.from(statsMap.keys());
    const radius = 360; // large radius = “vast”

    const clusterMap = new Map<string, ClusterStats>();

    toolMethods.forEach((method, index) => {
      const angle = (2 * Math.PI * index) / toolMethods.length;
      const x = centerX + radius * Math.cos(angle);
      const y = centerY + radius * Math.sin(angle);

      const nodeId = `tool-${method}`;
      const stats = statsMap.get(method)!;

      const hasError = stats.errors > 0;
      const avgLatency =
        stats.inbound > 0 ? stats.totalLatency / stats.inbound : 0;

      const status: CustomNodeData['status'] = hasError ? 'error' : 'success';
      const shortLabel = method.split('.').pop() || method;
      const strokeColor = getLatencyColor(avgLatency, hasError);

      const toolNode: Node = {
        id: nodeId,
        type: 'tool',
        position: { x, y },
        data: {
          label: shortLabel,
          method,
          status,
          requestId: stats.lastRequestId,
          selectedId: selectedNode,
          calls: stats.total,
          outbound: stats.outbound,
          inbound: stats.inbound,
          errors: stats.errors,
          avgLatencyMs: avgLatency,
        } as CustomNodeData,
        style: { zIndex: 3 },
      };

      nodeMap.set(nodeId, toolNode);

      // cluster accumulation
      const clusterInfo = getClusterInfo(method);
      const cluster =
        clusterMap.get(clusterInfo.id) ??
        {
          id: clusterInfo.id,
          label: clusterInfo.label,
          color: clusterInfo.color,
          xSum: 0,
          ySum: 0,
          count: 0,
        };

      cluster.xSum += x;
      cluster.ySum += y;
      cluster.count += 1;
      clusterMap.set(clusterInfo.id, cluster);

      // choose handles based on where the tool sits relative to the agent
      const toolOnRight = x >= centerX;
      const agentHandleId = toolOnRight ? 'right' : 'left';
      const toolHandleId = toolOnRight ? 'left' : 'right';

      edgeList.push({
        id: `edge-${nodeId}`,
        source: 'agent',
        target: nodeId,
        sourceHandle: agentHandleId,
        targetHandle: toolHandleId,
        animated: true,
        type: 'smoothstep',
        style: {
          stroke: strokeColor,
          strokeWidth: hasError ? 3.2 : 2.4,
        },
      });
    });

    // ----- cluster background nodes (blurred blobs behind tools) -----
    clusterMap.forEach((cluster) => {
      if (cluster.count === 0) return;
      const cx = cluster.xSum / cluster.count;
      const cy = cluster.ySum / cluster.count;

      const clusterNode: Node = {
        id: `cluster-${cluster.id}`,
        type: 'cluster',
        position: { x: cx - 130, y: cy - 130 }, // center the blob around tools
        data: {
          label: cluster.label,
          color: cluster.color,
        } as ClusterNodeData,
        draggable: false,
        selectable: false,
        style: { zIndex: 1 },
      };

      nodeMap.set(clusterNode.id, clusterNode);
    });

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
        fitViewOptions={{ padding: 0.2 }}
      >
        <Background />
        <Controls />
      </ReactFlow>
    </div>
  );
}
