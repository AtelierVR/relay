use std::sync::Arc;

use bytes::Bytes;
use tracing::{info, warn};

use crate::{
    client::client::AuthState,
    constants::PROTOCOL_VERSION,
    handlers::{context::AppState, packet::Packet},
    master::messages::EventClientConnected,
    proto::{
        buffer::{PacketReader, PacketWriter},
        header::encode_stream_packet,
        packet_type::PacketType,
    },
};

/// Handshake flags (matches C# `HandshakeFlags`).
#[repr(u8)]
enum HandshakeFlags {
    None = 0,
    IsOffline = 1 << 0,
}

pub fn handle(mut packet: Packet) {
    let state = packet.state.clone();
    let client_id = packet.client_id();
    let uid = packet.uid;
    let payload = packet.payload.clone();
    let resp = handle_inner(&state, client_id, uid, payload);
    if !resp.is_empty() {
        packet.reply_raw(resp);
    }
}

pub fn handle_inner(state: &Arc<AppState>, client_id: u16, uid: u16, payload: Bytes) -> Bytes {
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
        c.engine = engine.clone();
        c.platform = platform.clone();
        c.auth_state = AuthState::Handshaked;
    }

    // Notify the master server that a client has connected (platform/engine now known).
    let address = state
        .clients
        .get(client_id)
        .map(|arc| arc.read().address.clone())
        .unwrap_or_default();
    let connected_at = state
        .clients
        .get(client_id)
        .map(|arc| arc.read().connected_at)
        .unwrap_or(0);
    let _ = state.master.emit(
        "client_connected",
        EventClientConnected {
            id: client_id,
            address,
            platform: platform.clone(),
            engine: engine.clone(),
            connected_at,
        },
    );

    // Build response.
    let mut w = PacketWriter::new();
    w.write_u16(PROTOCOL_VERSION);
    w.write_u16(client_id);

    // We don't have the raw IP bytes in QUIC the same way, but we can use 0.0.0.0.
    // The client mainly uses this to know its own address.
    let ip_bytes = [0u8, 0, 0, 0];
    w.write_bytes(&ip_bytes);
    w.write_u16(0); // port

    let master_addr = &state.config.node_address;
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
