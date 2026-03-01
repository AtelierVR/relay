/// ```packet
/// [Length: u16][UID: u16][Type: u8 = 0x00]
/// [reason: string?]  (optional, present when payload has > 2 bytes remaining)
/// ```
/// Guard: must be handshaked.
/// Leaves all instances and closes the connection gracefully.
use bytes::Bytes;
use tracing::debug;

use crate::{
    handlers::{context::AppState, quit::leave_all_instances},
    proto::buffer::PacketReader,
};

pub fn handle(state: &AppState, client_id: u16, _uid: u16, payload: Bytes) -> Bytes {
    // Guard: must have completed handshake.
    let is_handshaked = state
        .clients
        .get(client_id)
        .map(|a| a.read().is_handshaked())
        .unwrap_or(false);
    if !is_handshaked {
        return Bytes::new();
    }

    // Optional reason string (Remaining() > 2 in C# = at least u16 length prefix + 1 byte).
    let mut r = PacketReader::new(payload);
    if r.remaining() > 2 {
        if let Some(reason) = r.read_string() {
            debug!("[Disconnect] client {client_id} reason: {reason}");
        }
    }

    // Leave all instances (broadcasts Leave to remaining ready players).
    leave_all_instances(state, client_id);

    Bytes::new()
}
