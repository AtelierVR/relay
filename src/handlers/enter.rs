/// ```packet
/// [Length: u16][UID: u16][Type: u8 = 0x06]
/// [InstanceId: u8]
/// [EnterFlags: u8]
/// (EnterFlags.UsePseudonyme  ? [Display: string])
/// (InstanceFlags.UsePassword ? [Password: string])
/// ```
/// Response (Success):
/// [iid: u8][EnterResult: u8][PlayerFlags: u32][PlayerId: u16]
/// [UserId: u32][UserAddress: string][Display: string][CreatedAt: i64]
/// [Tps: u8][Threshold: f32][RenderEntity: f32]
///
/// Broadcast Join: [iid][PlayerFlags][PlayerId][UserId][UserAddress][Display][CreatedAt][Engine][Platform]
use bitflags::bitflags;
use bytes::Bytes;
use tracing::{debug, info};

use crate::{
    handlers::{context::AppState, packet::Packet},
    instance::InstanceFlags,
    master::messages::EventPlayerJoin,
    player::{Player, PlayerFlags, PlayerStatus},
    proto::{
        buffer::{PacketReader, PacketWriter},
        header::encode_stream_packet,
        packet_type::PacketType,
    },
};

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct EnterFlags: u8 {
        const NONE           = 0;
        const AS_BOT         = 1 << 0;
        const USE_PSEUDONYM  = 1 << 1;
        const USE_PASSWORD   = 1 << 2;
        const HIDE_IN_LIST   = 1 << 3;
    }
}

#[repr(u8)]
enum EnterResult {
    Success = 0,
    NotFound = 1,
    Full = 2,
    Blacklisted = 3,
    NotWhitelisted = 4,
    InvalidGame = 5,
    IncorrectPassword = 6,
    Unknown = 7,
    Refused = 8,
    InvalidPseudonyme = 9,
}

