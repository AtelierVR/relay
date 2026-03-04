/// ```packet
/// [Length: u16][UID: u16][Type: u8 = 0x00]
/// [reason: string?]  (optional, present when payload has > 2 bytes remaining)
/// ```
/// Guard: must be handshaked.
/// Leaves all instances and closes the connection gracefully.
use bytes::Bytes;
use tracing::{debug, info};

use crate::{
    handlers::{context::AppState, quit::leave_all_instances},
    proto::buffer::PacketReader,
    utils::hex_fmt,
};

pub fn handle(state: &AppState, client_id: u16, _uid: u16, payload: Bytes) -> Bytes {
    debug!(
        "[Disconnect] client {}: raw payload: {}",
        client_id,
        hex_fmt::fmt_bytes(payload.as_ref(), 32)
    );
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
    let reason_str = if r.remaining() > 2 {
        r.read_string()
    } else {
        None
    };

    // Log disconnect with user info
    let user_info = if let Some(arc) = state.clients.get(client_id) {
        let c = arc.read();
        if let Some(u) = &c.user {
            format!("{}@{} (\"{}\")", u.id, u.address, u.display_name)
        } else {
            String::from("(not authenticated)")
        }
    } else {
        String::from("(unknown)")
    };

    if let Some(ref reason) = reason_str {
        info!(
            "[Disconnect] client {} disconnected | user={} reason={}",
            client_id, user_info, reason
        );
    } else {
        info!(
            "[Disconnect] client {} disconnected | user={}",
            client_id, user_info
        );
    }

    // Leave all instances (broadcasts Leave to remaining ready players).
    leave_all_instances(state, client_id);

    Bytes::new()
}
