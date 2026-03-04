/// PlayerUpdate handler — updates mutable player fields and broadcasts the change.
///
/// Request:  [iid: u8][pid: u16][flags: u8]
///   flags = None → echo all fields back to requester
///   DisplayName → [new_display: string]
/// Response / Broadcast: [iid: u8][PlayerUpdateResult: u8][pid: u16][flags: u8]
///   (?[display: string])(?[player_flags: u32])
use bitflags::bitflags;
use bytes::Bytes;
use tracing::debug;

use crate::{
    handlers::context::AppState,
    proto::{
        buffer::{PacketReader, PacketWriter},
        header::encode_stream_packet,
        packet_type::PacketType,
    },
};

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PlayerUpdateFlags: u8 {
        const NONE         = 0x00;
        const DISPLAY_NAME = 0x01;
        const FLAGS        = 0x02;
        const ALL          = 0x03;
    }
}

#[repr(u8)]
enum PlayerUpdateResult {
    Success = 0,
    Failure = 1,
    Change = 2,
}

pub async fn handle(state: &AppState, client_id: u16, uid: u16, payload: Bytes) -> Bytes {
    let mut r = PacketReader::new(payload);

    let iid = r.read_u8();
    let pid = r.read_u16();

    let inst_arc = match state.instances.get(iid) {
        Some(a) => a,
        None => return make_failure(uid, iid, "Instance not found."),
    };

    // Find the requested player (may or may not be the calling client).
    let _player_exists = {
        let inst = inst_arc.read();
        inst.get_player(pid).is_some()
    };
    if !_player_exists {
        return make_failure(uid, iid, "Player not found.");
    }

    let req_flags = PlayerUpdateFlags::from_bits_truncate(r.read_u8());

    // If none → echo All back to requester only.
    if req_flags.is_empty() {
        let pkt = build_update(&inst_arc, iid, pid, PlayerUpdateFlags::ALL, state);
        // Send to the requesting client with the request uid.
        let mut w = PacketWriter::new();
        // re-build with uid
        let (disp, pflags) = {
            let inst = inst_arc.read();
            let p = inst.get_player(pid).unwrap();
            (p.display.clone(), p.flags.bits())
        };
        let encoded = build_update_packet(
            uid,
            iid,
            PlayerUpdateResult::Change,
            pid,
            PlayerUpdateFlags::ALL,
            disp.as_deref(),
            pflags,
        );
        return encoded;
    }

    let mut result_flags = PlayerUpdateFlags::NONE;

    // Apply DisplayName change.
    if req_flags.contains(PlayerUpdateFlags::DISPLAY_NAME) {
        let new_display = r.read_string();
        let mut inst = inst_arc.write();
        if let Some(p) = inst.get_player_mut(pid) {
            p.display = new_display;
            result_flags |= PlayerUpdateFlags::DISPLAY_NAME;
            result_flags |= PlayerUpdateFlags::FLAGS; // also notify flags
        }
    }

    if result_flags.is_empty() {
        return make_failure(uid, iid, "No valid update flags.");
    }

    debug!(
        "[PlayerUpdate] player {} in instance {} updated {:?}",
        pid, iid, result_flags
    );

    // Read updated values.
    let (disp, pflags) = {
        let inst = inst_arc.read();
        let p = inst.get_player(pid).unwrap();
        (p.display.clone(), p.flags.bits())
    };

    // Broadcast to all other players.
    let other_cids: Vec<u16> = {
        let inst = inst_arc.read();
        inst.get_players()
            .iter()
            .filter(|p| p.client_id != client_id)
            .map(|p| p.client_id)
            .collect()
    };
    let bcast = build_update_packet(
        0,
        iid,
        PlayerUpdateResult::Change,
        pid,
        result_flags,
        disp.as_deref(),
        pflags,
    );
    for cid in other_cids {
        if let Some(arc) = state.clients.get(cid) {
            arc.read().try_push(bcast.clone());
        }
    }

    build_update_packet(
        uid,
        iid,
        PlayerUpdateResult::Change,
        pid,
        result_flags,
        disp.as_deref(),
        pflags,
    )
}

fn build_update(
    inst_arc: &crate::instance::ArcInstance,
    iid: u8,
    pid: u16,
    flags: PlayerUpdateFlags,
    state: &AppState,
) -> Bytes {
    let (disp, pflags) = {
        let inst = inst_arc.read();
        let p = inst.get_player(pid).unwrap();
        (p.display.clone(), p.flags.bits())
    };
    build_update_packet(
        0,
        iid,
        PlayerUpdateResult::Change,
        pid,
        flags,
        disp.as_deref(),
        pflags,
    )
}

fn build_update_packet(
    uid: u16,
    iid: u8,
    result: PlayerUpdateResult,
    pid: u16,
    flags: PlayerUpdateFlags,
    display: Option<&str>,
    player_flags: u32,
) -> Bytes {
    let mut w = PacketWriter::new();
    w.write_u8(iid);
    w.write_u8(result as u8);
    w.write_u16(pid);
    w.write_u8(flags.bits());
    if flags.contains(PlayerUpdateFlags::DISPLAY_NAME) {
        w.write_string(display.unwrap_or(""));
    }
    if flags.contains(PlayerUpdateFlags::FLAGS) {
        w.write_u32(player_flags);
    }
    encode_stream_packet(uid, PacketType::PlayerUpdate, w.finish().as_ref())
}

fn make_failure(uid: u16, iid: u8, reason: &str) -> Bytes {
    let mut w = PacketWriter::new();
    w.write_u8(iid);
    w.write_u8(PlayerUpdateResult::Failure as u8);
    w.write_string(reason);
    encode_stream_packet(uid, PacketType::PlayerUpdate, w.finish().as_ref())
}
