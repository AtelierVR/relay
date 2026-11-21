//! Generic networked-entity handling: create / remove entities, sync the room entity list, and hand
//! the ownership operation over to [`ownership`].
//!
//! The relay owns the entity ids: a `Create` carries no id, the client only names the type it needs
//! and the relay assigns the next free id. Failures are answered with an `Error` carrying a result
//! code.
//!
//! The entity structure (type / flags) travels here; the transform (initial position included)
//! travels through the transform channel and the payload through the properties channel.

mod ownership;

pub use ownership::reassign_ownership;

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

use ownership::on_ownership;

const OP_CREATE: u8 = 0;
const OP_REMOVE: u8 = 2;
const OP_SYNC: u8 = 4;
const OP_OWNERSHIP: u8 = 5;
const OP_ERROR: u8 = 9;

/// Result of an entity operation, echoed to the caller.
const RESULT_OK: u8 = 0;
/// The player may not do that (no privilege, entity owned by another player).
const RESULT_DENIED: u8 = 1;
/// The targeted entity or request does not exist.
const RESULT_NOT_FOUND: u8 = 2;
/// The instance has no free entity id left.
const RESULT_OVERFLOW: u8 = 3;

/// Rate bucket key of an `Entities` packet: `[iid][op]` → one budget per operation.
#[inline]
fn op_key(packet: &Packet) -> u64 {
    crate::rate::key(packet, &[(0, 2)])
}

/// Everything an entity event carries.
struct Event<'a> {
    iid: u8,
    op: u8,
    result: u8,
    sub: u8,
    id: u32,
    sender: u32,
    request: u16,
    entity: &'a Entity,
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

    match op {
        OP_CREATE => {
            // Creating an entity is a privileged operation.
            if !privileged {
                warn!(
                    "[Entities] unprivileged client {} tried to create an entity",
                    client_id
                );
                fail(state, client_id, iid, RESULT_DENIED, 0);
                return;
            }

            // No id: the relay assigns it. The client only names the type it needs.
            let kind = r.read_u32();
            let flags = r.read_u8();

            // An entity is identified by its type: a client asking for a type the room already holds (a
            // rejoin, a master change, a world reloaded) is mapped onto the entity already assigned.
            let existing = inst_arc.read().entities.find_by_kind(kind);

            if let Some(id) = existing {
                let data = {
                    let mut inst = inst_arc.write();
                    inst.entities.get_mut(id).map(|e| {
                        // The id is kept; the player creating it again owns it (the previous owner may be
                        // a client that left the room).
                        e.owner_id = self_player_id;
                        e.clone()
                    })
                };

                if let Some(data) = data {
                    debug!(
                        "[Entities] client {} created type {} entity: reusing id {}",
                        client_id, kind, id
                    );

                    broadcast(
                        state,
                        iid,
                        build(&Event {
                            iid,
                            op: OP_CREATE,
                            result: RESULT_OK,
                            sub: 0,
                            id,
                            sender: self_player_id,
                            request: 0,
                            entity: &data,
                        }),
                    );
                    return;
                }
            }

            let mut data = Entity::new();
            data.kind = kind;
            data.flags = flags;
            // The entity belongs to the player that created it.
            data.owner_id = self_player_id;

            let assigned = {
                let mut inst = inst_arc.write();
                let assigned = inst.next_entity_id();

                if assigned != u32::MAX {
                    inst.entities.insert(assigned, data.clone());
                }

                assigned
            };

            if assigned == u32::MAX {
                warn!(
                    "[Entities] client {} could not create a type {} entity: no free id",
                    client_id, kind
                );
                fail(state, client_id, iid, RESULT_OVERFLOW, 0);
                return;
            }

            broadcast(
                state,
                iid,
                build(&Event {
                    iid,
                    op: OP_CREATE,
                    result: RESULT_OK,
                    sub: 0,
                    id: assigned,
                    sender: self_player_id,
                    request: 0,
                    entity: &data,
                }),
            );
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
                let bytes = build(&Event {
                    iid,
                    op: OP_SYNC,
                    result: RESULT_OK,
                    sub: 0,
                    id: entity_id,
                    sender: self_player_id,
                    request: 0,
                    entity: &data,
                });
                push_to(state, client_id, bytes);

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

        OP_REMOVE => {
            let id = r.read_u32();

            let data = match resolve(&inst_arc, id) {
                Some(d) if d.owner_id == self_player_id || privileged => d,
                Some(_) => {
                    warn!(
                        "[Entities] player {} tried to remove entity {} owned by another player",
                        self_player_id, id
                    );
                    fail(state, client_id, iid, RESULT_DENIED, id);
                    return;
                }
                None => {
                    fail(state, client_id, iid, RESULT_NOT_FOUND, id);
                    return;
                }
            };

            let removed = {
                let mut inst = inst_arc.write();
                // The requests still pending on that entity have no entity anymore.
                inst.pending_ownership
                    .retain(|_, pending| pending.entity != id);
                inst.entities.remove(id)
            };
            if removed.is_some() {
                broadcast(
                    state,
                    iid,
                    build(&Event {
                        iid,
                        op: OP_REMOVE,
                        result: RESULT_OK,
                        sub: 0,
                        id,
                        sender: self_player_id,
                        request: 0,
                        entity: &data,
                    }),
                );
            }
        }

        OP_OWNERSHIP => on_ownership(
            state,
            &inst_arc,
            &mut r,
            iid,
            client_id,
            self_player_id,
            privileged,
        ),

        other => debug!("[Entities] unknown op {}", other),
    }
}

