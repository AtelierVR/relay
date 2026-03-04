/// Event handler — name-tagged binary broadcast with optional targeting.
///
/// Request:  [iid: u8][name: i64][data_len: u16][data: bytes]
///           (?[target_count: u8][target_ids: u16 × count])
/// Broadcast: [iid][player_id: u16][name: i64][data_len: u16][data: bytes]
use bytes::Bytes;
use tracing::debug;

use crate::{
    handlers::context::AppState,
    proto::{
        buffer::{PacketReader, PacketWriter},
        header::encode_stream_packet,
        packet_type::PacketType,
    },
};

pub async fn handle(state: &AppState, client_id: u16, uid: u16, payload: Bytes) {
    let mut r = PacketReader::new(payload);

    let iid = r.read_u8();
    let inst_arc = match state.instances.get(iid) {
        Some(a) => a,
        None => {
            debug!("[Event] unknown instance {}", iid);
            return;
        }
    };

    let self_player_id = {
        let inst = inst_arc.read();
        match inst.get_players().iter().find(|p| p.client_id == client_id) {
            Some(p) if p.is_ready() => p.id,
            _ => {
                debug!("[Event] not-ready player from client {}", client_id);
                return;
            }
        }
    };

    let name = r.read_i64();
    let data_len = r.read_u16();
    if data_len > 1024 {
        debug!(
            "[Event] data too large ({} bytes) from client {}",
            data_len, client_id
        );
        return;
    }
    let event_data = r.read_bytes(data_len as usize).to_vec();

    // Resolve targets.
    let target_cids: Vec<u16> = if r.remaining() > 0 {
        let count = r.read_u8();
        let mut targets = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let target_pid = r.read_u16();
            let inst = inst_arc.read();
            if let Some(p) = inst
                .get_players()
                .iter()
                .find(|p| p.id == target_pid && p.is_ready())
            {
                targets.push(p.client_id);
            }
        }
        targets
    } else {
        let inst = inst_arc.read();
        inst.get_players()
            .iter()
            .filter(|p| p.id != self_player_id && p.is_ready())
            .map(|p| p.client_id)
            .collect()
    };

    let broadcast = {
        let mut w = PacketWriter::new();
        w.write_u8(iid);
        w.write_u16(self_player_id);
        w.write_i64(name);
        w.write_u16(event_data.len() as u16);
        w.write_bytes(&event_data);
        encode_stream_packet(0, PacketType::Event, w.finish().as_ref())
    };

    for cid in target_cids {
        if let Some(arc) = state.clients.get(cid) {
            arc.read().try_push(broadcast.clone());
        }
    }
}
