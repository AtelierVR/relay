/// AvatarChanged packet handler.
///
/// Request:  [iid: u8][pid: u16][avatar_id: u32][server: string][version: u16]
/// Response: [iid: u8][AvatarChangedResult: u8]  (+ optional reason on failure)
/// Broadcast to others: [iid: u8][AvatarChangedResult::Changing][player_id: u16]
///                      [avatar_id: u32][server: string][version: u16]
use bytes::Bytes;
use tracing::debug;

use crate::{
    avatar::Avatar,
    handlers::context::AppState,
    proto::{
        buffer::{PacketReader, PacketWriter},
        header::encode_stream_packet,
        packet_type::PacketType,
    },
};

#[repr(u8)]
enum AvatarChangedResult {
    Changing = 0,
    Failed = 2,
    Success = 3,
}

pub async fn handle(state: &AppState, client_id: u16, uid: u16, payload: Bytes) -> Bytes {
    let mut r = PacketReader::new(payload);

    let iid = r.read_u8();
    let inst_arc = match state.instances.get(iid) {
        Some(a) => a,
        None => return make_error(uid, iid, "Invalid instance"),
    };

    // Self player must be ready.
    let self_player_id = {
        let inst = inst_arc.read();
        match inst.get_players().iter().find(|p| p.client_id == client_id) {
            Some(p) if p.is_ready() => p.id,
            _ => return make_error(uid, iid, "You are not a valid player"),
        }
    };

    let pid = r.read_u16();

    // Resolve operating player (may be someone else if caller has privilege).
    let op_player_id = if pid != u16::MAX && pid != self_player_id {
        // Check opPlayer exists and caller is allowed.
        let inst = inst_arc.read();
        let op = match inst.get_players().iter().find(|p| p.id == pid) {
            Some(p) if p.is_ready() => p,
            _ => return make_error(uid, iid, "Player not found"),
        };

        let self_p = inst
            .get_players()
            .iter()
            .find(|p| p.id == self_player_id)
            .unwrap();
        // A player is "allowed" to operate on another if they have privilege (matches C# IsAllowed).
        if !self_p.has_privilege() {
            return make_error(
                uid,
                iid,
                "You are not allowed to change this player's avatar",
            );
        }
        op.id
    } else {
        self_player_id
    };

    // Read new avatar.
    let avatar = Avatar {
        id: r.read_u32(),
        server: r.read_string().unwrap_or_default(),
        version: r.read_u16(),
        parameters: std::collections::HashMap::new(),
    };

    debug!(
        "[AvatarChanged] player {} (by client {}) in instance {}: avatar={}",
        op_player_id, client_id, iid, avatar.id
    );

    // Apply avatar.
    {
        let mut inst = inst_arc.write();
        if let Some(p) = inst.get_player_mut(op_player_id) {
            p.avatar = Some(avatar.clone());
        }
    }

    // Broadcast to all other players.
    let broadcast = build_broadcast(iid, op_player_id, &avatar);
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
            let c = arc.read();
            c.try_push(broadcast.clone());
        }
    }

    // Success response to caller.
    let mut w = PacketWriter::new();
    w.write_u8(iid);
    w.write_u8(AvatarChangedResult::Success as u8);
    encode_stream_packet(uid, PacketType::AvatarChanged, w.finish().as_ref())
}

fn build_broadcast(iid: u8, player_id: u16, avatar: &Avatar) -> Bytes {
    let mut w = PacketWriter::new();
    w.write_u8(iid);
    w.write_u8(AvatarChangedResult::Changing as u8);
    w.write_u16(player_id);
    w.write_u32(avatar.id);
    w.write_string(&avatar.server);
    w.write_u16(avatar.version);
    encode_stream_packet(0, PacketType::AvatarChanged, w.finish().as_ref())
}

fn make_error(uid: u16, iid: u8, reason: &str) -> Bytes {
    let mut w = PacketWriter::new();
    w.write_u8(iid);
    w.write_u8(AvatarChangedResult::Failed as u8);
    w.write_string(reason);
    encode_stream_packet(uid, PacketType::AvatarChanged, w.finish().as_ref())
}
