/// ServerConfig handler — reads and updates instance configuration.
/// Requires HIGH privilege.
///
/// Request:  [iid: u8][flags: u8]  (flags=None → echo current config to caller only)
///   Tps(0x01)       → [tps: u8]
///   Threshold(0x02) → [threshold: f32]
///   Capacity(0x04)  → [capacity: u16]
///   Password(0x08)  → [password: string]
///   Flags(0x10)     → [instance_flags: u32]
///
/// Response / Broadcast: [iid][ServerConfigResult::Change][flags][...values]
use bitflags::bitflags;
use bytes::Bytes;
use tracing::debug;

use crate::{
    handlers::context::AppState,
    instance::{ArcInstance, InstanceFlags},
    proto::{
        buffer::{PacketReader, PacketWriter},
        header::encode_stream_packet,
        packet_type::PacketType,
    },
};

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ServerConfigFlags: u8 {
        const NONE                  = 0x00;
        const TPS                   = 0x01;
        const THRESHOLD             = 0x02;
        const CAPACITY              = 0x04;
        const PASSWORD              = 0x08;
        const FLAGS                 = 0x10;
        const MIN_TPS               = 0x20;
        const MAX_TPS               = 0x40;
        const LOAD_BALANCING        = 0x80;
        const ALL                   = 0xFF;
    }
}

#[repr(u8)]
enum ServerConfigResult {
    Success = 0,
    Failure = 1,
    Change = 2,
}

pub async fn handle(state: &AppState, client_id: u16, uid: u16, payload: Bytes) -> Bytes {
    let mut r = PacketReader::new(payload);

    let iid = r.read_u8();
    let inst_arc = match state.instances.get(iid) {
        Some(a) => a,
        None => return make_failure(uid, iid, "Instance not found."),
    };

    let player_id = {
        let inst = inst_arc.read();
        match inst.get_players().iter().find(|p| p.client_id == client_id) {
            Some(p) => {
                if !p.has_high_privilege() {
                    return make_failure(uid, iid, "Permission denied.");
                }
                p.id
            }
            None => return make_failure(uid, iid, "You are not in this instance."),
        }
    };

    let req_flags = ServerConfigFlags::from_bits_truncate(r.read_u8());

    // No flags → echo all current config to caller only.
    if req_flags.is_empty() {
        return build_config_response(
            &inst_arc,
            iid,
            uid,
            ServerConfigFlags::ALL,
            player_id,
            &state.config.load_balancing,
        );
    }

    let mut result_flags = ServerConfigFlags::NONE;

    if req_flags.contains(ServerConfigFlags::TPS) {
        let tps = r.read_u8();
        inst_arc.write().tps = tps;
        result_flags |= ServerConfigFlags::TPS;
    }
    if req_flags.contains(ServerConfigFlags::THRESHOLD) {
        let threshold = r.read_f32();
        inst_arc.write().threshold = threshold;
        result_flags |= ServerConfigFlags::THRESHOLD;
    }
    if req_flags.contains(ServerConfigFlags::CAPACITY) {
        let cap = r.read_u16();
        inst_arc.write().capacity = cap;
        result_flags |= ServerConfigFlags::CAPACITY;
    }
    if req_flags.contains(ServerConfigFlags::PASSWORD) {
        let pw = r.read_string().unwrap_or_default();
        inst_arc
            .write()
            .set_password(if pw.is_empty() { None } else { Some(pw) });
        result_flags |= ServerConfigFlags::PASSWORD;
    }
    if req_flags.contains(ServerConfigFlags::FLAGS) {
        let f = InstanceFlags::from_bits_truncate(r.read_u32());
        inst_arc.write().flags = f;
        result_flags |= ServerConfigFlags::FLAGS;
    }
    if req_flags.contains(ServerConfigFlags::LOAD_BALANCING) {
        let enabled = r.read_u8() != 0;
        inst_arc.write().load_balancing_enabled = Some(enabled);
        result_flags |= ServerConfigFlags::LOAD_BALANCING;
    }

    if result_flags.is_empty() {
        return make_failure(uid, iid, "No valid configuration flags.");
    }

    debug!(
        "[ServerConfig] player {} in instance {} updated {:?}",
        player_id, iid, result_flags
    );

    // Collect players' IDs for broadcast.
    let player_ids: Vec<(u16, u16)> = {
        let inst = inst_arc.read();
        inst.get_players()
            .iter()
            .map(|p| (p.id, p.client_id))
            .collect()
    };

    for (pid, cid) in &player_ids {
        let resp = build_config_response(
            &inst_arc,
            iid,
            if cid == &client_id { uid } else { 0 },
            result_flags,
            *pid,
            &state.config.load_balancing,
        );
        if let Some(arc) = state.clients.get(*cid) {
            if cid == &client_id {
                // Will be returned directly
            } else {
                arc.read().try_push(resp);
            }
        }
    }

    build_config_response(
        &inst_arc,
        iid,
        uid,
        result_flags,
        player_id,
        &state.config.load_balancing,
    )
}

