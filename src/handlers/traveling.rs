/// Traveling handler — manages player travel state within an instance.
///
/// Request:  [iid: u8][TravelingAction: u8]
///
/// Actions:
///   Travel(0) → player requests world node info
///   Ready(1)  → player finished loading and is now fully Ready
///   Failed(2) → travel failed (no-op server side)
///
/// Response: [iid: u8][TravelingResults: u8](?world fields | reason)
use bytes::Bytes;
use tracing::debug;

use crate::{
    constants::SAFE_LOCAL_ADDRESS,
    handlers::packet::Packet,
    player::PlayerStatus,
    proto::{
        buffer::{PacketReader, PacketWriter},
        header::encode_stream_packet,
        packet_type::PacketType,
    },
};

#[repr(u8)]
enum TravelingAction {
    Travel = 0,
    Ready = 1,
    Failed = 2,
}

impl TravelingAction {
    fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Travel),
            1 => Some(Self::Ready),
            2 => Some(Self::Failed),
            _ => None,
        }
    }
}

#[repr(u8)]
enum TravelingResults {
    None = 0,
    UseUrl = 1 << 1,
    UseNode = 1 << 2,
    UseHash = 1 << 3,
    Unknown = 1 << 4,
    Ready = 1 << 5,
}

pub async fn handle(mut packet: Packet) {
    let state = packet.state.clone();
    let client_id = packet.client_id();
    let uid = packet.uid;
    let payload = packet.payload.clone();

    let mut r = PacketReader::new(payload);

    let iid = r.read_u8();
    let inst_arc = match state.instances.get(iid) {
        Some(a) => a,
        None => {
            return packet.reply_raw(make_response(
                uid,
                iid,
                TravelingResults::Unknown,
                Some("Instance not found."),
            ))
        }
    };

    let self_player_id = {
        let inst = inst_arc.read();
        match inst.get_players().iter().find(|p| p.client_id == client_id) {
            Some(p) => p.id,
            None => {
                return packet.reply_raw(make_response(
                    uid,
                    iid,
                    TravelingResults::Unknown,
                    Some("You are not in the instance."),
                ))
            }
        }
    };

    let action = match TravelingAction::from_u8(r.read_u8()) {
        Some(a) => a,
        None => return packet.reply_raw(make_response(uid, iid, TravelingResults::Unknown, Some("Unknown action."))),
    };

    match action {
        TravelingAction::Travel => do_travel(&mut packet, iid, self_player_id, &inst_arc),
        TravelingAction::Ready => do_ready(&mut packet, iid, self_player_id, &inst_arc),
        TravelingAction::Failed => {}
    }
}

fn do_travel(packet: &mut Packet, iid: u8, player_id: u16, inst_arc: &crate::instance::ArcInstance) {
    let uid = packet.uid;
    let status = {
        let inst = inst_arc.read();
        inst.get_player(player_id)
            .map(|p| p.status)
            .unwrap_or(PlayerStatus::None)
    };
    if status == PlayerStatus::None {
        return packet.reply_raw(make_response(uid, iid, TravelingResults::Unknown, Some("Player is not ready to travel.")));
    }

    let (master_id, address, version) = {
        let inst = inst_arc.read();
        let w = &inst.world;
        if w.master_id == 0 && w.address.is_empty() {
            return packet.reply_raw(make_response(uid, iid, TravelingResults::Unknown, Some("World is not set for the instance.")));
        }
        let address = if w.address == SAFE_LOCAL_ADDRESS {
            packet.state.config.node_address.clone()
        } else {
            w.address.clone()
        };
        (w.master_id, address, w.version)
    };

    {
        let mut inst = inst_arc.write();
        if let Some(p) = inst.get_player_mut(player_id) {
            p.status = PlayerStatus::Traveling;
        }
    }

    debug!("[Traveling] player {} traveling in instance {}", player_id, iid);

    let mut w = PacketWriter::new();
    w.write_u8(iid);
    w.write_u8(TravelingResults::UseNode as u8);
    w.write_u32(master_id);
    w.write_string(&address);
    w.write_u16(version);
    packet.reply_raw(encode_stream_packet(uid, PacketType::Traveling, w.finish().as_ref()));
}

fn do_ready(packet: &mut Packet, iid: u8, player_id: u16, inst_arc: &crate::instance::ArcInstance) {
    let uid = packet.uid;
    let status = {
        let inst = inst_arc.read();
        inst.get_player(player_id)
            .map(|p| p.status)
            .unwrap_or(PlayerStatus::None)
    };

    if status != PlayerStatus::Traveling {
        return packet.reply_raw(make_response(uid, iid, TravelingResults::Unknown, Some("Player is not traveling.")));
    }

    {
        let mut inst = inst_arc.write();
        if let Some(p) = inst.get_player_mut(player_id) {
            p.status = PlayerStatus::Ready;
        }
    }

    debug!("[Traveling] player {} is now Ready in instance {}", player_id, iid);

    packet.reply_raw(make_response(uid, iid, TravelingResults::Ready, None));
}

fn make_response(uid: u16, iid: u8, result: TravelingResults, reason: Option<&str>) -> Bytes {
    let mut w = PacketWriter::new();
    w.write_u8(iid);
    w.write_u8(result as u8);
    if let Some(r) = reason {
        w.write_string(r);
    }
    encode_stream_packet(uid, PacketType::Traveling, w.finish().as_ref())
}
