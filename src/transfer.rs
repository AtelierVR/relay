//! Transfer tokens.
//!
//! A `Transfer` packet tells a client to reconnect to another relay. The token it carries is an
//! opaque, client-unique byte sequence (no shared secret, no format): it is issued elsewhere (the
//! node) and simply relayed to the client, which presents it back in its next handshake.
//!
//! Resolution of a token is not implemented yet: [`consume`] always returns `None`, so a client
//! presenting one simply falls back to the usual challenge-response authentication. It will later
//! return a [`TransferResult`] describing which room and which user the token grants.

use std::time::{Duration, Instant};

use bytes::Bytes;
use tracing::{debug, warn};

use crate::client::client::{ArcClient, PendingTransfer};
use crate::{client::user::User, handlers::context::AppState, proto::buffer::PacketWriter};

/// Operation carried by a `Transfer` packet (`op` byte).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferOp {
    /// Change relay: `[address: string][token: prefixed bytes][reason: string]`.
    Server = 0,
    /// Change room on the same relay: `[room: u8][reason: string]`.
    Room = 1,
}

/// Payload of a relay change (`TransferOp::Server`).
///
/// The token is an opaque, client-unique byte sequence (empty = no handshake credential needed);
/// the target relay picks the room it wants the client to join (see its handshake `auto_room`).
pub fn server_payload(address: &str, token: &[u8], reason: &str) -> Bytes {
    let mut w = PacketWriter::new();
    w.write_u8(TransferOp::Server as u8);
    w.write_string(address);
    w.write_bytes_prefixed(token);
    w.write_string(reason);
    w.finish()
}

/// Payload of a room move (`TransferOp::Room`): the transport and the loaded world are kept, only
/// the room membership is rebuilt.
pub fn room_payload(room: u8, reason: &str) -> Bytes {
    let mut w = PacketWriter::new();
    w.write_u8(TransferOp::Room as u8);
    w.write_u8(room);
    w.write_string(reason);
    w.finish()
}

/// What a transfer token grants once the client presents it in a handshake.
#[derive(Debug, Clone)]
pub struct TransferResult {
    /// User authenticated by the token.
    pub user: User,
    /// Relay the client is coming from.
    pub from: String,
    /// Room the client is expected to join, when the token is bound to one.
    pub room: Option<u8>,
}

/// Validate a transfer token presented in a handshake.
///
/// Always `None` for now: tokens cannot be resolved until the node can tell us which room and
/// which user they belong to.
pub fn consume(_state: &AppState, _token: &[u8]) -> Option<TransferResult> {
    None
}

/// Records a `Transfer` request on the client so the relay can enforce it.
///
/// The packet is only a request: the client owns its connection, so a modified client could simply
/// ignore it. With a pending transfer registered, [`enforce`] removes such a client once the
/// deadline passes, and [`crate::handlers::enter`] refuses it any room other than the target.
///
/// `timeout_secs == 0` disables the enforcement (fire-and-forget request).
pub fn schedule(client: &ArcClient, target: Option<u8>, timeout_secs: u16) {
    if timeout_secs == 0 {
        return;
    }

    let deadline = Instant::now() + Duration::from_secs(timeout_secs as u64);
    let mut c = client.write();
    c.pending_transfer = Some(PendingTransfer { target, deadline });

    debug!(
        "[Transfer] client {} has {timeout_secs}s to {}",
        c.id,
        match target {
            Some(room) => format!("join room {room}"),
            None => String::from("leave this relay"),
        }
    );
}

/// Room the client currently plays in, if any.
fn client_room(state: &AppState, client_id: u16) -> Option<u8> {
    let mut found = None;
    state.instances.for_each(|iid, inst_arc| {
        if found.is_some() {
            return;
        }

        if inst_arc
            .read()
            .get_players()
            .iter()
            .any(|p| p.client_id == client_id)
        {
            found = Some(iid);
        }
    });

    found
}

/// Enforces the pending transfers: a client that did not comply in time is removed from its rooms
/// and disconnected, so an uncooperative client can neither stay where it was nor keep its seat.
///
/// Called on a short interval (see `main.rs`); does nothing when no transfer is pending.
pub fn enforce(state: &AppState) {
    let now = Instant::now();
    let mut done: Vec<u16> = Vec::new();
    let mut expired: Vec<(u16, PendingTransfer)> = Vec::new();

    state.clients.for_each(|id, arc| {
        let c = arc.read();
        let Some(pending) = &c.pending_transfer else {
            return;
        };

        match pending.target {
            // Compliant as soon as it sits in the requested room.
            Some(target) => {
                if client_room(state, id) == Some(target) {
                    done.push(id);
                } else if now >= pending.deadline {
                    expired.push((id, pending.clone()));
                }
            }
            // A relay change is compliant as soon as the client is gone: its record disappears
            // with the connection, so only the stragglers reach this point.
            None => {
                if now >= pending.deadline {
                    expired.push((id, pending.clone()));
                }
            }
        }
    });

    for id in done {
        if let Some(arc) = state.clients.get(id) {
            arc.write().pending_transfer = None;
        }
    }

    for (id, pending) in expired {
        warn!(
            "[Transfer] client {id} did not {} in time, applying it",
            match pending.target {
                Some(room) => format!("move to room {room}"),
                None => String::from("leave this relay"),
            }
        );

        // Drop its seat(s) first: the remaining players must see it leave.
        crate::handlers::quit::leave_all_instances(state, id);

        if let Some(arc) = state.clients.get(id) {
            let mut c = arc.write();
            c.pending_transfer = None;
            // Closing the QUIC connection is the only thing a client cannot ignore.
            c.conn.close(0u32.into(), b"transfer not applied");
        }
    }
}
