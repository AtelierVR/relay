/// Tests for `handlers::latency`.
///
/// C# spec (LatencyHandler.cs):
/// - Guard: must be handshaked
/// - Request:  [client_timestamp: i64 ms][client_long: i64]
/// - Response: [client_timestamp: i64][server_timestamp: i64][client_long: i64]
use bytes::Bytes;

use crate::{
    handlers::latency,
    proto::{buffer::{PacketReader, PacketWriter}, packet_type::PacketType},
};

use super::helpers::{decode_stream, make_state, register_client, set_handshaked};

// ─── Helper ───────────────────────────────────────────────────────────────────

fn latency_payload(client_ts: i64, client_long: i64) -> Bytes {
    let mut w = PacketWriter::new();
    w.write_i64(client_ts);
    w.write_i64(client_long);
    w.finish()
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[test]
fn non_handshaked_client_rejected() {
    let state = make_state();
    let _rx = register_client(&state, 1);
    // auth_state = None → must return empty
    let resp = latency::handle(&state, 1, 0, latency_payload(123, 456));
    assert!(resp.is_empty(), "non-handshaked client must be rejected");
}

#[test]
fn unknown_client_rejected() {
    let state = make_state();
    // client 99 not registered
    let resp = latency::handle(&state, 99, 0, latency_payload(0, 0));
    assert!(resp.is_empty());
}

#[test]
fn handshaked_client_gets_response() {
    let state = make_state();
    let _rx = register_client(&state, 1);
    set_handshaked(&state, 1);
    let resp = latency::handle(&state, 1, 5, latency_payload(1_700_000_000_000, 42));
    assert!(!resp.is_empty(), "handshaked client must receive a response");
}

#[test]
fn response_packet_type_and_uid() {
    let state = make_state();
    let _rx = register_client(&state, 1);
    set_handshaked(&state, 1);
    let resp = latency::handle(&state, 1, 77, latency_payload(0, 0));
    let (uid, ptype, _) = decode_stream(resp);
    assert_eq!(uid, 77, "UID must be echoed");
    assert_eq!(ptype, PacketType::Latency);
}

#[test]
fn response_echoes_client_timestamp() {
    let state = make_state();
    let _rx = register_client(&state, 1);
    set_handshaked(&state, 1);
    let client_ts = 1_600_000_000_123_i64;
    let resp = latency::handle(&state, 1, 0, latency_payload(client_ts, 0));
    let (_, _, payload) = decode_stream(resp);
    let mut r = PacketReader::new(payload);
    let echoed_ts = r.read_i64();
    assert_eq!(echoed_ts, client_ts, "first field must be the client's timestamp echoed back");
}

#[test]
fn response_server_timestamp_is_positive() {
    let state = make_state();
    let _rx = register_client(&state, 1);
    set_handshaked(&state, 1);
    let resp = latency::handle(&state, 1, 0, latency_payload(0, 0));
    let (_, _, payload) = decode_stream(resp);
    let mut r = PacketReader::new(payload);
    r.skip(8); // skip echoed client_ts
    let server_ts = r.read_i64();
    assert!(server_ts > 0, "server timestamp must be a positive Unix-ms value, got {server_ts}");
}

#[test]
fn response_echoes_client_long() {
    let state = make_state();
    let _rx = register_client(&state, 1);
    set_handshaked(&state, 1);
    let magic: i64 = 0x_DEAD_BEEF_0042_i64;
    let resp = latency::handle(&state, 1, 0, latency_payload(0, magic));
    let (_, _, payload) = decode_stream(resp);
    let mut r = PacketReader::new(payload);
    r.skip(8); // client_ts
    r.skip(8); // server_ts
    let echoed_long = r.read_i64();
    assert_eq!(echoed_long, magic, "third field must be the client long echoed back");
}

#[test]
fn response_payload_is_exactly_24_bytes() {
    // [client_ts: 8][server_ts: 8][client_long: 8] = 24 bytes
    let state = make_state();
    let _rx = register_client(&state, 1);
    set_handshaked(&state, 1);
    let resp = latency::handle(&state, 1, 0, latency_payload(1, 2));
    let (_, _, payload) = decode_stream(resp);
    assert_eq!(payload.len(), 24, "latency payload must be exactly 24 bytes");
}

#[test]
fn response_fields_order_matches_c_sharp() {
    // C#: Write(client_datetime), Write(DateTime.UtcNow), Write(client_long)
    let state = make_state();
    let _rx = register_client(&state, 1);
    set_handshaked(&state, 1);
    let client_ts: i64 = 100_000;
    let client_long: i64 = 200_000;
    let resp = latency::handle(&state, 1, 0, latency_payload(client_ts, client_long));
    let (_, _, payload) = decode_stream(resp);
    let mut r = PacketReader::new(payload);
    // 1) echoed client timestamp
    assert_eq!(r.read_i64(), client_ts,   "field 1: echoed client_ts");
    // 2) server timestamp (just check it's non-zero and reasonable)
    let srv = r.read_i64();
    assert!(srv > 1_000_000_000_000, "field 2: server timestamp must be a recent Unix ms");
    // 3) echoed client long
    assert_eq!(r.read_i64(), client_long, "field 3: echoed client_long");
    assert!(r.is_empty(), "no extra bytes");
}
