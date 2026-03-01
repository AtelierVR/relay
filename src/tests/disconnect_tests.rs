/// Tests for `handlers::disconnect`.
///
/// C# spec (DisconnectHandler.cs):
/// - Guard: must be handshaked (`IsHandshake`)
/// - Reads optional reason string when `Remaining() > 2`
/// - Calls `client.Disconnect()` → leaves all instances, broadcasts Leave to remaining players
/// - Returns `Bytes::new()` (no response packet)
use bytes::Bytes;

use crate::{
    handlers::disconnect,
    instance::Instance,
    player::{Player, PlayerStatus},
    proto::buffer::PacketWriter,
};

use super::helpers::{make_state, register_client, set_handshaked};

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn reason_payload(reason: &str) -> Bytes {
    let mut w = PacketWriter::new();
    w.write_string(reason);
    w.finish()
}

// ─── Guard tests ─────────────────────────────────────────────────────────────

#[test]
fn non_handshaked_client_rejected() {
    let state = make_state();
    let _rx = register_client(&state, 1);
    // auth_state is still None → must be rejected
    let resp = disconnect::handle(&state, 1, 0, Bytes::new());
    assert!(resp.is_empty(), "non-handshaked client must be rejected");
}

#[test]
fn unknown_client_no_panic() {
    let state = make_state();
    // client 99 was never registered
    let resp = disconnect::handle(&state, 99, 0, Bytes::new());
    assert!(resp.is_empty());
}

// ─── Response format ─────────────────────────────────────────────────────────

#[test]
fn always_returns_empty_bytes() {
    let state = make_state();
    let _rx = register_client(&state, 1);
    set_handshaked(&state, 1);
    let resp = disconnect::handle(&state, 1, 0, Bytes::new());
    assert!(resp.is_empty(), "disconnect handler must not return a packet");
}

#[test]
fn returns_empty_even_with_reason() {
    let state = make_state();
    let _rx = register_client(&state, 1);
    set_handshaked(&state, 1);
    let resp = disconnect::handle(&state, 1, 0, reason_payload("test reason"));
    assert!(resp.is_empty(), "disconnect handler must not return a packet even when reason is present");
}

// ─── Reason parsing ───────────────────────────────────────────────────────────

#[test]
fn empty_payload_does_not_panic() {
    let state = make_state();
    let _rx = register_client(&state, 1);
    set_handshaked(&state, 1);
    // empty payload → no reason read, should not panic
    disconnect::handle(&state, 1, 0, Bytes::new());
}

#[test]
fn short_payload_does_not_panic() {
    let state = make_state();
    let _rx = register_client(&state, 1);
    set_handshaked(&state, 1);
    // 1 byte payload → Remaining() == 1 ≤ 2 → skip reason read, should not panic
    disconnect::handle(&state, 1, 0, Bytes::from_static(b"\x01"));
}

#[test]
fn two_byte_payload_does_not_panic() {
    let state = make_state();
    let _rx = register_client(&state, 1);
    set_handshaked(&state, 1);
    // Exactly 2 bytes → Remaining() == 2, NOT > 2 → skip reason read
    disconnect::handle(&state, 1, 0, Bytes::from_static(b"\x00\x00"));
}

#[test]
fn reason_string_payload_does_not_panic() {
    let state = make_state();
    let _rx = register_client(&state, 1);
    set_handshaked(&state, 1);
    disconnect::handle(&state, 1, 0, reason_payload("leaving due to AFK"));
}

// ─── Instance leave tests ─────────────────────────────────────────────────────

#[test]
fn player_removed_from_instance_after_disconnect() {
    let state = make_state();
    let _rx = register_client(&state, 1);
    set_handshaked(&state, 1);

    // Put client 1 into instance 1 as a ready player.
    let inst = Instance::new(1, 0);
    let arc_inst = state.instances.add(inst);
    {
        let mut inst_w = arc_inst.write();
        let mut p = Player::new(1, 1, 1);
        p.status = PlayerStatus::Ready;
        inst_w.add_player(p);
    }

    assert_eq!(arc_inst.read().get_players().len(), 1, "setup: player should be in instance");

    disconnect::handle(&state, 1, 0, Bytes::new());

    assert_eq!(
        arc_inst.read().get_players().len(), 0,
        "player must be removed from instance after disconnect"
    );
}

#[test]
fn player_removed_from_all_instances_after_disconnect() {
    let state = make_state();
    let _rx = register_client(&state, 5);
    set_handshaked(&state, 5);

    // Add the same client to two different instances.
    let inst_a = Instance::new(10, 0);
    let inst_b = Instance::new(20, 0);
    let arc_a = state.instances.add(inst_a);
    let arc_b = state.instances.add(inst_b);

    {
        let mut w = arc_a.write();
        let mut p = Player::new(1, 5, 10);
        p.status = PlayerStatus::Ready;
        w.add_player(p);
    }
    {
        let mut w = arc_b.write();
        let mut p = Player::new(2, 5, 20);
        p.status = PlayerStatus::Ready;
        w.add_player(p);
    }

    disconnect::handle(&state, 5, 0, Bytes::new());

    assert_eq!(arc_a.read().get_players().len(), 0, "instance A must be empty after disconnect");
    assert_eq!(arc_b.read().get_players().len(), 0, "instance B must be empty after disconnect");
}

#[test]
fn other_players_receive_leave_broadcast() {
    let state = make_state();

    // Leaving client
    let _rx_leaver = register_client(&state, 1);
    set_handshaked(&state, 1);

    // Observer client
    let mut rx_observer = register_client(&state, 2);
    set_handshaked(&state, 2);

    let inst = Instance::new(1, 0);
    let arc_inst = state.instances.add(inst);

    {
        let mut w = arc_inst.write();
        let mut leaver = Player::new(10, 1, 1);
        leaver.status = PlayerStatus::Ready;
        w.add_player(leaver);

        let mut observer = Player::new(20, 2, 1);
        observer.status = PlayerStatus::Ready;
        w.add_player(observer);
    }

    disconnect::handle(&state, 1, 0, Bytes::new());

    // Observer should have received a Leave broadcast.
    match rx_observer.try_recv() {
        Ok(pkt) => assert!(!pkt.is_empty(), "Leave broadcast must not be empty"),
        Err(_) => panic!("observer did not receive Leave broadcast after disconnect"),
    }
}

#[test]
fn disconnect_does_not_affect_other_clients() {
    let state = make_state();

    let _rx1 = register_client(&state, 1);
    set_handshaked(&state, 1);
    let _rx2 = register_client(&state, 2);
    set_handshaked(&state, 2);

    // Put both clients in separate instances — only client 1 disconnects.
    let inst_a = Instance::new(1, 0);
    let inst_b = Instance::new(2, 0);
    let arc_a = state.instances.add(inst_a);
    let arc_b = state.instances.add(inst_b);

    {
        let mut w = arc_a.write();
        let mut p = Player::new(1, 1, 1);
        p.status = PlayerStatus::Ready;
        w.add_player(p);
    }
    {
        let mut w = arc_b.write();
        let mut p = Player::new(2, 2, 2);
        p.status = PlayerStatus::Ready;
        w.add_player(p);
    }

    disconnect::handle(&state, 1, 0, Bytes::new());

    // Instance B's player must be untouched.
    assert_eq!(
        arc_b.read().get_players().len(), 1,
        "client 2's player must not be removed when client 1 disconnects"
    );
}
