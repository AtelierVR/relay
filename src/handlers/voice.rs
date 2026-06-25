/// Voice / Stream datagram handler.
///
/// Input format (after outer header is stripped by packet dispatcher):
///   [iid: u8][sub_type: u8][channel_id: u32][level_flags: u8][sample: bytes…]
///
/// Broadcast format (matches C# StreamEvent.FromBuffer):
///   [iid: u8][sub_type: u8][player_id: u16][channel_id: u32][level_flags: u8][sample: bytes…]
///
/// Sub-type 0x00 = Sample, 0x01 = Control.
use tracing::debug;

use crate::{
    handlers::packet::Packet,
    proto::{
        buffer::{PacketReader, PacketWriter},
        header::encode_datagram,
        packet_type::PacketType,
    },
};

pub async fn handle(packet: Packet) {
    let state = &packet.state;
    let client_id = packet.client_id();
    let payload = packet.payload.clone();
    let mut r = PacketReader::new(payload);

    let iid = r.read_u8();
    let inst_arc = match state.instances.get(iid) {
        Some(a) => a,
        None => {
            debug!("[Voice] unknown instance {}", iid);
            return;
        }
    };

    let self_player_id = {
        let inst = inst_arc.read();
        match inst.get_players().iter().find(|p| p.client_id == client_id) {
            Some(p) if p.is_ready() => p.id,
            _ => {
                debug!("[Voice] not-ready player from client {}", client_id);
                return;
            }
        }
    };

    // Parse StreamRequest format: [sub_type: u8][channel_id: u32][level_flags: u8][sample: bytes]
    let sub_type = r.read_u8();
    let channel_id = r.read_u32();
    let level_flags = r.read_u8();
    let remaining = r.remaining();
    let sample = r.read_bytes(remaining);

    // Build outbound datagram matching StreamEvent.FromBuffer() wire format:
    //   [iid: u8][sub_type: u8][player_id: u16][channel_id: u32][level_flags: u8][sample: bytes]
    let broadcast = {
        let mut w = PacketWriter::new();
        w.write_u8(iid);
        w.write_u8(sub_type);
        w.write_u16(self_player_id);
        w.write_u32(channel_id);
        w.write_u8(level_flags);
        w.write_bytes(&sample);
        encode_datagram(0, PacketType::Voice, w.finish().as_ref())
    };

    let recipients: Vec<u16> = {
        let inst = inst_arc.read();
        inst.get_players()
            .iter()
            .filter(|p| p.client_id != client_id)
            .map(|p| p.client_id)
            .collect()
    };

    for cid in recipients {
        if let Some(arc) = state.clients.get(cid) {
            arc.read().try_push(broadcast.clone());
        }
    }
}
