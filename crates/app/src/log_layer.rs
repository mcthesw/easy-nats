//! In-memory tracing layer that captures log events into a ring buffer for the UI.

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

const RING_CAPACITY: usize = 1000;

/// Shared handle to the in-memory log ring buffer.
pub type SharedLogBuffer = Arc<Mutex<VecDeque<LogEntry>>>;

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
        let mut visitor = MessageVisitor(String::new());
        event.record(&mut visitor);

        let entry = LogEntry {
            level: *event.metadata().level(),
            target: event.metadata().target().to_string(),
            message: visitor.0,
            timestamp: std::time::SystemTime::now(),
        };

        if let Ok(mut buf) = self.buffer.lock() {
            if buf.len() >= RING_CAPACITY {
                buf.pop_front();
            }
            buf.push_back(entry);
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
