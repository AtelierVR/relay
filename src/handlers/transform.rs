/// Transform packet handler — datagram path, broadcasts spatial updates.
///
/// Datagram wire (after [UID:u16][Type:u8]):
///
/// EntityPart (sub-type 0x01):
///   [iid: u8][TransformType: u8 = 1][pid: u16][rig: u16][flags: u8]
///   (?position: vec3)(?rotation: quat)(?scale: vec3)(?velocity: vec3)(?ang_velocity: vec3)
///   Broadcast: [iid][TransformType::EntityPart][eId][rig][flags][...][bId]
///
/// ByPath (sub-type 0x00):
///   [iid: u8][TransformType: u8 = 0][path: string][flags: u8][...]
///   Broadcast: [iid][TransformType::ByPath][path][flags][...][pId]
use bytes::Bytes;
use tracing::debug;

use crate::{
    handlers::packet::Packet,
    player::rig::{Transform, TransformFlags},
    proto::{
        buffer::{PacketReader, PacketWriter},
        header::encode_datagram,
        packet_type::PacketType,
    },
};

#[repr(u8)]
pub enum TransformType {
    ByPath = 0,
    EntityPart = 1,
}

pub async fn handle(packet: Packet) {
    let state = &packet.state;
    let client_id = packet.client_id();
    let payload = packet.payload.clone();
    let mut r = PacketReader::new(payload);

    let iid = r.read_u8();
    let inst_arc = match state.instances.get(iid) {
        Some(a) => a,
        None => {
            debug!(
                "[Transform] unknown instance {} from client {}",
                iid, client_id
            );
            return;
        }
    };

    let sub_type = r.read_u8();

    match sub_type {
        1 => on_entity_part(&packet, iid, inst_arc, r),
        0 => on_by_path(&packet, iid, inst_arc, r),
        _ => debug!("[Transform] unknown sub-type {}", sub_type),
    }
}

fn on_entity_part(
    packet: &Packet,
    iid: u8,
    inst_arc: crate::instance::ArcInstance,
    mut r: PacketReader,
) {
    let state = &packet.state;
    let client_id = packet.client_id();
    // Self player must be ready.
    let self_player_id = {
        let inst = inst_arc.read();
        match inst.get_players().iter().find(|p| p.client_id == client_id) {
            Some(p) if p.is_ready() => p.id,
            _ => {
                debug!(
                    "[Transform] EntityPart: not-ready player from client {}",
                    client_id
                );
                return;
            }
        }
    };

    let pid = r.read_u16();
    let op_player_id = if pid != u16::MAX && pid != self_player_id {
        let inst = inst_arc.read();
        match inst.get_players().iter().find(|p| p.id == pid) {
            Some(p) if p.is_ready() => {
                // must have privilege to operate on another
                let self_p = inst
                    .get_players()
                    .iter()
                    .find(|p| p.id == self_player_id)
                    .unwrap();
                if !self_p.has_privilege() {
                    debug!(
                        "[Transform] EntityPart: client {} lacks privilege for player {}",
                        client_id, pid
                    );
                    return;
                }
                p.id
            }
            _ => {
                debug!("[Transform] EntityPart: op-player {} not ready", pid);
                return;
            }
        }
    } else {
        self_player_id
    };

    let rig_id = r.read_u16();
    let flags = TransformFlags::from_bits_truncate(r.read_u8());

    // Get or default existing transform.
    let mut tr = if flags.contains(TransformFlags::RESET) {
        Transform::default()
    } else {
        let inst = inst_arc.read();
        inst.get_player(op_player_id)
            .and_then(|p| p.transforms.get(rig_id))
            .unwrap_or_default()
    };

    let active_flags = flags & !TransformFlags::RESET;

    if flags.contains(TransformFlags::POSITION) {
        tr.position = r.read_vec3();
    }
    if flags.contains(TransformFlags::ROTATION) {
        tr.rotation = r.read_quat();
    }
    if flags.contains(TransformFlags::SCALE) {
        tr.scale = r.read_vec3();
    }
    if flags.contains(TransformFlags::VELOCITY) {
        tr.velocity = r.read_vec3();
    }
    if flags.contains(TransformFlags::ANG_VELOCITY) {
        tr.ang_velocity = r.read_vec3();
    }

    // Build broadcast datagram.
    let broadcast = build_entity_part(iid, op_player_id, self_player_id, rig_id, active_flags, &tr);

    // Send to all visible others.
    let recipients: Vec<u16> = {
        let inst = inst_arc.read();
        inst.get_players()
            .iter()
            .filter(|p| p.client_id != client_id && inst.can_user_see_user(p.id, op_player_id))
            .map(|p| p.client_id)
            .collect()
    };
    send_datagram_broadcast(state, &recipients, broadcast);

    // Store updated transform (strip RESET, accumulate flags).
    {
        let is_reset = flags.contains(TransformFlags::RESET);
        let mut inst = inst_arc.write();
        if let Some(p) = inst.get_player_mut(op_player_id) {
            p.transforms.set(rig_id, active_flags, tr, is_reset);
        }
    }
}

