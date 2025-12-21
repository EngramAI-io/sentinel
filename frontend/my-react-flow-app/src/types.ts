export enum StreamDirection {
  Inbound = 'Inbound',
  Outbound = 'Outbound',
}

export interface McpLog {
  timestamp: number;
  direction: StreamDirection;
  method?: string;
  request_id?: number;
  latency_ms?: number;
  payload: any;
  session_id: String;
  trace_id: String;
  span_id: String;
  parent_span_id?: String;
}

