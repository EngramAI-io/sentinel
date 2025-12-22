use crate::protocol::JsonRpcMessage;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StreamDirection {
    Inbound,  // From child stdout (response)
    Outbound, // From parent stdin (request)
}

/// Raw bytes as observed by Sentinel (no ordering decided here).
#[derive(Debug, Clone)]
pub struct RawTap {
    pub direction: StreamDirection,
    pub bytes: Bytes,
    pub observed_ts_ms: u64,
}

/// Canonical, ordered tap event (ordering decided by the sequencer).
#[derive(Debug, Clone)]
pub struct TapEvent {
    pub event_id: u64,
    pub direction: StreamDirection,
    pub bytes: Bytes,
    pub observed_ts_ms: u64,
}

pub fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpLog {

    /// Identifier for this run of Sentinel
    pub run_id: String,
    
    /// Canonical ordering assigned by Sentinel
    pub event_id: u64,

    /// When Sentinel observed the bytes (source-of-truth for ordering)
    pub observed_ts_ms: u64,

    /// When the structured log was emitted (may be slightly later)
    pub timestamp: u64,

    pub direction: StreamDirection,
    pub method: Option<String>,
    pub request_id: Option<u64>,
    pub latency_ms: Option<u64>,
    pub payload: serde_json::Value,

    // Trace fields
    pub session_id: String,
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
}

impl McpLog {
    pub fn from_message(
        run_id: String,
        event_id: u64,
        observed_ts_ms: u64,
        direction: StreamDirection,
        message: JsonRpcMessage,
        latency_ms: Option<u64>,
        session_id: &str,
        trace_id: &str,
        span_id: String,
        parent_span_id: Option<String>,
    ) -> Self {
        let timestamp = current_timestamp_ms();

        let (method, request_id) = match &message {
            JsonRpcMessage::Request(req) => (Some(req.method.clone()), req.id),
            JsonRpcMessage::Response(resp) => (None, resp.id),
        };

        let payload = match &message {
            JsonRpcMessage::Request(req) => serde_json::to_value(req).unwrap_or_default(),
            JsonRpcMessage::Response(resp) => serde_json::to_value(resp).unwrap_or_default(),
        };

        Self {
            run_id, 
            event_id,
            observed_ts_ms,
            timestamp,
            direction,
            method,
            request_id,
            latency_ms,
            payload,
            session_id: session_id.to_string(),
            trace_id: trace_id.to_string(),
            span_id,
            parent_span_id,
        }
    }
}
