/// ```packet
/// [Length: u16][UID: u16][Type: u8 = 0x04]
/// [ClientTimestamp: i64]   ← Unix ms sent by the client
/// [ClientLong: i64]        ← opaque long sent by the client
/// ```
/// Response: `[ClientTimestamp: i64][ServerTimestamp: i64][ClientLong: i64]`
use crate::{
    handlers::packet::Packet,
    proto::{
        buffer::{PacketReader, PacketWriter},
        header::encode_stream_packet,
        packet_type::PacketType,
    },
};

pub fn handle(mut packet: Packet) {
    let client_id = packet.client_id();
    let uid = packet.uid;
    let payload = packet.payload.clone();

    // Reject if not handshaked.
    if let Some(arc) = packet.state.clients.get(client_id) {
        if !arc.read().is_handshaked() {
            return;
        }
    } else {
        return;
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
    packet.reply_raw(encode_stream_packet(
        uid,
        PacketType::Latency,
        w.finish().as_ref(),
    ));
}
