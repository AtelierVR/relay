/// Common test fixtures and helpers used across all handler test modules.
use std::sync::Arc;

use bytes::Bytes;
use parking_lot::Mutex;

use crate::{
    client::{client::new_client, ClientManager},
    config::Config,
    constants::DEFAULT_NODE_ADDRESS,
    handlers::context::AppState,
    instance::InstanceManager,
    master::MasterClient,
    proto::{buffer::PacketReader, packet_type::PacketType},
    utils::log_buffer::LogBuffer,
};

/// Build a minimal `AppState` for unit tests.
/// - `use_address` is set to `"127.0.0.1:23032"` so `has_master = true`.
pub fn make_state() -> AppState {
    let config = Arc::new(Config {
        port: 23032,
        node_gateway: String::new(),
        node_address: DEFAULT_NODE_ADDRESS.to_owned(),
        token: String::new(),
        max_instances: 3,
        connection_timeout: 15,
        keep_alive_interval: 5,
        debug: false,
    });
    let clients = Arc::new(ClientManager::new());
    let instances = Arc::new(InstanceManager::new());
    let log_buf = Arc::new(Mutex::new(LogBuffer::new(16)));
    let log_forwarder = Arc::new(Mutex::new(None));
    AppState::new(
        clients.clone(),
        instances.clone(),
        Arc::new(MasterClient::new(
            config.clone(),
            clients,
            instances,
            log_buf.clone(),
            log_forwarder,
        )),
        config,
        log_buf,
    )
}

/// Register a fresh client in `state` and return the push channel receiver.
pub fn register_client(state: &AppState, client_id: u16) -> tokio::sync::mpsc::Receiver<Bytes> {
    let (arc, rx) = new_client(client_id);
    state.clients.add(arc);
    rx
}

/// Mark a registered client as handshaked.
pub fn set_handshaked(state: &AppState, client_id: u16) {
    if let Some(arc) = state.clients.get(client_id) {
        arc.write().auth_state = crate::client::client::AuthState::Handshaked;
    }
}

/// Decode a fully-framed stream packet returned by a handler.
///
/// Wire: `[total: u16 BE][uid: u16 BE][type: u8][payload...]`
/// where `total` is the inclusive total length (= `data.len()`).
///
/// Returns `(uid, PacketType, payload_bytes)`.
pub fn decode_stream(data: Bytes) -> (u16, PacketType, Bytes) {
    assert!(!data.is_empty(), "expected a non-empty response packet");
    let mut r = PacketReader::new(data.clone());
    let total = r.read_u16() as usize;
    assert_eq!(
        total,
        data.len(),
        "stream packet: total-length field mismatch"
    );
    let uid = r.read_u16();
    let type_byte = r.read_u8();
    let ptype = PacketType::try_from(type_byte)
        .unwrap_or_else(|b| panic!("unknown packet type byte 0x{b:02X}"));
    let payload = r.read_remaining();
    (uid, ptype, payload)
}