pub async fn handle(mut packet: Packet) {
    let state = packet.state.clone();
    let client_id = packet.client_id();
    let uid = packet.uid;
    let payload = packet.payload.clone();

    let mut r = PacketReader::new(payload);

    let iid = r.read_u8();
    let enter_flags = EnterFlags::from_bits_truncate(r.read_u8());

    // Check if this is a bot in debug mode (can skip auth)
    let is_bot = enter_flags.contains(EnterFlags::AS_BOT);
    let allow_no_auth = state.config.debug && is_bot;

    // Must be authenticated (unless bot in debug mode).
    let _client_arc = match state.clients.get(client_id) {
        Some(a) if a.read().is_authenticated() || allow_no_auth => a,
        _ => {
            debug!("[Enter] client {client_id} attempted to enter instance {iid} but is not authenticated");
            return;
        }
    };

    debug!(
        "[Enter] client {client_id} → instance {iid}{}",
        if allow_no_auth {
            " (bot, auth bypassed)"
        } else {
            ""
        }
    );

    let inst_arc = match state.instances.get(iid) {
        Some(a) => a,
        None => return packet.reply_raw(make_error(uid, iid, EnterResult::NotFound, Some("Instance not found."))),
    };

    // Check not already in instance.
    {
        let inst = inst_arc.read();
        if inst.get_players().iter().any(|p| p.client_id == client_id) {
            return packet.reply_raw(make_error(uid, iid, EnterResult::Unknown, Some("Already in instance.")));
        }
        if inst.is_full() {
            return packet.reply_raw(make_error(uid, iid, EnterResult::Full, Some("Instance is full.")));
        }
        // Moderation: blacklist check omitted for now (no user addresses in basic relay)
        if inst.flags.contains(InstanceFlags::USE_WHITELIST) {
            // Would check whitelist here; proceed without user object for now.
        }
    }

    let mut p_flags = PlayerFlags::NONE;

    if enter_flags.contains(EnterFlags::AS_BOT) {
        let is_bot_allowed = inst_arc.read().flags.contains(InstanceFlags::AUTHORIZE_BOT);
        if !is_bot_allowed {
            return packet.reply_raw(make_error(
                uid,
                iid,
                EnterResult::Refused,
                Some("Bots are not authorized in this instance."),
            ));
        }
        p_flags |= PlayerFlags::IS_BOT;
    }

    if enter_flags.contains(EnterFlags::HIDE_IN_LIST) {
        p_flags |= PlayerFlags::HIDE_IN_LIST;
    }

    let display: Option<String> = if enter_flags.contains(EnterFlags::USE_PSEUDONYM) {
        r.read_string()
    } else {
        None
    };

    if inst_arc.read().flags.contains(InstanceFlags::USE_PASSWORD) {
        let provided = if enter_flags.contains(EnterFlags::USE_PASSWORD) {
            r.read_string().unwrap_or_default()
        } else {
            String::new()
        };
        if !inst_arc.read().verify_password(&provided) {
            return packet.reply_raw(make_error(
                uid,
                iid,
                EnterResult::IncorrectPassword,
                Some("Incorrect password."),
            ));
        }
    }

    // Create the player.
    let player_id = inst_arc.read().next_player_id();
    let mut player = Player::new(player_id, client_id, iid);
    player.flags = p_flags;
    player.status = PlayerStatus::Preparing;
    player.display = display.filter(|s| !s.trim().is_empty());

    // Add to instance.
    {
        let mut inst = inst_arc.write();
        inst.add_player(player);
    }

    // Log player entering with user info
    let user_info = if let Some(arc) = state.clients.get(client_id) {
        let c = arc.read();
        if let Some(u) = &c.user {
            format!("{}@{} (\"{}\")", u.id, u.address, u.display_name)
        } else {
            String::from("(unauthenticated)")
        }
    } else {
        String::from("(unknown)")
    };
    info!(
        "[Enter] player {} (client {}) entered instance {} | user={}",
        player_id, client_id, iid, user_info
    );

    // Notify node of player joining.
    {
        let (user_id_opt, display_name) = if let Some(arc) = state.clients.get(client_id) {
            let c = arc.read();
            let uid = c.user.as_ref().map(|u| u.to_identifier());
            let disp = c.user.as_ref().map(|u| u.display_name.clone())
                .unwrap_or_else(|| format!("Player {}", player_id));
            (uid, disp)
        } else {
            (None, format!("Player {}", player_id))
        };
        let _ = state.master.emit("player_join", EventPlayerJoin {
            client_id: client_id.to_string(),
            player_id: player_id.to_string(),
            instance_id: iid.to_string(),
            user: user_id_opt,
            display: display_name,
        });
    }

    // Promote master if none exists yet.
    {
        let mut inst = inst_arc.write();
        let has_master = inst.get_master().is_some();
        if !has_master {
            if let Some(oldest) = inst.get_players_mut().first_mut() {
                oldest.flags |= PlayerFlags::INSTANCE_MASTER;
            }
        }
    }

    // --- Broadcast Join to all other clients in the instance. ---
    broadcast_join(&state, client_id, player_id, &inst_arc);

    // --- Send Enter response to the entering client. ---
    build_enter_response(&mut packet, inst_arc.clone(), player_id);
}

fn build_enter_response(
    packet: &mut Packet,
    inst_arc: std::sync::Arc<parking_lot::RwLock<crate::instance::Instance>>,
    player_id: u16,
) {
    let client_id = packet.client_id();
    let uid = packet.uid;
    let inst = inst_arc.read();
    let player = match inst.get_player(player_id) {
        Some(p) => p,
        None => return,
    };
    let client = match packet.state.clients.get(client_id) {
        Some(a) => a,
        None => return,
    };
    let c = client.read();
    let (user_id, user_addr, display) = if let Some(u) = &c.user {
        (
            u.id,
            u.address.clone(),
            player
                .display
                .clone()
                .unwrap_or_else(|| u.display_name.clone()),
        )
    } else {
        (
            0u32,
            String::new(),
            player
                .display
                .clone()
                .unwrap_or_else(|| format!("Player {}", player_id)),
        )
    };

    let tps = if player.custom_tps != 0 {
        player.custom_tps
    } else {
        inst.get_effective_tps(&packet.state.config.load_balancing)
    };
    let threshold = if player.custom_threshold != 0.0 {
        player.custom_threshold
    } else {
        inst.get_effective_threshold(&packet.state.config.load_balancing)
    };

    let mut w = PacketWriter::new();
    w.write_u8(inst.internal_id);
    w.write_u8(EnterResult::Success as u8);
    w.write_u32(player.flags.bits());
    w.write_u16(player.id);
    w.write_u32(user_id);
    w.write_string(&user_addr);
    w.write_string(&display);
    w.write_i64(player.created_at);
    w.write_u8(tps);
    w.write_f32(threshold);
    w.write_f32(inst.render_entity);
    packet.reply_raw(encode_stream_packet(uid, PacketType::Enter, w.finish().as_ref()));
}

