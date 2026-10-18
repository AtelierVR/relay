/// Properties handler — broadcasts a key/value byte dictionary for an entity.
///
/// A player is an entity, so players and networked entities (props, ...) share this single path: the
/// owner of the target entity (a player owns itself) or a privileged player may write, and the
/// dictionary is relayed to the other players of the instance. The client dispatches on the entity
/// it resolves: an entity payload for a prop, the avatar-parameter pipeline for a player.
///
/// Request:  [iid: u8][eid: u32][count: u8] then count×([key: i32][len: u8][bytes: len])
///   `eid` is any entity id; `SELF` (`u32::MAX`) selects the sending player itself.
///   count=0 → empty dictionary echoed to the caller only
/// Broadcast: [iid][eId][bId(self)][count][...entries]
use bytes::Bytes;
use std::collections::HashMap;
use tracing::debug;

use crate::{
    constants::SELF,
    handlers::packet::Packet,
    proto::{
        buffer::{PacketReader, PacketWriter},
        header::encode_stream_packet,
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
            debug!("[Properties] unknown instance {}", iid);
            return;
        }
    };

    // Only a ready player may write properties.
    let (self_player_id, privileged) = {
        let inst = inst_arc.read();
        match inst.get_player_by_client(client_id) {
            Some(p) if p.is_ready() => (p.id, p.has_privilege()),
            _ => {
                debug!("[Properties] not-ready player from client {}", client_id);
                return;
            }
        }
    };

    // `SELF` is the "self player" shortcut of the protocol; every other id is an entity id.
    let requested = r.read_u32();
    let entity_id = if requested == SELF {
        self_player_id
    } else {
        requested
    };

    // A player is an entity: the owner of the target (a player owns itself) or a privileged player
    // may write to it.
    let owner = {
        let inst = inst_arc.read();
        inst.get_entity(entity_id).map(|entity| entity.owner_id)
    };

    if owner != Some(self_player_id) && !privileged {
        debug!(
            "[Properties] player {} tried to write the properties of entity {} (owner {:?})",
            self_player_id, entity_id, owner
        );
        return;
    }

    let count = r.read_u8();

    // C#: if count==0, echo the empty dictionary back to the caller only and return.
    if count == 0 {
        let packet = build_properties(iid, entity_id, self_player_id, &HashMap::new());
        if let Some(arc) = state.clients.get(client_id) {
            arc.read().try_push(packet);
        }
        return;
    }

    let mut dict: HashMap<i32, Vec<u8>> = HashMap::with_capacity(count as usize);
    for _ in 0..count {
        let key = r.read_i32();
        let len = r.read_u8();
        let val = r.read_bytes(len as usize).to_vec();
        dict.insert(key, val);
    }

    // Broadcast to every other player of the instance.
    let broadcast = build_properties(iid, entity_id, self_player_id, &dict);
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
    entity_id: u32,
    broadcaster_id: u32,
    dict: &HashMap<i32, Vec<u8>>,
) -> Bytes {
    let mut w = PacketWriter::new();
    w.write_u8(iid);
    w.write_u32(entity_id);
    w.write_u32(broadcaster_id);
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
