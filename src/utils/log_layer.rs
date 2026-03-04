use parking_lot::Mutex;
use std::sync::Arc;
use std::time::SystemTime;
use tracing::{Level, Subscriber};
use tracing_subscriber::Layer;

use crate::utils::log_buffer::{LogBuffer, LogEntry};

pub type LogForwarder = Arc<Mutex<Option<tokio::sync::mpsc::UnboundedSender<LogEntry>>>>;

/// A tracing [`Layer`] that captures every log event into the shared [`LogBuffer`]
/// and optionally forwards each entry in real-time over an mpsc channel.
pub struct LogBufferLayer {
    buf: Arc<Mutex<LogBuffer>>,
    forwarder: LogForwarder,
}

impl LogBufferLayer {
    pub fn new(buf: Arc<Mutex<LogBuffer>>, forwarder: LogForwarder) -> Self {
        Self { buf, forwarder }
    }
}

impl<S: Subscriber> Layer<S> for LogBufferLayer {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let level = match *event.metadata().level() {
            Level::ERROR => "error",
            Level::WARN => "warning",
            Level::INFO => "log",
            Level::DEBUG => "debug",
            Level::TRACE => "debug", // Map trace to debug
        };

        let mut visitor = MsgVisitor::default();
        event.record(&mut visitor);

        // Build the message from all fields
        let mut message = if !visitor.fields.is_empty() {
            visitor.fields.join(" ")
        } else {
            format!("[{}]", event.metadata().target())
        };

        // Extract tag from [Tag] prefix if present
        let tag = extract_tag(&mut message);

        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        let entry = LogEntry {
            timestamp,
            level: level.to_string(),
            message,
            tag,
        };

        // Forward to WS in real-time if a session is active.
        if let Some(tx) = self.forwarder.lock().as_ref() {
            let _ = tx.send(entry.clone());
        }
        self.buf.lock().push(entry);
    }
}

#[derive(Default)]
struct MsgVisitor {
    fields: Vec<String>,
}

impl tracing::field::Visit for MsgVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.fields.push(value.to_string());
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.fields.push(format!("{:?}", value));
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        if field.name() != "message" {
            self.fields.push(format!("{}={}", field.name(), value));
        } else {
            self.fields.push(value.to_string());
        }
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        if field.name() != "message" {
            self.fields.push(format!("{}={}", field.name(), value));
        } else {
            self.fields.push(value.to_string());
        }
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        if field.name() != "message" {
            self.fields.push(format!("{}={}", field.name(), value));
        } else {
            self.fields.push(value.to_string());
        }
    }
}

/// Extract a tag from a message that starts with `[TagName]`.
/// Returns the tag and modifies the message to remove the prefix.
fn extract_tag(message: &mut String) -> Option<String> {
    if message.starts_with('[') {
        if let Some(end) = message.find(']') {
            if end < 30 {
                // Sanity check: tag shouldn't be too long
                let tag = message[1..end].to_string();
                // Remove "[Tag] " prefix (with space)
                let new_message = if message.len() > end + 1 && message.as_bytes()[end + 1] == b' '
                {
                    message[end + 2..].to_string()
                } else {
                    message[end + 1..].to_string()
                };
                *message = new_message;
                return Some(tag);
            }
        }
    }
    None
}
