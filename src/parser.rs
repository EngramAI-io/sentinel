use crate::events::{McpLog, StreamDirection};
use crate::protocol::JsonRpcMessage;
use crate::session::Session;

use bytes::Bytes;
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Parser converts raw tapped bytes into structured MCP logs
pub struct Parser {
    log_tx: mpsc::Sender<McpLog>,
    session: Arc<Session>,

    /// request_id -> (span_id, start_time)
    pending_spans: HashMap<u64, (String, Instant)>,
}

impl Parser {
    pub fn new(
        log_tx: mpsc::Sender<McpLog>,
        session: Arc<Session>,
    ) -> Self {
        Self {
            log_tx,
            session,
            pending_spans: HashMap::new(),
        }
    }

    pub async fn process_stream(
        mut self,
        mut tap_rx: mpsc::Receiver<(StreamDirection, Bytes)>,
    ) -> anyhow::Result<()> {
        while let Some((direction, bytes)) = tap_rx.recv().await {
            let message: JsonRpcMessage = match serde_json::from_slice(&bytes) {
                Ok(m) => m,
                Err(_) => continue, // Ignore non-JSON
            };

            match (&direction, &message) {
                // ----------------------------
                // Outbound REQUEST
                // ----------------------------
                (StreamDirection::Outbound, JsonRpcMessage::Request(req)) => {
                    let span_id = Uuid::new_v4().to_string();
                    let start = Instant::now();

                    if let Some(request_id) = req.id {
                        self.pending_spans.insert(request_id, (span_id.clone(), start));
                    }

                    let log = McpLog {
                        timestamp: current_timestamp(),
                        direction,
                        method: Some(req.method.clone()),
                        request_id: req.id,
                        latency_ms: None,
                        payload: serde_json::to_value(req).unwrap_or_default(),

                        session_id: self.session.session_id.clone(),
                        trace_id: self.session.trace_id.clone(),
                        span_id,
                        parent_span_id: None,
                    };

                    let _ = self.log_tx.send(log).await;
                }

                // ----------------------------
                // Inbound RESPONSE
                // ----------------------------
                (StreamDirection::Inbound, JsonRpcMessage::Response(resp)) => {
                    let (span_id, latency_ms) = match resp.id {
                        Some(id) => {
                            if let Some((span_id, start)) = self.pending_spans.remove(&id) {
                                let latency =
                                    start.elapsed().as_millis() as u64;
                                (span_id, Some(latency))
                            } else {
                                (Uuid::new_v4().to_string(), None)
                            }
                        }
                        None => (Uuid::new_v4().to_string(), None),
                    };

                    let log = McpLog {
                        timestamp: current_timestamp(),
                        direction,
                        method: None,
                        request_id: resp.id,
                        latency_ms,
                        payload: serde_json::to_value(resp).unwrap_or_default(),

                        session_id: self.session.session_id.clone(),
                        trace_id: self.session.trace_id.clone(),
                        span_id: span_id.clone(),
                        parent_span_id: Some(span_id),
                    };

                    let _ = self.log_tx.send(log).await;
                }

                _ => {}
            }
        }

        Ok(())
    }
}

fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
