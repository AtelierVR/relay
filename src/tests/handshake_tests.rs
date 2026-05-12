/// Tests for `handlers::handshake`.
///
/// C# spec (HandshakeHandler.cs):
/// - Guard: none (any state accepted)
/// - Request:  [protocol: u16][engine: string][platform: string]
/// - Version mismatch → no response (Bytes::new())
/// - Response: [protocol: u16][client_id: u16][ip: 4 bytes][port: u16]
///             [flags: u8](?[master: string]) [max_pkt: u16]
///             [conn_timeout: u16][keepalive: u16][seg_timeout: u16]
/// - Sets client.auth_state = Handshaked
use bytes::Bytes;

use crate::{
    client::client::AuthState,
    constants::PROTOCOL_VERSION,
    handlers::handshake,
    proto::{buffer::PacketWriter, packet_type::PacketType},
};

use super::helpers::{decode_stream, make_state, register_client};

// ─── Helper ───────────────────────────────────────────────────────────────────

/// Build a valid handshake request payload.
fn handshake_payload(protocol: u16, engine: &str, platform: &str) -> Bytes {
    let mut w = PacketWriter::new();
    w.write_u16(protocol);
    w.write_string(engine);
    w.write_string(platform);
    w.finish()
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[test]
fn wrong_protocol_returns_empty() {
    let state = make_state();
    let _rx = register_client(&state, 1);
    let response = handshake::handle_inner(&state, 1, 99, handshake_payload(0xFFFF, "Unity", "PC"));
    assert!(
        response.is_empty(),
        "incompatible protocol must return empty Bytes"
    );
}

#[test]
fn correct_protocol_returns_response() {
    let state = make_state();
    let _rx = register_client(&state, 1);
    let response = handshake::handle_inner(
        &state,
        1,
        1,
        handshake_payload(PROTOCOL_VERSION, "Unity", "PC"),
    );
    assert!(
        !response.is_empty(),
        "valid handshake must return a response"
    );
}

#[test]
fn response_is_handshake_packet_type() {
    let state = make_state();
    let _rx = register_client(&state, 1);
    let response = handshake::handle_inner(
        &state,
        1,
        7,
        handshake_payload(PROTOCOL_VERSION, "Unity", "PC"),
    );
    let (uid, ptype, _) = decode_stream(response);
    assert_eq!(uid, 7, "UID must be echoed back");
    assert_eq!(ptype, PacketType::Handshake);
}

#[test]
fn response_payload_matches_c_sharp_format() {
    use crate::proto::buffer::PacketReader;

    let state = make_state();
    let _rx = register_client(&state, 42);
    let response = handshake::handle_inner(
        &state,
        42,
        1,
        handshake_payload(PROTOCOL_VERSION, "Unreal", "Quest"),
    );
    let (_uid, _ptype, payload) = decode_stream(response);

    let mut r = PacketReader::new(payload);

    // [protocol: u16]
    assert_eq!(r.read_u16(), PROTOCOL_VERSION, "protocol version");
    // [client_id: u16]
    assert_eq!(r.read_u16(), 42, "client id must be 42");
    // [ip: 4 bytes] — always 0.0.0.0 in QUIC path
    let ip = r.read_bytes(4);
    assert_eq!(ip, &[0, 0, 0, 0], "IP bytes");
    // [port: u16]
    assert_eq!(r.read_u16(), 0, "port");
    // [flags: u8] — 0 = None (has_master = true because use_address is set)
    let flags = r.read_u8();
    assert_eq!(flags, 0, "flags should be None (master address is set)");
    // [master: string] — written when has_master
    let master = r.read_string().expect("master address string");
    assert!(!master.is_empty(), "master address must not be empty");
    // [max_packet_size: u16]
    assert_eq!(r.read_u16(), crate::constants::MAX_PACKET_SIZE as u16);
    // [connection_timeout: u16]
    assert_eq!(r.read_u16(), 15);
    // [keep_alive_interval: u16]
    assert_eq!(r.read_u16(), 5);
    // Nothing left
    assert!(r.is_empty(), "no extra bytes in response");
}

#[test]
fn sets_client_auth_state_to_handshaked() {
    let state = make_state();
    let _rx = register_client(&state, 5);
    // Before: None
    assert_eq!(
        state.clients.get(5).unwrap().read().auth_state,
        AuthState::None
    );
    handshake::handle_inner(
        &state,
        5,
        0,
        handshake_payload(PROTOCOL_VERSION, "Unity", "PC"),
    );
    // After: Handshaked
    assert_eq!(
        state.clients.get(5).unwrap().read().auth_state,
        AuthState::Handshaked
    );
}

#[test]
fn stores_engine_and_platform() {
    let state = make_state();
    let _rx = register_client(&state, 10);
    handshake::handle_inner(
        &state,
        10,
        0,
        handshake_payload(PROTOCOL_VERSION, "GodotEngine", "Android"),
    );
    let arc = state.clients.get(10).unwrap();
    let c = arc.read();
    assert_eq!(c.engine, "GodotEngine");
    assert_eq!(c.platform, "Android");
}

#[test]
fn response_uid_matches_request_uid() {
    let state = make_state();
    let _rx = register_client(&state, 1);
    for uid in [0_u16, 1, 100, u16::MAX] {
        let resp = handshake::handle_inner(
            &state,
            1,
            uid,
            handshake_payload(PROTOCOL_VERSION, "u", "p"),
        );
        if !resp.is_empty() {
            let (resp_uid, _, _) = decode_stream(resp);
            assert_eq!(resp_uid, uid);
        }
        // reset auth state for next iteration
        if let Some(arc) = state.clients.get(1) {
            arc.write().auth_state = AuthState::None;
        }
    }
}

#[test]
fn wrong_protocol_does_not_mutate_auth_state() {
    let state = make_state();
    let _rx = register_client(&state, 3);
    handshake::handle_inner(&state, 3, 0, handshake_payload(0x9999, "X", "Y"));
    assert_eq!(
        state.clients.get(3).unwrap().read().auth_state,
        AuthState::None,
        "auth state must remain None after protocol mismatch"
    );
}