/// Current record of an entity, or `None` when the instance does not know it.
fn resolve(inst_arc: &crate::instance::ArcInstance, id: u32) -> Option<Entity> {
    inst_arc.read().entities.get(id).cloned()
}

/// Build the `EntityEvent` payload (with the room prefix consumed by the client).
fn build(event: &Event) -> Bytes {
    let mut w = PacketWriter::new();
    w.write_u8(event.iid);
    w.write_u8(event.op);
    w.write_u8(event.result);
    w.write_u8(event.sub);
    w.write_u32(event.id);
    w.write_u32(event.sender);
    w.write_u32(event.entity.owner_id);
    w.write_u32(event.entity.kind);
    w.write_u8(event.entity.flags);
    w.write_u16(event.request);

    encode_stream_packet(0, PacketType::Entities, w.finish().as_ref())
}

/// The failure of an entity operation, ready to be pushed to its sender.
fn error_bytes(iid: u8, result: u8, id: u32) -> Bytes {
    build(&Event {
        iid,
        op: OP_ERROR,
        result,
        sub: 0,
        id,
        sender: 0,
        request: 0,
        entity: &Entity::new(),
    })
}

/// Answer the caller that its request could not be performed, and why.
fn fail(state: &AppState, client_id: u16, iid: u8, result: u8, id: u32) {
    push_to(state, client_id, error_bytes(iid, result, id));
}

/// Push an entity event to a single client.
fn push_to(state: &AppState, client_id: u16, bytes: Bytes) {
    if let Some(arc) = state.clients.get(client_id) {
        arc.read().try_push(bytes);
    }
}

/// Push an entity event to the client of a player of the instance.
fn push_to_player(
    state: &AppState,
    inst_arc: &crate::instance::ArcInstance,
    player_id: u32,
    bytes: Bytes,
) {
    let client_id = inst_arc.read().get_player(player_id).map(|p| p.client_id);

    if let Some(client_id) = client_id {
        push_to(state, client_id, bytes);
    }
}

/// Push an entity event to every privileged player of the instance.
fn send_to_privileged(state: &AppState, inst_arc: &crate::instance::ArcInstance, bytes: Bytes) {
    let clients: Vec<u16> = inst_arc
        .read()
        .get_players()
        .iter()
        .filter(|p| p.is_ready() && p.has_privilege())
        .map(|p| p.client_id)
        .collect();

    for cid in clients {
        push_to(state, cid, bytes.clone());
    }
}

/// Push an entity event to every client of the instance (including the sender).
fn broadcast(state: &AppState, iid: u8, bytes: Bytes) {
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
        push_to(state, cid, bytes.clone());
    }
}
