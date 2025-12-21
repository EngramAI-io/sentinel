use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use uuid::Uuid;


pub struct Session {
    pub session_id: String,
    pub trace_id: String,
}

pub struct SessionState {
    pending_requests: HashMap<u64, Instant>,
}


pub struct SessionTracker {
    pending: HashMap<u64, (u64, String)>, // request_id -> (timestamp, span_id)
}

impl SessionTracker {
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
        }
    }

    pub fn start_span(&mut self, request_id: u64) -> String {
        let span_id = Uuid::new_v4().to_string();

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        self.pending.insert(request_id, (timestamp, span_id.clone()));
        span_id
    }

    pub fn end_span(&mut self, request_id: u64) -> Option<(u64, String)> {
        self.pending.remove(&request_id)
    }

}

impl SessionState {
    pub fn new() -> Self {
        Self {
            pending_requests: HashMap::new(),
        }
    }

    pub fn record_request(&mut self, request_id: u64) {
        self.pending_requests.insert(request_id, Instant::now());
    }

    /// Called when we see a response; returns latency in ms if we know the request.
    pub fn complete_request(&mut self, request_id: u64) -> Option<u64> {
        if let Some(start) = self.pending_requests.remove(&request_id) {
            let dur = start.elapsed();
            Some(dur.as_secs() * 1000 + dur.subsec_millis() as u64)
        } else {
            None
        }
    }

    /// Drop old requests older than `max_age_seconds` to avoid unbounded growth.
    pub fn clear_old_requests(&mut self, max_age_seconds: u64) {
        let cutoff = Instant::now() - Duration::from_secs(max_age_seconds);
        self.pending_requests.retain(|_, t| *t > cutoff);
    }
}

impl Default for SessionState {
    fn default() -> Self {
        Self::new()
    }
}
