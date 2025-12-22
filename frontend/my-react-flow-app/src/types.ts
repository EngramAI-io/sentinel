export enum StreamDirection {
  Inbound = 'Inbound',
  Outbound = 'Outbound',
}

export interface McpLog {
  event_id: number;
  run_id: string;

  timestamp: number;
  direction: StreamDirection;
  method?: string;
  request_id?: number;
  latency_ms?: number;
  payload: any;

  session_id: string;
  trace_id: string;
  span_id: string;
  parent_span_id?: string;
}

