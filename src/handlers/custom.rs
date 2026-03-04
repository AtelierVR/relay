use crate::handlers::context::AppState;
/// Custom packet handler — stub for user-defined packet types.
use bytes::Bytes;

pub async fn handle(_state: &AppState, _client_id: u16, _uid: u16, _payload: Bytes) -> Bytes {
    Bytes::new()
}
