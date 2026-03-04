use crate::handlers::context::AppState;
/// Teleport handler — stub (no C# implementation yet).
use bytes::Bytes;

pub async fn handle(_state: &AppState, _client_id: u16, _uid: u16, _payload: Bytes) -> Bytes {
    Bytes::new()
}
