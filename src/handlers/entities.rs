//! Generic networked-entity handling: create / update / remove entities, and sync the room entity
//! list on demand.
//!
//! The relay owns the entity ids; a client only sends the operation. The canonical state is
//! broadcast back to every player (including the sender of a `Create`, so it learns the assigned
//! id). Props are just one entity `kind`.
//!
//! The entity structure (kind / flags) travels here; the transform (initial position included)
//! travels through the transform channel and the payload through the properties channel.

use crate::handlers::context::AppState;
use crate::instance::Entity;
use crate::player::rig::{Transform, TransformFlags};
use crate::proto::{
    buffer::{PacketReader, PacketWriter},
    header::encode_stream_packet,
    packet_type::PacketType,
};
use bytes::Bytes;
use tracing::{debug, warn};

use super::packet::Packet;
use super::transform::build_entity_part;

const OP_CREATE: u8 = 0;
const OP_UPDATE: u8 = 1;
const OP_REMOVE: u8 = 2;
const OP_SYNC: u8 = 4;

/// Rate bucket key of an `Entities` packet: `[iid][op]` → one budget per operation.
#[inline]
fn op_key(packet: &Packet) -> u64 {
    crate::rate::key(packet, &[(0, 2)])
}

pub async fn handle(packet: Packet) {
    let state = &packet.state;
    let client_id = packet.client_id();

    // Meter per operation (`[iid][op]`) before any lookup.
    if !crate::rate::check(state, client_id, op_key(&packet)) {
        return;
    }

    let mut r = PacketReader::new(packet.payload.clone());

    let iid = r.read_u8();
    let inst_arc = match state.instances.get(iid) {
        Some(a) => a,
        None => {
            debug!("[Entities] unknown instance {}", iid);
            return;
        }
    };

    let (self_player_id, privileged) = {
        let inst = inst_arc.read();
        match inst
            .get_players()
            .iter()
            .find(|p| p.client_id == client_id && p.is_ready())
        {
            Some(p) => (p.id, p.has_privilege()),
            None => {
                debug!("[Entities] not-ready player from client {}", client_id);
                return;
            }
        }
    };

    let op = r.read_u8();
    let id = r.read_u32();
    let kind = r.read_string().unwrap_or_default();
    let flags = r.read_u8();

    match op {
        OP_CREATE => {
            if kind.is_empty() {
                debug!("[Entities] create without kind from client {}", client_id);
                return;
            }

            let mut data = Entity::new();
            data.kind = kind;
            data.flags = flags;
            data.owner_id = self_player_id;

            let assigned = {
                let mut inst = inst_arc.write();
                let id = inst.next_entity_id();
                inst.entities.insert(id, data.clone());
                id
            };

            broadcast_entity(state, iid, OP_CREATE, assigned, self_player_id, &data);
        }

        OP_SYNC => {
            let list: Vec<(u32, Entity)> = {
                let inst = inst_arc.read();
                inst.entities
                    .iter()
                    .map(|(id, d)| (*id, d.clone()))
                    .collect()
            };

            for (entity_id, data) in list {
                let bytes = build_entity_event(iid, OP_SYNC, entity_id, self_player_id, &data);
                if let Some(arc) = state.clients.get(client_id) {
                    arc.read().try_push(bytes);
                }

                // Replay the last known transform of every part through the transform channel (an
                // entity is multipart: the base part plus every other part it ever moved). Only the
                // fields the client actually sent are replayed.
                let parts: Vec<(u16, TransformFlags, Transform)> = data.transforms.iter().collect();
                for (rig, part_flags, tr) in parts {
                    let part_flags = part_flags & TransformFlags::ALL;
                    if part_flags.is_empty() {
                        continue;
                    }

                    let tf =
                        build_entity_part(iid, entity_id, self_player_id, rig, part_flags, &tr);
                    if let Some(arc) = state.clients.get(client_id) {
                        arc.read().send_datagram(tf);
                    }
                }
            }
        }

        OP_REMOVE | OP_UPDATE => {
            let owner = {
                let inst = inst_arc.read();
                inst.entities.owner_of(id)
            };

            let owner = match owner {
                Some(o) => o,
                None => {
                    debug!("[Entities] entity {} not found", id);
                    return;
                }
            };

            if owner != self_player_id && !privileged {
                warn!(
                    "[Entities] player {} tried to modify entity {} owned by {}",
                    self_player_id, id, owner
                );
                return;
            }

            match op {
                OP_REMOVE => {
                    let removed = {
                        let mut inst = inst_arc.write();
                        inst.entities.remove(id)
                    };
                    if let Some(data) = removed {
                        broadcast_entity(state, iid, OP_REMOVE, id, self_player_id, &data);
                    }
                }
                _ => {
                    // OP_UPDATE: keep the existing kind when the request omits it.
                    let current_kind = {
                        let inst = inst_arc.read();
                        inst.entities.kind_of(id).map(|k| k.to_string())
                    };
                    let new_kind = if kind.is_empty() {
                        current_kind.unwrap_or_default()
                    } else {
                        kind
                    };
                    let updated = {
                        let mut inst = inst_arc.write();
                        if inst.entities.update(id, new_kind, flags, owner) {
                            inst.entities.get(id).cloned()
                        } else {
                            None
                        }
                    };
                    if let Some(data) = updated {
                        broadcast_entity(state, iid, OP_UPDATE, id, self_player_id, &data);
                    }
                }
            }
        }

        other => debug!("[Entities] unknown op {}", other),
    }
}

/// Build the `EntityEvent` payload (with the room prefix consumed by the client).
fn build_entity_event(iid: u8, op: u8, id: u32, sender: u32, data: &Entity) -> Bytes {
    let mut w = PacketWriter::new();
    w.write_u8(iid);
    w.write_u8(op);
    w.write_u32(id);
    w.write_u32(sender);
    w.write_u32(data.owner_id);
    w.write_string(&data.kind);
    w.write_u8(data.flags);

    encode_stream_packet(0, PacketType::Entities, w.finish().as_ref())
}

/// Push an entity event to every client of the instance (including the sender).
fn broadcast_entity(state: &AppState, iid: u8, op: u8, id: u32, sender: u32, data: &Entity) {
    let bytes = build_entity_event(iid, op, id, sender, data);

    let client_ids: Vec<u16> = match state.instances.get(iid) {
        Some(arc) => arc
            .read()
            .get_players()
            .iter()
            .map(|p| p.client_id)
            .collect(),
        None => return,
    };

    for cid in client_ids {
        if let Some(arc) = state.clients.get(cid) {
            arc.read().try_push(bytes.clone());
        }
    }
}
