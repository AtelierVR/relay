/// Stream datagram handler — multiplexes audio samples and hearing control
/// within a single packet type (Stream = 0x14).
///
/// Wire format (after the outer QUIC envelope is stripped):
///   [iid: u8][sub_type: u8][data...]
///
/// Sub-type 0x00 — Sample (Opus audio):
///   Received: [iid][0x00][channel_id:u32][level_flags:u8][sample:bytes…]
///   Broadcast:[iid][0x00][auth_player_id:u16][channel_id:u32][level_flags:u8][sample:bytes…]
///
/// Note: The client does NOT send `player_id` — the server resolves the
/// authenticated player from the QUIC connection and injects it in broadcasts.
///
/// Sub-type 0x01 — Control (hearing permission):
///   Received: [iid][0x01][listener_id:u16][speaker_id:u16][control_flags:u8]
///   Echo:[iid][0x01][listener_id:u16][speaker_id:u16][control_flags:u8]
use tracing::debug;

use crate::{
    handlers::packet::Packet,
    proto::{
        buffer::{PacketReader, PacketWriter},
        header::encode_datagram,
        packet_type::PacketType,
    },
};

/// Sub-types within the Stream(0x14) packet.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamSubType {
    /// Opus audio sample: [channel_id:u32][level_flags:u8][sample:bytes…]
    Sample = 0x00,
    /// Hearing control: [listener_id:u16][speaker_id:u16][control_flags:u8]
    Control = 0x01,
}

/// LevelFlags bit constants.
mod level {
    /// Bits 0-1: distance mode (0=Normal, 1=Whisper, 2=Broadcast).
    pub const DISTANCE_MASK: u8 = 0x03;
    /// Bit 2: Group flag — when set, `group_id:u16` follows `level_flags`.
    pub const GROUP_BIT: u8 = 0x04;
}

/// ControlFlags bit constants.
mod ctrl {
    /// Bit 0: can_hear (1=allow, 0=deny).
    pub const CAN_HEAR_BIT: u8 = 0x01;
}

impl TryFrom<u8> for StreamSubType {
    type Error = u8;

    fn try_from(v: u8) -> Result<Self, u8> {
        match v {
            0x00 => Ok(Self::Sample),
            0x01 => Ok(Self::Control),
            other => Err(other),
        }
    }
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
            debug!("[Stream] unknown instance {}", iid);
            return;
        }
    };

    let sub_type = match StreamSubType::try_from(r.read_u8()) {
        Ok(st) => st,
        Err(v) => {
            debug!("[Stream] unknown sub_type 0x{:02X}", v);
            return;
        }
    };

    match sub_type {
        StreamSubType::Sample => {
            handle_sample(&packet, &inst_arc, client_id, iid, &mut r).await;
        }
        StreamSubType::Control => {
            handle_control(&inst_arc, &mut r).await;
        }
    }
}

async fn handle_sample(
    packet: &Packet,
    inst_arc: &parking_lot::RwLock<crate::instance::Instance>,
    client_id: u16,
    iid: u8,
    r: &mut PacketReader,
) {
    let state = &packet.state;

    let self_player_id = {
        let inst = inst_arc.read();
        match inst.get_players().iter().find(|p| p.client_id == client_id) {
            Some(p) if p.is_ready() => p.id,
            _ => {
                debug!(
                    "[Stream::Sample] not-ready player from client {}",
                    client_id
                );
                return;
            }
        }
    };

    let channel_id = r.read_u32();
    let level_flags = r.read_u8();

    // If the Group flag is set, read the group ID.
    let group_id = if (level_flags & level::GROUP_BIT) != 0 {
        r.read_u16()
    } else {
        0
    };

    // Frame index (i32) and timestamp (f64) for jitter buffer ordering
    let frame_index = r.read_i32();
    let timestamp = r.read_f64();

    let remaining = r.remaining();
    if remaining == 0 {
        debug!("[Stream::Sample] empty sample from client {}", client_id);
        return;
    }

    let sample = r.read_bytes(remaining);

    let broadcast = {
        let mut w = PacketWriter::new();
        w.write_u8(iid);
        w.write_u8(StreamSubType::Sample as u8);
        w.write_u16(self_player_id);
        w.write_u32(channel_id);
        w.write_u8(level_flags);
        if (level_flags & level::GROUP_BIT) != 0 {
            w.write_u16(group_id);
        }
        w.write_i32(frame_index);
        w.write_f64(timestamp);
        w.write_bytes(&sample);
        encode_datagram(0, PacketType::Stream, w.finish().as_ref())
    };

    let inst = inst_arc.read();

    let recipients: Vec<u16> = inst
        .get_players()
        .iter()
        .filter(|p| p.client_id != client_id && p.is_ready())
        .filter(|p| {
            let hm = &inst.hearing_map;
            hm.is_empty() || hm.contains_key(&(p.id, self_player_id, channel_id))
        })
        .map(|p| p.client_id)
        .collect();

    for cid in recipients {
        if let Some(arc) = state.clients.get(cid) {
            let client = arc.read();
            let _ = client.try_push(broadcast.clone());
        }
    }
}

async fn handle_control(
    inst_arc: &parking_lot::RwLock<crate::instance::Instance>,
    r: &mut PacketReader,
) {
    let listener_id = r.read_u16();
    let speaker_id = r.read_u16();
    let control_flags = r.read_u8();
    let can_hear = (control_flags & ctrl::CAN_HEAR_BIT) != 0;

    const WILDCARD_CHANNEL: u32 = 0;
    {
        let mut inst = inst_arc.write();
        if can_hear {
            inst.hearing_map
                .insert((listener_id, speaker_id, WILDCARD_CHANNEL), true);
        } else {
            inst.hearing_map
                .remove(&(listener_id, speaker_id, WILDCARD_CHANNEL));
        }
    }

    debug!(
        "[Stream::Control] listener={} speaker={} can_hear={}",
        listener_id, speaker_id, can_hear
    );
}
