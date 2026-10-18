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
    /// The transfer token was accepted: the client is already authenticated.
    Transferred = 1 << 1,
    /// The response carries the room a client should join when it has no valid target.
    AutoRoom = 1 << 2,
}

/// Room a client should join by default, when it has no valid target of its own.
///
/// A relay hosting a single room always sends clients there (standalone relays, which have nothing
/// to pick). Otherwise the first room flagged `hub` in the config is the default, which lets a relay
/// force its clients onto a public hub.
fn auto_room(state: &AppState) -> Option<u8> {
    let mut ids = Vec::new();
    state.instances.for_each(|id, _| ids.push(id));

    if ids.len() == 1 {
        return ids.first().copied();
    }

    let index = state.config.instances.iter().position(|spec| spec.hub)?;
    let id = u8::try_from(index).ok()?;
    state.instances.get(id).is_some().then_some(id)
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

    let address = r.read_string().unwrap_or_default();
    let port = r.read_u16();

    let engine = r.read_string().unwrap_or_default();
    let platform = r.read_string().unwrap_or_default();

    // Optional trailing transfer token (empty for a normal connection). When the token can be
    // resolved, the client is authenticated immediately and skips the challenge-response below.
    let transfer_token = r.read_bytes_prefixed();
    let transferred = if transfer_token.is_empty() {
        None
    } else {
        match crate::transfer::consume(state, &transfer_token) {
            Some(result) => {
                info!(
                    "[Handshake] client {} transferred as user={}@{} display={} (from={})",
                    client_id,
                    result.user.id,
                    result.user.address,
                    result.user.display_name,
                    result.from
                );
                Some(result)
            }
            None => {
                warn!(
                    "[Handshake] client {}: unresolved transfer token ({} bytes), falling back to authentication",
                    client_id,
                    transfer_token.len()
                );
                None
            }
        }
    };

    info!("[Handshake] client {client_id}: engine=\"{engine}\" platform=\"{platform}\"");

    // Update client state.
    if let Some(arc) = state.clients.get(client_id) {
        let mut c = arc.write();
        c.engine = engine.clone();
        c.platform = platform.clone();
        c.from = format!("{}:{}", address, port);
        c.auth_state = if transferred.is_some() {
            AuthState::Authenticated
        } else {
            AuthState::Handshaked
        };
        if let Some(result) = &transferred {
            c.user = Some(result.user.clone());
        }
    }

    // Notify the master server that a client has connected (platform/engine now known).
    let client_address = state
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
            address: client_address,
            from: format!("{}:{}", address, port),
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
    let auto_room = auto_room(state);
    let mut flags = 0u8;
    if !has_master {
        flags |= HandshakeFlags::IsOffline as u8;
    }
    if transferred.is_some() {
        flags |= HandshakeFlags::Transferred as u8;
    }
    if auto_room.is_some() {
        flags |= HandshakeFlags::AutoRoom as u8;
    }
    w.write_u8(flags);
    if has_master {
        w.write_string(master_addr);
    }

    w.write_u16(crate::constants::MAX_PACKET_SIZE as u16);
    w.write_u16(state.config.connection_timeout);
    w.write_u16(state.config.keep_alive_interval);

    if let Some(iid) = auto_room {
        w.write_u8(iid);
    }

    encode_stream_packet(uid, PacketType::Handshake, w.finish().as_ref())
}
