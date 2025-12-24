import React, { useEffect, useCallback, useMemo } from 'react';
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
  BackgroundVariant,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import type { McpLog } from '../types';
import { StreamDirection } from '../types';

interface GraphProps {
  events: McpLog[];
  onNodeClick: (nodeId: string | null) => void;
  selectedNode: string | null;
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
  glowColor: string;
};

type ClusterStats = {
  id: string;
  label: string;
  color: string;
  glowColor: string;
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

// ============================================
// THEME CONSTANTS
// ============================================

const NEON_COLORS = {
  green: '#22c55e',
  greenGlow: 'rgba(34, 197, 94, 0.6)',
  red: '#ef4444',
  redGlow: 'rgba(239, 68, 68, 0.6)',
  purple: '#8b5cf6',
  purpleGlow: 'rgba(139, 92, 246, 0.6)',
  cyan: '#06b6d4',
  yellow: '#eab308',
  orange: '#f97316',
};

const BG_COLORS = {
  primary: '#0d1117',
  secondary: '#161b22',
  card: '#1c2128',
};

// ============================================
// HELPER FUNCTIONS
// ============================================

function getClusterInfo(method: string): { id: string; label: string; color: string; glowColor: string } {
  if (method.startsWith('postgres.') || method.startsWith('redis.')) {
    return { id: 'db', label: 'Databases', color: 'rgba(6, 182, 212, 0.15)', glowColor: NEON_COLORS.cyan };
  }
  if (method.startsWith('github.')) {
    return { id: 'github', label: 'GitHub', color: 'rgba(139, 92, 246, 0.15)', glowColor: NEON_COLORS.purple };
  }
  if (method.startsWith('slack.')) {
    return { id: 'slack', label: 'Slack', color: 'rgba(34, 197, 94, 0.12)', glowColor: NEON_COLORS.green };
  }
  if (method.startsWith('kubernetes.')) {
    return { id: 'k8s', label: 'Kubernetes', color: 'rgba(59, 130, 246, 0.15)', glowColor: '#3b82f6' };
  }
  if (method.startsWith('llm.')) {
    return { id: 'llm', label: 'LLM Tools', color: 'rgba(234, 179, 8, 0.15)', glowColor: NEON_COLORS.yellow };
  }
  if (method.startsWith('fs.')) {
    return { id: 'fs', label: 'Filesystem', color: 'rgba(148, 163, 184, 0.12)', glowColor: '#94a3b8' };
  }
  if (method.startsWith('browser.')) {
    return { id: 'browser', label: 'Browser', color: 'rgba(249, 115, 22, 0.15)', glowColor: NEON_COLORS.orange };
  }
  if (method.startsWith('billing.')) {
    return { id: 'billing', label: 'Billing', color: 'rgba(236, 72, 153, 0.15)', glowColor: '#ec4899' };
  }
  if (method.startsWith('monitoring.')) {
    return { id: 'monitoring', label: 'Monitoring', color: 'rgba(20, 184, 166, 0.15)', glowColor: '#14b8a6' };
  }
  return { id: 'other', label: 'Other Tools', color: 'rgba(107, 114, 128, 0.12)', glowColor: '#6b7280' };
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

// ============================================
// CUSTOM NODE COMPONENTS
// ============================================

const AgentNode: React.FC<NodeProps> = (props) => {
  const data = props.data as CustomNodeData;

  return (
    <div
      style={{
        position: 'relative',
        padding: '16px 32px',
        background: `linear-gradient(135deg, ${NEON_COLORS.purple} 0%, #7c3aed 100%)`,
        color: 'white',
        borderRadius: '999px',
        fontSize: '20px',
        fontWeight: 700,
        letterSpacing: '0.5px',
        border: `2px solid rgba(255, 255, 255, 0.3)`,
        boxShadow: `
          0 0 20px ${NEON_COLORS.purpleGlow},
          0 0 40px ${NEON_COLORS.purpleGlow},
          0 0 60px rgba(139, 92, 246, 0.3),
          inset 0 0 20px rgba(255, 255, 255, 0.1)
        `,
        animation: 'agent-pulse 3s ease-in-out infinite',
        textShadow: '0 0 10px rgba(255, 255, 255, 0.5)',
      }}
    >
      <Handle
        id="left"
        type="source"
        position={Position.Left}
        style={{
          background: NEON_COLORS.purple,
          width: 10,
          height: 10,
          borderRadius: '50%',
          border: '2px solid white',
          boxShadow: `0 0 8px ${NEON_COLORS.purple}`,
        }}
      />
      <Handle
        id="right"
        type="source"
        position={Position.Right}
        style={{
          background: NEON_COLORS.purple,
          width: 10,
          height: 10,
          borderRadius: '50%',
          border: '2px solid white',
          boxShadow: `0 0 8px ${NEON_COLORS.purple}`,
        }}
      />
      <Handle
        id="top"
        type="source"
        position={Position.Top}
        style={{
          background: NEON_COLORS.purple,
          width: 10,
          height: 10,
          borderRadius: '50%',
          border: '2px solid white',
          boxShadow: `0 0 8px ${NEON_COLORS.purple}`,
        }}
      />
      <Handle
        id="bottom"
        type="source"
        position={Position.Bottom}
        style={{
          background: NEON_COLORS.purple,
          width: 10,
          height: 10,
          borderRadius: '50%',
          border: '2px solid white',
          boxShadow: `0 0 8px ${NEON_COLORS.purple}`,
        }}
      />

      {data.label}
    </div>
  );
};

const ToolNode: React.FC<NodeProps> = (props) => {
  const data = props.data as CustomNodeData;

  const isError = data.status === 'error';
  const neonColor = isError ? NEON_COLORS.red : NEON_COLORS.green;
  const neonGlow = isError ? NEON_COLORS.redGlow : NEON_COLORS.greenGlow;
  const icon = getToolIcon(data.method);

  const isSelected =
    data.requestId !== undefined &&
    data.selectedId != null &&
    data.requestId.toString() === data.selectedId;

  const latencyLabel =
    typeof data.avgLatencyMs === 'number'
      ? `${Math.round(data.avgLatencyMs)}ms`
      : '—';

  const boxShadow = isSelected
    ? `
        0 0 20px ${neonColor},
        0 0 40px ${neonColor},
        0 0 60px ${neonGlow},
        0 0 80px ${neonGlow},
        inset 0 0 20px rgba(255, 255, 255, 0.15)
      `
    : `
        0 0 15px ${neonGlow},
        0 0 30px ${neonGlow},
        inset 0 0 15px rgba(255, 255, 255, 0.1)
      `;

  return (
    <div
      style={{
        position: 'relative',
        padding: '12px 20px',
        background: BG_COLORS.card,
        color: 'white',
        borderRadius: '12px',
        fontSize: '13px',
        fontWeight: 600,
        border: `2px solid ${neonColor}`,
        boxShadow,
        transition: 'all 0.2s ease',
        transform: isSelected ? 'scale(1.05)' : 'scale(1)',
        display: 'flex',
        alignItems: 'center',
        gap: 12,
        minWidth: 160,
      }}
    >
      <Handle
        id="left"
        type="target"
        position={Position.Left}
        style={{
          background: neonColor,
          width: 8,
          height: 8,
          borderRadius: '50%',
          border: '2px solid white',
          boxShadow: `0 0 6px ${neonColor}`,
        }}
      />
      <Handle
        id="right"
        type="target"
        position={Position.Right}
        style={{
          background: neonColor,
          width: 8,
          height: 8,
          borderRadius: '50%',
          border: '2px solid white',
          boxShadow: `0 0 6px ${neonColor}`,
        }}
      />
      <Handle
        id="top"
        type="target"
        position={Position.Top}
        style={{
          background: neonColor,
          width: 8,
          height: 8,
          borderRadius: '50%',
          border: '2px solid white',
          boxShadow: `0 0 6px ${neonColor}`,
        }}
      />
      <Handle
        id="bottom"
        type="target"
        position={Position.Bottom}
        style={{
          background: neonColor,
          width: 8,
          height: 8,
          borderRadius: '50%',
          border: '2px solid white',
          boxShadow: `0 0 6px ${neonColor}`,
        }}
      />

      {/* Icon with glow */}
      <div
        style={{
          fontSize: 24,
          filter: `drop-shadow(0 0 4px ${neonColor})`,
        }}
      >
        {icon}
      </div>

      {/* Content */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
        <div
          style={{
            color: neonColor,
            textShadow: `0 0 8px ${neonGlow}`,
            fontWeight: 700,
          }}
        >
          {data.label}
        </div>
        {data.method && (
          <div
            style={{
              fontSize: '10px',
              color: 'rgba(255, 255, 255, 0.6)',
              fontFamily: 'monospace',
            }}
          >
            {data.method}
          </div>
        )}
        <div
          style={{
            fontSize: '10px',
            color: 'rgba(255, 255, 255, 0.5)',
            display: 'flex',
            gap: 8,
            marginTop: 2,
          }}
        >
          <span>
            <span style={{ color: neonColor }}>{data.calls ?? 0}</span> calls
          </span>
          <span>
            <span style={{ color: '#06b6d4' }}>{latencyLabel}</span>
          </span>
          {typeof data.errors === 'number' && data.errors > 0 && (
            <span style={{ color: NEON_COLORS.red }}>
              {data.errors} err
            </span>
          )}
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
        width: 300,
        height: 300,
        borderRadius: '50%',
        background: `radial-gradient(circle, ${data.color} 0%, transparent 70%)`,
        filter: 'blur(30px)',
        opacity: 0.8,
        pointerEvents: 'none',
      }}
    >
      <div
        style={{
          position: 'absolute',
          top: '50%',
          left: '50%',
          transform: 'translate(-50%, -50%)',
          fontSize: 11,
          fontWeight: 600,
          color: data.glowColor,
          textTransform: 'uppercase',
          letterSpacing: 1,
          textShadow: `0 0 10px ${data.glowColor}`,
          whiteSpace: 'nowrap',
          filter: 'blur(0)',
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

// ============================================
// SVG FILTER DEFINITIONS FOR EDGE GLOW
// ============================================

const EdgeGlowFilters: React.FC = () => (
  <svg style={{ position: 'absolute', width: 0, height: 0 }}>
    <defs>
      <filter id="glow-green" x="-50%" y="-50%" width="200%" height="200%">
        <feGaussianBlur stdDeviation="3" result="coloredBlur" />
        <feMerge>
          <feMergeNode in="coloredBlur" />
          <feMergeNode in="SourceGraphic" />
        </feMerge>
      </filter>
      <filter id="glow-red" x="-50%" y="-50%" width="200%" height="200%">
        <feGaussianBlur stdDeviation="4" result="coloredBlur" />
        <feMerge>
          <feMergeNode in="coloredBlur" />
          <feMergeNode in="SourceGraphic" />
        </feMerge>
      </filter>
    </defs>
  </svg>
);

// ============================================
// MAIN GRAPH COMPONENT
// ============================================

export default function Graph({ events, onNodeClick, selectedNode }: GraphProps) {
  const [nodes, setNodes, onNodesChange] = useNodesState<Node[]>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge[]>([]);

  // Compute aggregate stats
  const statsMap = useMemo(() => {
    const map = new Map<string, ToolStats>();

    events.forEach((e) => {
      if (!e.method || e.request_id == null) return;
      const method = e.method;

      const stats: ToolStats = map.get(method) ?? {
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

      map.set(method, stats);
    });

    return map;
  }, [events]);

  useEffect(() => {
    const nodeMap = new Map<string, Node>();
    const edgeList: Edge[] = [];

    const centerX = 600;
    const centerY = 400;

    // Central Agent Node
    const agentNode: Node = {
      id: 'agent',
      type: 'agent',
      position: { x: centerX, y: centerY },
      data: { label: 'Agent', status: 'pending', selectedId: null } as CustomNodeData,
      draggable: false,
      style: { zIndex: 10 },
    };
    nodeMap.set('agent', agentNode);

    // Tool nodes in radial layout
    const toolMethods = Array.from(statsMap.keys());
    const radius = 320;
    const clusterMap = new Map<string, ClusterStats>();

    toolMethods.forEach((method, index) => {
      const angle = (2 * Math.PI * index) / toolMethods.length - Math.PI / 2;
      const x = centerX + radius * Math.cos(angle);
      const y = centerY + radius * Math.sin(angle);

      const nodeId = `tool-${method}`;
      const stats = statsMap.get(method)!;

      const hasError = stats.errors > 0;
      const avgLatency = stats.inbound > 0 ? stats.totalLatency / stats.inbound : 0;

      const status: CustomNodeData['status'] = hasError ? 'error' : 'success';
      const shortLabel = method.split('.').pop() || method;

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
        style: { zIndex: 5 },
      };

      nodeMap.set(nodeId, toolNode);

      // Cluster accumulation
      const clusterInfo = getClusterInfo(method);
      const cluster = clusterMap.get(clusterInfo.id) ?? {
        id: clusterInfo.id,
        label: clusterInfo.label,
        color: clusterInfo.color,
        glowColor: clusterInfo.glowColor,
        xSum: 0,
        ySum: 0,
        count: 0,
      };

      cluster.xSum += x;
      cluster.ySum += y;
      cluster.count += 1;
      clusterMap.set(clusterInfo.id, cluster);

      // Determine handle positions based on angle
      const normalizedAngle = ((angle % (2 * Math.PI)) + 2 * Math.PI) % (2 * Math.PI);
      let agentHandle: string;
      let toolHandle: string;

      if (normalizedAngle >= 0 && normalizedAngle < Math.PI / 4) {
        agentHandle = 'right';
        toolHandle = 'left';
      } else if (normalizedAngle >= Math.PI / 4 && normalizedAngle < (3 * Math.PI) / 4) {
        agentHandle = 'bottom';
        toolHandle = 'top';
      } else if (normalizedAngle >= (3 * Math.PI) / 4 && normalizedAngle < (5 * Math.PI) / 4) {
        agentHandle = 'left';
        toolHandle = 'right';
      } else if (normalizedAngle >= (5 * Math.PI) / 4 && normalizedAngle < (7 * Math.PI) / 4) {
        agentHandle = 'top';
        toolHandle = 'bottom';
      } else {
        agentHandle = 'right';
        toolHandle = 'left';
      }

      // Edge with neon styling
      const edgeColor = hasError ? NEON_COLORS.red : NEON_COLORS.green;
      const edgeClass = hasError ? 'edge-error' : 'edge-success';

      edgeList.push({
        id: `edge-${nodeId}`,
        source: 'agent',
        target: nodeId,
        sourceHandle: agentHandle,
        targetHandle: toolHandle,
        type: 'smoothstep',
        className: edgeClass,
        style: {
          stroke: edgeColor,
          strokeWidth: hasError ? 3 : 2.5,
          filter: `drop-shadow(0 0 3px ${edgeColor}) drop-shadow(0 0 6px ${edgeColor})`,
        },
      });
    });

    // Cluster background nodes
    clusterMap.forEach((cluster) => {
      if (cluster.count === 0) return;
      const cx = cluster.xSum / cluster.count;
      const cy = cluster.ySum / cluster.count;

      const clusterNode: Node = {
        id: `cluster-${cluster.id}`,
        type: 'cluster',
        position: { x: cx - 150, y: cy - 150 },
        data: {
          label: cluster.label,
          color: cluster.color,
          glowColor: cluster.glowColor,
        } as ClusterNodeData,
        draggable: false,
        selectable: false,
        style: { zIndex: 1 },
      };

      nodeMap.set(clusterNode.id, clusterNode);
    });

    setNodes(Array.from(nodeMap.values()));
    setEdges(edgeList);
  }, [statsMap, selectedNode, setNodes, setEdges]);

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
    <div style={{ width: '100%', height: '100%', background: BG_COLORS.primary }}>
      <EdgeGlowFilters />
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onNodeClick={onNodeClickHandler}
        nodeTypes={nodeTypes}
        fitView
        fitViewOptions={{ padding: 0.3 }}
        proOptions={{ hideAttribution: true }}
      >
        <Background
          variant={BackgroundVariant.Dots}
          gap={20}
          size={1}
          color="rgba(255, 255, 255, 0.05)"
        />
        <Controls
          style={{
            background: BG_COLORS.secondary,
            borderRadius: 8,
            border: '1px solid #30363d',
          }}
        />
      </ReactFlow>
    </div>
  );
}
