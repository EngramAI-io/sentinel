use crate::events::{McpLog, StreamDirection, TapEvent};
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
    run_id: String,
    session: Arc<Session>,
    log_tx: mpsc::Sender<McpLog>,
    

    /// request_id -> (span_id, start_time)
    pending_spans: HashMap<u64, (String, Instant)>,
}

impl Parser {
    pub fn new(
        run_id: String,
        log_tx: mpsc::Sender<McpLog>,
        session: Arc<Session>,
    ) -> Self {
        Self {
            run_id,
            session,
            log_tx,
            pending_spans: HashMap::new(),
        }
    }
    
    pub async fn process_stream(
        mut self,
        mut tap_rx: mpsc::Receiver<(TapEvent)>,
    ) -> anyhow::Result<()> {
        let mut expected_id = 1u64;
            while let Some(evt) = tap_rx.recv().await {
                if evt.event_id != expected_id {
                    eprintln!(
                        "⚠️  Warning: Missing event IDs. Expected {}, got {}",
                        expected_id, evt.event_id
                    );
                }
                expected_id = evt.event_id + 1;
                let direction = evt.direction;
                let bytes = evt.bytes.clone();

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

                let log = McpLog::from_message(
                    self.run_id.clone(),
                    evt.event_id,
                    evt.observed_ts_ms,
                    direction,
                    message,
                    None,
                    &self.session.session_id,
                    &self.session.trace_id,
                    span_id,
                    None, // parent_span_id (leave None unless you later model nesting)
                );

                    let _ = self.log_tx.send(log).await;
                }

                // ----------------------------
                // Inbound RESPONSE
                // ----------------------------
                (StreamDirection::Inbound, JsonRpcMessage::Response(resp)) => {
                let (span_id, latency_ms) = if let Some(id) = resp.id {
                    if let Some((span, start)) = self.pending_spans.remove(&id) {
                        (span, Some(start.elapsed().as_millis() as u64))
                    } else {
                        (Uuid::new_v4().to_string(), None)
                    }
                } else {
                    (Uuid::new_v4().to_string(), None)
                };

                let log = McpLog::from_message(
                    self.run_id.clone(),
                    evt.event_id,
                    evt.observed_ts_ms,
                    direction,
                    message,
                    latency_ms,
                    &self.session.session_id,
                    &self.session.trace_id,
                    span_id,
                    None, // IMPORTANT: response is not its own parent
                );

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
