use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// A single log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Unix millisecond timestamp.
    pub timestamp: i64,
    /// Log level string: "error", "warning", "log", "debug".
    pub level: String,
    /// The log message.
    pub message: String,
    /// Optional tag (e.g., "Auth", "Crypto") extracted from [Tag] prefix.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}

/// Bounded ring-buffer of recent log entries.
/// Thread-safety is provided by the caller (wrap in `Mutex<LogBuffer>`).
#[derive(Debug)]
pub struct LogBuffer {
    entries: VecDeque<LogEntry>,
    max: usize,
}

impl LogBuffer {
    pub fn new(max: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(max),
            max,
        }
    }

    /// Push a new entry, evicting the oldest if the buffer is full.
    pub fn push(&mut self, entry: LogEntry) {
        if self.entries.len() >= self.max {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    /// Return all entries with `timestamp >= since_ms` in chronological order.
    pub fn since(&self, since_ms: i64) -> Vec<LogEntry> {
        self.entries
            .iter()
            .filter(|e| e.timestamp >= since_ms)
            .cloned()
            .collect()
    }

    /// Return the last `n` entries in chronological order.
    pub fn last_n(&self, n: usize) -> Vec<LogEntry> {
        let start = self.entries.len().saturating_sub(n);
        self.entries.range(start..).cloned().collect()
    }

    /// Return all entries.
    pub fn all(&self) -> Vec<LogEntry> {
        self.entries.iter().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
