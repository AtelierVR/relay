/// ```packet
/// [Length: u16][UID: u16][Type: u8 = 0x01]
/// [ProtocolVersion: u16]
/// [Engine: string]
/// [Platform: string]
/// ```
/// Response:
/// [ProtocolVersion: u16][ClientId: u16]
/// [RemoteIP: bytes (4 or 16)][RemotePort: u16]
/// [Flags: u8]
/// (Flags.HasMaster ? [MasterAddress: string])
/// [MaxPacketSize: u16][ConnectionTimeout: u16][KeepAliveInterval: u16]
use std::net::IpAddr;

use bytes::Bytes;
use tracing::{debug, info, warn};

use crate::{
    client::client::AuthState,
    constants::PROTOCOL_VERSION,
    handlers::context::AppState,
    proto::{
        buffer::{PacketReader, PacketWriter},
        header::encode_stream_packet,
        packet_type::PacketType,
    },
    utils::hex_fmt,
};

/// Handshake flags (matches C# `HandshakeFlags`).
#[repr(u8)]
enum HandshakeFlags {
    None = 0,
    IsOffline = 1 << 0,
}

pub fn handle(state: &AppState, client_id: u16, uid: u16, payload: Bytes) -> Bytes {
    debug!(
        "[Handshake] client {}: raw payload: {}",
        client_id,
        hex_fmt::fmt_bytes(payload.as_ref(), 32)
    );
    let mut r = PacketReader::new(payload);
    let protocol = r.read_u16();

    if protocol != PROTOCOL_VERSION {
        warn!("[Handshake] client {client_id}: incompatible protocol {protocol} (expected {PROTOCOL_VERSION})");
        return Bytes::new();
    }

    let engine = r.read_string().unwrap_or_default();
    let platform = r.read_string().unwrap_or_default();

    info!("[Handshake] client {client_id}: engine=\"{engine}\" platform=\"{platform}\"");

    // Update client state.
    if let Some(arc) = state.clients.get(client_id) {
        let mut c = arc.write();
        c.engine = engine;
        c.platform = platform;
        c.auth_state = AuthState::Handshaked;
    }

    // Build response.
    let mut w = PacketWriter::new();
    w.write_u16(PROTOCOL_VERSION);
    w.write_u16(client_id);

    // We don't have the raw IP bytes in QUIC the same way, but we can use 0.0.0.0.
    // The client mainly uses this to know its own address.
    let ip_bytes = [0u8, 0, 0, 0];
    w.write_bytes(&ip_bytes);
    w.write_u16(0); // port

    let master_addr = &state.config.use_address;
    let has_master = !master_addr.is_empty();
    let flags: u8 = if has_master {
        HandshakeFlags::None as u8
    } else {
        HandshakeFlags::IsOffline as u8
    };
    w.write_u8(flags);
    if has_master {
        w.write_string(master_addr);
    }

    w.write_u16(crate::constants::MAX_PACKET_SIZE as u16);
    w.write_u16(state.config.connection_timeout);
    w.write_u16(state.config.keep_alive_interval);

    encode_stream_packet(uid, PacketType::Handshake, w.finish().as_ref())
}
