/// Voice datagram handler — broadcasts raw audio samples to all other players.
///
/// Datagram: [iid: u8][player_id: u16][sample: bytes(remaining)]
/// Broadcast: [iid: u8][player_id: u16][sample: bytes]
use bytes::Bytes;
use tracing::debug;

use crate::{
    handlers::context::AppState,
    proto::{
        buffer::{PacketReader, PacketWriter},
        header::encode_datagram,
        packet_type::PacketType,
    },
};

pub async fn handle(state: &AppState, client_id: u16, _uid: u16, payload: Bytes) {
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

    let remaining = r.remaining();
    if remaining == 0 || !remaining.is_multiple_of(2) {
        debug!(
            "[Voice] invalid sample length {} from client {}",
            remaining, client_id
        );
        return;
    }

    let sample = r.read_bytes(remaining);

    // Build outbound datagram.
    let broadcast = {
        let mut w = PacketWriter::new();
        w.write_u8(iid);
        w.write_u16(self_player_id);
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