fn on_by_path(
    packet: &Packet,
    iid: u8,
    inst_arc: crate::instance::ArcInstance,
    mut r: PacketReader,
) {
    let state = &packet.state;
    let client_id = packet.client_id();
    let self_player_id = {
        let inst = inst_arc.read();
        match inst.get_players().iter().find(|p| p.client_id == client_id) {
            Some(p) if p.is_ready() && p.has_privilege() => p.id,
            _ => {
                debug!(
                    "[Transform] ByPath: not-ready or unprivileged client {}",
                    client_id
                );
                return;
            }
        }
    };

    let path = match r.read_string() {
        Some(p) if !p.is_empty() => p,
        _ => {
            debug!("[Transform] ByPath: empty path from client {}", client_id);
            return;
        }
    };

    let flags = TransformFlags::from_bits_truncate(r.read_u8());
    let mut tr = Transform::default();
    let active_flags = flags & !TransformFlags::RESET;

    if flags.contains(TransformFlags::POSITION) {
        tr.position = r.read_vec3();
    }
    if flags.contains(TransformFlags::ROTATION) {
        tr.rotation = r.read_quat();
    }
    if flags.contains(TransformFlags::SCALE) {
        tr.scale = r.read_vec3();
    }
    if flags.contains(TransformFlags::VELOCITY) {
        tr.velocity = r.read_vec3();
    }
    if flags.contains(TransformFlags::ANG_VELOCITY) {
        tr.ang_velocity = r.read_vec3();
    }

    let broadcast = build_by_path(iid, &path, self_player_id, active_flags, &tr);

    let recipients: Vec<u16> = {
        let inst = inst_arc.read();
        inst.get_players()
            .iter()
            .filter(|p| p.client_id != client_id)
            .map(|p| p.client_id)
            .collect()
    };
    send_datagram_broadcast(state, &recipients, broadcast);
}

// ── Wire builders ─────────────────────────────────────────────────────────────

fn build_entity_part(
    iid: u8,
    entity_id: u16,
    broadcaster_id: u16,
    rig: u16,
    flags: TransformFlags,
    tr: &Transform,
) -> Bytes {
    let mut w = PacketWriter::new();
    w.write_u8(iid);
    w.write_u8(TransformType::EntityPart as u8);
    w.write_u16(entity_id);
    w.write_u16(rig);
    w.write_u8(flags.bits());
    write_transform_fields(&mut w, flags, tr);
    w.write_u16(broadcaster_id);
    encode_datagram(0, PacketType::Transform, w.finish().as_ref())
}

fn build_by_path(
    iid: u8,
    path: &str,
    player_id: u16,
    flags: TransformFlags,
    tr: &Transform,
) -> Bytes {
    let mut w = PacketWriter::new();
    w.write_u8(iid);
    w.write_u8(TransformType::ByPath as u8);
    w.write_string(path);
    w.write_u8(flags.bits());
    write_transform_fields(&mut w, flags, tr);
    w.write_u16(player_id);
    encode_datagram(0, PacketType::Transform, w.finish().as_ref())
}

fn write_transform_fields(w: &mut PacketWriter, flags: TransformFlags, tr: &Transform) {
    if flags.contains(TransformFlags::POSITION) {
        w.write_vec3(tr.position);
    }
    if flags.contains(TransformFlags::ROTATION) {
        w.write_quat(tr.rotation);
    }
    if flags.contains(TransformFlags::SCALE) {
        w.write_vec3(tr.scale);
    }
    if flags.contains(TransformFlags::VELOCITY) {
        w.write_vec3(tr.velocity);
    }
    if flags.contains(TransformFlags::ANG_VELOCITY) {
        w.write_vec3(tr.ang_velocity);
    }
}

fn send_datagram_broadcast(state: &crate::handlers::context::AppState, recipients: &[u16], packet: Bytes) {
    for cid in recipients {
        if let Some(arc) = state.clients.get(*cid) {
            arc.read().send_datagram(packet.clone());
        }
    }
}
