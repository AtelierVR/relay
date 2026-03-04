/// Quit packet handler — leaves an instance and broadcasts the departure.
///
/// Request:  [iid: u8][QuitType: u8](?[reason: string])
/// Response (Quit): [iid: u8][QuitType: u8](?[reason: string])
/// Broadcast (Leave) to each other ready player: [iid: u8][QuitType: u8][player_id: u16]
///   ... also sends each other player's leave back to the departing player.
use bitflags::bitflags;
use bytes::Bytes;
use tracing::{debug, info};

use crate::{
    handlers::context::AppState,
    player::PlayerStatus,
    proto::{
        buffer::{PacketReader, PacketWriter},
        header::encode_stream_packet,
        packet_type::PacketType,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum QuitType {
    Normal = 0,
    Timeout = 1,
    ModerationKick = 2,
    VoteKick = 3,
    ConfigurationError = 4,
    UnknownError = 5,
}

impl QuitType {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Normal,
            1 => Self::Timeout,
            2 => Self::ModerationKick,
            3 => Self::VoteKick,
            4 => Self::ConfigurationError,
            5 => Self::UnknownError,
            _ => Self::UnknownError,
        }
    }

    pub fn is_moderation_action(self) -> bool {
        matches!(self, Self::ModerationKick | Self::VoteKick)
    }
}

pub async fn handle(state: &AppState, client_id: u16, uid: u16, payload: Bytes) -> Bytes {
    let mut r = PacketReader::new(payload);

    let iid = r.read_u8();
    let inst_arc = match state.instances.get(iid) {
        Some(a) => a,
        None => {
            return make_quit_response(
                uid,
                iid,
                QuitType::UnknownError,
                Some("Instance not found."),
            )
        }
    };

    let self_player_id = {
        let inst = inst_arc.read();
        match inst.get_players().iter().find(|p| p.client_id == client_id) {
            Some(p) => p.id,
            None => {
                return make_quit_response(
                    uid,
                    iid,
                    QuitType::UnknownError,
                    Some("You are not in the instance."),
                )
            }
        }
    };

    let quit_type = QuitType::from_u8(r.read_u8());
    let reason: Option<String> = if r.remaining() > 2 {
        r.read_string()
    } else {
        None
    };

    // Log player leaving with user info
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
    let reason_str = reason.as_deref().unwrap_or("none");
    info!(
        "[Quit] player {} (client {}) left instance {} | user={} type={:?} reason={}",
        self_player_id, client_id, iid, user_info, quit_type, reason_str
    );

    // Collect all other READY players for Leave fanout: (player_id, client_id, has_privilege).
    let others: Vec<(u16, u16, bool)> = {
        let inst = inst_arc.read();
        inst.get_players()
            .iter()
            .filter(|p| p.id != self_player_id && p.status == PlayerStatus::Ready)
            .map(|p| (p.id, p.client_id, p.has_privilege()))
            .collect()
    };

    // Determine whether this player was ready (affects whether we do the Leave fanout).
    let was_ready = {
        let inst = inst_arc.read();
        inst.get_players()
            .iter()
            .find(|p| p.id == self_player_id)
            .map(|p| p.status == PlayerStatus::Ready)
            .unwrap_or(false)
    };

    if was_ready {
        // Send Leave to each other, and send their Leave back to self.
        // C#: `allMessage = other.HasPrivilege` for client-initiated quits.
        for (other_player_id, other_cid, other_has_priv) in &others {
            // Privileged receivers see QuitType::Normal; others see the real type.
            let type_for_other = if *other_has_priv || quit_type == QuitType::Normal {
                QuitType::Normal
            } else {
                quit_type
            };
            // For moderation actions, privileged receivers also get [by_id: u16] + reason.
            let by_id = if *other_has_priv && quit_type.is_moderation_action() {
                Some(self_player_id)
            } else {
                None
            };
            let reason_to_send = if by_id.is_some() {
                reason.as_deref()
            } else {
                None
            };
            let leave_to_other =
                build_leave(iid, self_player_id, type_for_other, by_id, reason_to_send);
            if let Some(arc) = state.clients.get(*other_cid) {
                arc.read().try_push(leave_to_other);
            }

            // Message sent to self about each other leaving.
            let leave_to_self = build_leave(iid, *other_player_id, QuitType::Normal, None, None);
            if let Some(arc) = state.clients.get(client_id) {
                arc.read().try_push(leave_to_self);
            }
        }
    }

    // Remove player from instance.
    {
        let mut inst = inst_arc.write();
        inst.remove_player(self_player_id);
    }

    // If instance is empty now, we could leave it – for now we keep it.

    make_quit_response(uid, iid, quit_type, reason.as_deref())
}

fn build_leave(
    iid: u8,
    player_id: u16,
    quit_type: QuitType,
    by_id: Option<u16>,
    reason: Option<&str>,
) -> Bytes {
    let mut w = PacketWriter::new();
    w.write_u8(iid);
    w.write_u8(quit_type as u8);
    w.write_u16(player_id);
    // C#: privileged receivers get [by_id: u16] (and optional reason) for moderation actions.
    if let Some(by) = by_id {
        w.write_u16(by);
        if let Some(r) = reason {
            w.write_string(r);
        }
    }
    encode_stream_packet(0, PacketType::Leave, w.finish().as_ref())
}

/// Remove a client from all instances they are in, broadcasting `Leave` to remaining players.
/// Called on explicit `Disconnect` packet and on QUIC connection close.
pub fn leave_all_instances(state: &AppState, client_id: u16) {
    let mut entries: Vec<(u8, u16, bool)> = Vec::new();
    state.instances.for_each(|iid, inst_arc| {
        let inst = inst_arc.read();
        if let Some(p) = inst.get_players().iter().find(|p| p.client_id == client_id) {
            entries.push((iid, p.id, p.is_ready()));
        }
    });

    for (iid, player_id, was_ready) in entries {
        let inst_arc = match state.instances.get(iid) {
            Some(a) => a,
            None => continue,
        };

        if was_ready {
            let others: Vec<u16> = {
                let inst = inst_arc.read();
                inst.get_players()
                    .iter()
                    .filter(|p| p.id != player_id && p.is_ready())
                    .map(|p| p.client_id)
                    .collect()
            };
            let leave = build_leave(iid, player_id, QuitType::Normal, None, None);
            for other_cid in others {
                if let Some(arc) = state.clients.get(other_cid) {
                    arc.read().try_push(leave.clone());
                }
            }
        }

        inst_arc.write().remove_player(player_id);
    }
}

fn make_quit_response(uid: u16, iid: u8, quit_type: QuitType, reason: Option<&str>) -> Bytes {
    let mut w = PacketWriter::new();
    w.write_u8(iid);
    w.write_u8(quit_type as u8);
    if let Some(r) = reason {
        w.write_string(r);
    }
    encode_stream_packet(uid, PacketType::Quit, w.finish().as_ref())
}
