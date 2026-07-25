//! In-memory tracing layer that captures log events into a ring buffer for the UI.
#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use tracing::Level;
use tracing_subscriber::Layer;

/// A single captured log entry.
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub level: Level,
    pub target: String,
    pub message: String,
    pub timestamp: std::time::SystemTime,
}

const RING_CAPACITY: usize = 2000;

/// Crate targets that are allowed at all log levels (TRACE and above).
const OWN_TARGETS: &[&str] = &["easy_nats", "nats_backend"];

/// Ring buffer that evicts the oldest entry when full.
pub struct LogBuffer {
    entries: VecDeque<LogEntry>,
    capacity: usize,
}

impl LogBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, entry: LogEntry) {
        if self.entries.len() >= self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn entries(&self) -> &VecDeque<LogEntry> {
        &self.entries
    }
}

impl Default for LogBuffer {
    fn default() -> Self {
        Self::new(RING_CAPACITY)
    }
}

/// Shared handle to the in-memory log ring buffer.
pub type SharedLogBuffer = Arc<Mutex<LogBuffer>>;

/// A `tracing_subscriber::Layer` that writes events to the shared buffer.
pub struct AppLogLayer {
    buffer: SharedLogBuffer,
}

impl AppLogLayer {
    pub fn new(buffer: SharedLogBuffer) -> Self {
        Self { buffer }
    }
}

impl<S> Layer<S> for AppLogLayer
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let meta = event.metadata();

        // Third-party targets are capped at INFO; only own crates get DEBUG/TRACE.
        if *meta.level() > Level::INFO {
            let target = meta.target();
            if !OWN_TARGETS.iter().any(|t| target.starts_with(t)) {
                return;
            }
        }

        let mut visitor = MessageVisitor(String::new());
        event.record(&mut visitor);

        let entry = LogEntry {
            level: *meta.level(),
            target: meta.target().to_string(),
            message: visitor.0,
            timestamp: std::time::SystemTime::now(),
        };

        if let Ok(mut buf) = self.buffer.lock() {
            buf.push(entry);
        }
    }
}

struct MessageVisitor(String);

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.0 = format!("{value:?}");
        } else if self.0.is_empty() {
            self.0 = format!("{} = {value:?}", field.name());
        } else {
            self.0 = format!("{}, {} = {value:?}", self.0, field.name());
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.0 = value.to_string();
        } else if self.0.is_empty() {
            self.0 = format!("{} = {value}", field.name());
        } else {
            self.0 = format!("{}, {} = {value}", self.0, field.name());
        }
    }
}
