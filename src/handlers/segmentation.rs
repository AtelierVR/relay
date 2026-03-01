/// No-op: QUIC streams are unbounded — segmentation is not needed.
/// Packet is accepted and silently discarded.
use bytes::Bytes;
use crate::handlers::context::AppState;

pub fn handle(_state: &AppState, _client_id: u16, _uid: u16, _payload: Bytes) -> Bytes {
    Bytes::new()
}
