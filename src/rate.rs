//! Per-client packet rate limiting.
//!
//! A player is expected to send its traffic at the instance TPS (transforms, properties, events,
//! voice, …). A modified client can ignore that contract, so the relay meters what it receives and
//! simply **drops** the excess instead of disconnecting: dropping is what the unreliable channels
//! already do, it never punishes a client for a latency spike, and a legitimate client loses nothing
//! (the next tick carries a fresher value).
//!
//! The metering is fully generic: a handler computes a **bucket key** for the packet it is about to
//! handle (see [`key`]) and checks it against the TPS. The key is a hash of a few payload byte
//! groups, which decides the granularity:
//!
//! - `key(packet, &[])` → one budget per packet type: the "once per tick" traffic.
//! - `key(packet, &[(offset, len), …])` → one budget per value(s) of those bytes, i.e. per avatar
//!   part for a `Transform`, per channel for a `Stream`, per entity for a `Properties`, …
//!
//! Buckets are token buckets refilled with the elapsed time, capped at `rate * burst_ms / 1000 + 1`.
//! That capacity is the jitter margin: a client that was stalled (GC, lag spike, packet loss) can
//! catch up with a short burst without being throttled, while a client that constantly sends more
//! than its budget runs out of tokens and gets its extra packets dropped.

use std::collections::HashMap;
use std::time::Instant;

use tracing::debug;

use crate::client::client::Client;
use crate::constants::DEFAULT_TPS;
use crate::handlers::{context::AppState, packet::Packet};

/// Highest number of buckets tracked per client.
///
/// Keys are derived from what the client sends, so the map has to be bounded: without it a modified
/// client would create an unbounded number of buckets by naming random entities/parts/paths.
pub const MAX_BUCKETS: usize = 512;

/// FNV-1a offset basis.
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
/// FNV-1a prime.
const FNV_PRIME: u64 = 1_099_511_628_211;

/// Bucket key of a packet: a hash of its type and of the given payload byte groups.
///
/// Each group is a `(offset, length)` range of the payload, so a handler can pick exactly the fields
/// that identify its traffic and skip the others (room, sub-type, entity, part, channel, event name,
/// path, …). Two packets with the same values in those ranges land in the same bucket while the
/// hashing itself stays generic.
///
/// - `&[]` → one budget for the whole packet type (the "once per tick" traffic).
/// - `&[(0, 2)]` → one budget per room, whatever the rest of the payload.
pub fn key(packet: &Packet, groups: &[(usize, usize)]) -> u64 {
    let mut hash = FNV_OFFSET ^ u64::from(packet.packet_type as u8);
    let len = packet.payload.len();

    for &(offset, count) in groups {
        let start = offset.min(len);
        let end = start.saturating_add(count).min(len);

        for byte in &packet.payload[start..end] {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
    }

    hash
}

/// A token bucket: `rate` packets per second, refilled from the elapsed time.
#[derive(Debug, Clone, Copy, Default)]
pub struct TokenBucket {
    tokens: f32,
    last: Option<Instant>,
}

impl TokenBucket {
    /// Refills the bucket with `rate` tokens per second (capped at `capacity`), then consumes one
    /// token. Returns `false` when the sender is over budget.
    ///
    /// A fresh bucket starts **full**, so the first packet of a new key is never dropped.
    pub fn allow(&mut self, now: Instant, rate: f32, capacity: f32) -> bool {
        let capacity = capacity.max(1.0);

        self.tokens = match self.last {
            Some(last) => (self.tokens + (now - last).as_secs_f32() * rate).min(capacity),
            None => capacity,
        };
        self.last = Some(now);

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Forgets the accumulated state (the budget follows a new room/TPS).
    pub fn reset(&mut self) {
        self.last = None;
    }
}

/// The metered budgets of a client, plus the drop counters shown by `clients <id>`.
#[derive(Debug, Default)]
pub struct RateLimits {
    buckets: HashMap<u64, TokenBucket>,
    /// Packets dropped because they exceeded their budget.
    pub dropped: u64,
    /// When the last over-budget warning was logged (throttles the logs, not the packets).
    pub last_warning: Option<Instant>,
}

impl RateLimits {
    /// Charges one packet against the bucket `key`. Returns `false` when it must be dropped.
    #[inline]
    pub fn allow(&mut self, key: u64, rate: f32, config: &crate::config::RateLimitConfig) -> bool {
        if !config.enabled {
            return true;
        }

        let rate = rate.max(1.0);
        let capacity = burst(rate, config);
        let now = Instant::now();

        // Fast path: one lookup for a known key, no allocation.
        if let Some(bucket) = self.buckets.get_mut(&key) {
            if bucket.allow(now, rate, capacity) {
                return true;
            }

            self.dropped += 1;
            return false;
        }

        // A client naming random ids must not be able to allocate buckets forever.
        if self.buckets.len() >= MAX_BUCKETS {
            self.dropped += 1;
            return false;
        }

        let mut bucket = TokenBucket::default();
        let allowed = bucket.allow(now, rate, capacity);
        self.buckets.insert(key, bucket);

        if !allowed {
            self.dropped += 1;
        }

        allowed
    }

    /// Forgets the accumulated tokens of every budget.
    pub fn reset(&mut self) {
        self.buckets.clear();
    }
}

/// Bucket capacity: one packet plus the burst window, i.e. how much a stalled client may catch up.
fn burst(rate: f32, config: &crate::config::RateLimitConfig) -> f32 {
    rate * (f32::from(config.burst_ms) / 1000.0) + 1.0
}

/// Meters a packet against the TPS of the client's room. Returns `false` when it must be dropped.
///
/// Called by the handlers themselves (they are the only ones that know how their payload is
/// identified), and as early as possible: a dropped packet then costs a single client lock and one
/// hash-map lookup, no room/player scan.
pub fn check(state: &AppState, client_id: u16, key: u64) -> bool {
    let config = &state.config.rate_limit;
    if !config.enabled {
        return true;
    }

    let Some(arc) = state.clients.get(client_id) else {
        return true;
    };

    let mut client = arc.write();

    // Resolve the budget from the room while the client is already locked (one lock, not three).
    let rate = client
        .room
        .and_then(|iid| state.instances.get(iid))
        .map(|inst| inst.read().get_effective_tps(&state.config.load_balancing))
        .unwrap_or(DEFAULT_TPS);

    if client.limits.allow(key, f32::from(rate.max(1)), config) {
        return true;
    }

    warn_over_budget(&mut client, client_id);
    false
}

/// Logs an over-budget packet, at most once per second per client: the log is throttled, not the
/// packets (an abusive client would otherwise flood the console).
fn warn_over_budget(client: &mut Client, client_id: u16) {
    let now = Instant::now();
    let warn = match client.limits.last_warning {
        Some(last) if now.duration_since(last).as_secs() < 1 => false,
        _ => {
            client.limits.last_warning = Some(now);
            true
        }
    };

    if warn {
        let dropped = client.limits.dropped;
        debug!("[RateLimit] client {client_id} over budget, {dropped} dropped");
    }
}

/// Resets every budget of a client (room change: the TPS it is allowed changes).
pub fn reset(client: &mut Client) {
    client.limits.reset();
}
