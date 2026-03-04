/// Properties handler — broadcasts a per-player dictionary of key/value byte blobs.
///
/// Request:  [iid: u8][pid: u16][count: u8] then count×([key: i32][len: u8][bytes: len])
///   count=0 → no-op (echo nothing, just broadcast empty dict to all)
/// Broadcast: [iid][eId(op)][bId(self)][count][...entries]
use bytes::Bytes;
use std::collections::HashMap;
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
            debug!("[Properties] unknown instance {}", iid);
            return;
        }
    };

    let self_player_id = {
        let inst = inst_arc.read();
        match inst.get_players().iter().find(|p| p.client_id == client_id) {
            Some(p) if p.is_ready() => p.id,
            _ => {
                debug!("[Properties] not-ready player from client {}", client_id);
                return;
            }
        }
    };

    let pid = r.read_u16();

    // Resolve op player.
    let op_player_id = if pid != u16::MAX && pid != self_player_id {
        let inst = inst_arc.read();
        match inst.get_players().iter().find(|p| p.id == pid) {
            Some(p) if p.is_ready() => {
                let self_p = inst
                    .get_players()
                    .iter()
                    .find(|p| p.id == self_player_id)
                    .unwrap();
                if !self_p.has_privilege() {
                    debug!("[Properties] unprivileged operation on player {}", pid);
                    return;
                }
                p.id
            }
            _ => {
                debug!("[Properties] op-player {} not ready", pid);
                return;
            }
        }
    } else {
        self_player_id
    };

    let count = r.read_u8();

    // C#: if count==0, echo empty props back to the caller only and return.
    if count == 0 {
        let packet = build_properties(iid, op_player_id, self_player_id, &HashMap::new());
        if let Some(arc) = state.clients.get(client_id) {
            arc.read().try_push(packet);
        }
        return;
    }

    let mut dict: HashMap<i32, Vec<u8>> = HashMap::new();
    for _ in 0..count {
        let key = r.read_i32();
        let len = r.read_u8();
        let val = r.read_bytes(len as usize).to_vec();
        dict.insert(key, val);
    }

    // Broadcast to all other players.
    let broadcast = build_properties(iid, op_player_id, self_player_id, &dict);
    let other_cids: Vec<u16> = {
        let inst = inst_arc.read();
        inst.get_players()
            .iter()
            .filter(|p| p.client_id != client_id)
            .map(|p| p.client_id)
            .collect()
    };
    for cid in other_cids {
        if let Some(arc) = state.clients.get(cid) {
            arc.read().try_push(broadcast.clone());
        }
    }
}

fn build_properties(
    iid: u8,
    entity_id: u16,
    broadcaster_id: u16,
    dict: &HashMap<i32, Vec<u8>>,
) -> Bytes {
    let mut w = PacketWriter::new();
    w.write_u8(iid);
    w.write_u16(entity_id);
    w.write_u16(broadcaster_id);
    w.write_u8(dict.len() as u8);
    for (key, val) in dict {
        w.write_i32(*key);
        w.write_u8(val.len() as u8);
        if !val.is_empty() {
            w.write_bytes(val);
        }
    }
    encode_stream_packet(0, PacketType::Properties, w.finish().as_ref())
}
