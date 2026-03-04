/// ```packet
/// [Length: u16][UID: u16][Type: u8 = 0x04]
/// [ClientTimestamp: i64]   ← Unix ms sent by the client
/// [ClientLong: i64]        ← opaque long sent by the client
/// ```
/// Response: `[ClientTimestamp: i64][ServerTimestamp: i64][ClientLong: i64]`
use bytes::Bytes;

use crate::{
    handlers::context::AppState,
    proto::{
        buffer::{PacketReader, PacketWriter},
        header::encode_stream_packet,
        packet_type::PacketType,
    },
};

pub fn handle(state: &AppState, client_id: u16, uid: u16, payload: Bytes) -> Bytes {
    // Reject if not handshaked.
    if let Some(arc) = state.clients.get(client_id) {
        if !arc.read().is_handshaked() {
            return Bytes::new();
        }
    } else {
        return Bytes::new();
    }

    let mut r = PacketReader::new(payload);
    let client_ts = r.read_i64(); // client's DateTime (Unix ms)
    let client_long = r.read_i64(); // opaque client long

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    let mut w = PacketWriter::new();
    w.write_i64(client_ts); // echo client timestamp back
    w.write_i64(now_ms); // server timestamp
    w.write_i64(client_long); // echo client long back
    encode_stream_packet(uid, PacketType::Latency, w.finish().as_ref())
}