fn build_config_response(
    inst_arc: &ArcInstance,
    iid: u8,
    uid: u16,
    flags: ServerConfigFlags,
    player_id: u16,
    lb_config: &crate::config::LoadBalancingConfig,
) -> Bytes {
    let inst = inst_arc.read();
    let (custom_tps, custom_threshold) = inst
        .get_player(player_id)
        .map(|p| (p.custom_tps, p.custom_threshold))
        .unwrap_or((0, 0.0));

    let mut w = PacketWriter::new();
    w.write_u8(iid);
    w.write_u8(ServerConfigResult::Change as u8);
    w.write_u8(flags.bits());

    if flags.contains(ServerConfigFlags::TPS) {
        w.write_u8(if custom_tps != 0 {
            custom_tps
        } else {
            // Use adaptive TPS from load balancing
            inst.get_player_tps(player_id, lb_config)
        });
    }
    if flags.contains(ServerConfigFlags::THRESHOLD) {
        w.write_f32(if custom_threshold != 0.0 {
            custom_threshold
        } else {
            // Use adaptive threshold from load balancing
            inst.get_player_threshold(player_id, lb_config)
        });
    }
    if flags.contains(ServerConfigFlags::CAPACITY) {
        w.write_u16(inst.capacity);
    }
    if flags.contains(ServerConfigFlags::FLAGS) {
        w.write_u32(inst.flags.bits());
    }
    if flags.contains(ServerConfigFlags::PASSWORD) {
        // Just signal password presence (1 byte boolean).
        w.write_u8(if inst.has_password() { 1 } else { 0 });
    }
    if flags.contains(ServerConfigFlags::MIN_TPS) {
        w.write_u8(lb_config.min_tps);
    }
    if flags.contains(ServerConfigFlags::MAX_TPS) {
        w.write_u8(inst.tps); // Max TPS is the base TPS of the instance
    }
    if flags.contains(ServerConfigFlags::LOAD_BALANCING) {
        let enabled = inst.load_balancing_enabled.unwrap_or(lb_config.enabled);
        w.write_u8(if enabled { 1 } else { 0 });
    }

    encode_stream_packet(uid, PacketType::ServerConfig, w.finish().as_ref())
}

fn make_failure(uid: u16, iid: u8, reason: &str) -> Bytes {
    let mut w = PacketWriter::new();
    w.write_u8(iid);
    w.write_u8(ServerConfigResult::Failure as u8);
    w.write_string(reason);
    encode_stream_packet(uid, PacketType::ServerConfig, w.finish().as_ref())
}

/// Broadcast ServerConfig change to all players in an instance.
/// Should be called when adaptive load balancing changes TPS/threshold.
pub fn broadcast_config_change(
    state: &AppState,
    inst_arc: &ArcInstance,
    iid: u8,
    flags: ServerConfigFlags,
) {
    use tracing::info;

    let player_ids: Vec<(u16, u16)> = {
        let inst = inst_arc.read();
        inst.get_players()
            .iter()
            .map(|p| (p.id, p.client_id))
            .collect()
    };

    if player_ids.is_empty() {
        return;
    }

    info!(
        "[ServerConfig] Broadcasting {:?} to {} players in instance {}",
        flags,
        player_ids.len(),
        iid
    );

    for (pid, cid) in &player_ids {
        let resp = build_config_response(
            inst_arc,
            iid,
            0, // uid=0 for broadcasts
            flags,
            *pid,
            &state.config.load_balancing,
        );
        if let Some(arc) = state.clients.get(*cid) {
            arc.read().try_push(resp);
        }
    }
}