/// Send a Join broadcast to all other players already in the instance.
fn broadcast_join(
    state: &AppState,
    entering_client_id: u16,
    entering_player_id: u16,
    inst_arc: &std::sync::Arc<parking_lot::RwLock<crate::instance::Instance>>,
) {
    // Build the Join packet for the entering player.
    let join_packet = {
        let inst = inst_arc.read();
        let player = inst.get_player(entering_player_id);
        let Some(player) = player else {
            return;
        };
        let Some(client_arc) = state.clients.get(entering_client_id) else {
            return;
        };
        let c = client_arc.read();
        let (user_id, user_addr, display) = if let Some(u) = &c.user {
            (
                u.id,
                u.address.clone(),
                player
                    .display
                    .clone()
                    .unwrap_or_else(|| u.display_name.clone()),
            )
        } else {
            (
                0u32,
                String::new(),
                player
                    .display
                    .clone()
                    .unwrap_or_else(|| format!("Player {}", player.id)),
            )
        };

        let mut w = PacketWriter::new();
        w.write_u8(inst.internal_id);
        w.write_u32(player.flags.bits());
        w.write_u16(player.id);
        w.write_u32(user_id);
        w.write_string(&user_addr);
        w.write_string(&display);
        w.write_i64(player.created_at);
        w.write_string(&c.engine);
        w.write_string(&c.platform);
        encode_stream_packet(0, PacketType::Join, w.finish().as_ref())
    };

    // Get the list of OTHER players' client IDs.
    let other_client_ids: Vec<u16> = {
        let inst = inst_arc.read();
        inst.get_players()
            .iter()
            .filter(|p| p.client_id != entering_client_id)
            .map(|p| p.client_id)
            .collect()
    };

    // Send the Join packet to each other player.
    for cid in other_client_ids {
        if let Some(arc) = state.clients.get(cid) {
            let c = arc.read();
            c.try_push(join_packet.clone());
        }
    }

    // Also: send each existing player's Join packet to the entering client.
    // (So the entering client knows about everyone already present.)
    let iid = inst_arc.read().internal_id;
    let existing: Vec<(u16, u16)> = {
        let inst = inst_arc.read();
        inst.get_players()
            .iter()
            .filter(|p| p.client_id != entering_client_id)
            .map(|p| (p.id, p.client_id))
            .collect()
    };

    for (other_pid, other_cid) in existing {
        let other_join = build_existing_join(state, iid, other_pid, other_cid, inst_arc);
        if let Some(joining_arc) = state.clients.get(entering_client_id) {
            let c = joining_arc.read();
            c.try_push(other_join);
        }
    }
}

fn build_existing_join(
    state: &AppState,
    iid: u8,
    player_id: u16,
    other_cid: u16,
    inst_arc: &std::sync::Arc<parking_lot::RwLock<crate::instance::Instance>>,
) -> Bytes {
    let inst = inst_arc.read();
    let Some(player) = inst.get_player(player_id) else {
        return Bytes::new();
    };
    let Some(client_arc) = state.clients.get(other_cid) else {
        return Bytes::new();
    };
    let c = client_arc.read();
    let (user_id, user_addr, display) = if let Some(u) = &c.user {
        (
            u.id,
            u.address.clone(),
            player
                .display
                .clone()
                .unwrap_or_else(|| u.display_name.clone()),
        )
    } else {
        (
            0u32,
            String::new(),
            player
                .display
                .clone()
                .unwrap_or_else(|| format!("Player {}", player_id)),
        )
    };

    let mut w = PacketWriter::new();
    w.write_u8(iid);
    w.write_u32(player.flags.bits());
    w.write_u16(player.id);
    w.write_u32(user_id);
    w.write_string(&user_addr);
    w.write_string(&display);
    w.write_i64(player.created_at);
    w.write_string(&c.engine);
    w.write_string(&c.platform);
    encode_stream_packet(0, PacketType::Join, w.finish().as_ref())
}

fn make_error(uid: u16, iid: u8, result: EnterResult, reason: Option<&str>) -> Bytes {
    let mut w = PacketWriter::new();
    w.write_u8(iid);
    w.write_u8(result as u8);
    if let Some(r) = reason {
        w.write_string(r);
    }
    encode_stream_packet(uid, PacketType::Enter, w.finish().as_ref())
}
