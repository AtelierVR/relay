//! Ownership arbitration of the networked entities.
//!
//! The ownership is a single `Ownership` operation refined by a sub-operation:
//!
//! * `Change` (client → relay): `[eid][pid]`, `pid = 0` meaning the sender itself.
//!   * a privileged player changes the ownership straight away: everybody (him included) receives
//!     `Changed`;
//!   * anybody else only asks: the request gets an id and goes to the privileged players (and back
//!     to the requester as an acknowledgement) as `Requested [rid]`, or the requester receives an
//!     `Error` when the request cannot be made.
//! * `Requested` (relay → privileged players + requester): `[rid][eid][requester]`.
//! * `Changed` (relay → every player): `[eid][owner]`.
//! * `Refuse` (privileged player → relay): `[rid]`; the requester then receives an `Error`.
//!   A privileged player grants a request with a `Change` naming the requester as `pid`.
//!
//! A request is dropped when its requester leaves, when its entity is removed, and when nobody
//! answers it in time. Granting one entity refuses every other request pending on it.

use std::sync::Arc;
use std::time::Duration;

use crate::handlers::context::AppState;
use crate::instance::Entity;
use crate::proto::buffer::PacketReader;
use tracing::{debug, warn};

use super::{
    broadcast, build, error_bytes, fail, push_to, push_to_player, resolve, send_to_privileged,
    Event, OP_OWNERSHIP, RESULT_DENIED, RESULT_NOT_FOUND, RESULT_OK, RESULT_OVERFLOW,
};

/// Nobody answered the request in time.
const RESULT_TIMEOUT: u8 = 4;

/// Sub-operation of `OP_OWNERSHIP`.
const OW_CHANGE: u8 = 0;
const OW_CHANGED: u8 = 1;
const OW_REQUESTED: u8 = 2;
const OW_REFUSE: u8 = 3;

/// Handle the `Ownership` operation and its sub-operation.
pub(super) fn on_ownership(
    state: &Arc<AppState>,
    inst_arc: &crate::instance::ArcInstance,
    r: &mut PacketReader,
    iid: u8,
    client_id: u16,
    self_player_id: u32,
    privileged: bool,
) {
    let sub = r.read_u8();

    match sub {
        OW_CHANGE => change(
            state,
            inst_arc,
            r,
            iid,
            client_id,
            self_player_id,
            privileged,
        ),
        OW_REFUSE => refuse(
            state,
            inst_arc,
            r,
            iid,
            client_id,
            self_player_id,
            privileged,
        ),
        other => debug!("[Entities] unknown ownership sub-op {}", other),
    }
}

/// `Change [eid][pid]`: a privileged player changes the ownership, anybody else asks for it.
fn change(
    state: &Arc<AppState>,
    inst_arc: &crate::instance::ArcInstance,
    r: &mut PacketReader,
    iid: u8,
    client_id: u16,
    self_player_id: u32,
    privileged: bool,
) {
    // `pid = 0` is the sender itself.
    let eid = r.read_u32();
    let pid = r.read_u32();
    let owner = if pid == 0 { self_player_id } else { pid };

    let data = match resolve(inst_arc, eid) {
        Some(d) => d,
        None => {
            fail(state, client_id, iid, RESULT_NOT_FOUND, eid);
            return;
        }
    };

    if !privileged {
        // A regular player can only ask for itself, and only for an entity it does not already own:
        // the privileged players decide.
        if owner != self_player_id || data.owner_id == self_player_id {
            debug!(
                "[Entities] player {} cannot change the ownership of entity {}",
                self_player_id, eid
            );
            return;
        }

        request(
            state,
            inst_arc,
            Request {
                iid,
                client_id,
                requester: self_player_id,
                receiver: owner,
                entity: eid,
            },
            &data,
        );
        return;
    }

    grant(state, inst_arc, iid, client_id, self_player_id, eid, owner);
}

/// An ownership request on its way to the privileged players.
#[derive(Clone, Copy)]
struct Request {
    iid: u8,
    client_id: u16,
    requester: u32,
    receiver: u32,
    entity: u32,
}

/// Register the request of a regular player and forward it to the privileged players.
fn request(
    state: &Arc<AppState>,
    inst_arc: &crate::instance::ArcInstance,
    req: Request,
    data: &Entity,
) {
    // A request lives as long as a client is allowed to wait for its room.
    let timeout = Duration::from_secs(state.config.connection_timeout.max(1) as u64);

    let rid = match inst_arc.write().push_ownership_request(
        req.entity,
        req.requester,
        req.receiver,
        timeout,
    ) {
        Some(rid) => rid,
        None => {
            warn!(
                "[Entities] too many ownership requests pending on entity {}",
                req.entity
            );
            fail(state, req.client_id, req.iid, RESULT_OVERFLOW, req.entity);
            return;
        }
    };

    // The requests are expired by the tick of the instance, started with the first one.
    start_tick(state, req.iid);

    let requested = build(&Event {
        iid: req.iid,
        op: OP_OWNERSHIP,
        result: RESULT_OK,
        sub: OW_REQUESTED,
        id: req.entity,
        sender: req.requester,
        request: rid,
        entity: data,
    });

    // The requester learns the id of its request, the privileged players may answer it.
    push_to(state, req.client_id, requested.clone());
    send_to_privileged(state, inst_arc, requested);
}

/// Run the tick of the instance that expires the ownership requests, once per instance.
///
/// The task ticks at the instance rate and stops itself as soon as no request is pending: an
/// instance with nothing to expire does not keep a timer alive.
fn start_tick(state: &Arc<AppState>, iid: u8) {
    let Some(inst_arc) = state.instances.get(iid) else {
        return;
    };

    {
        let mut inst = inst_arc.write();

        if inst.ownership_ticking {
            return;
        }

        inst.ownership_ticking = true;
    }

    let state = Arc::clone(state);

    tokio::spawn(async move {
        loop {
            let tps = match state.instances.get(iid) {
                Some(arc) => arc.read().tps.max(1) as u64,
                None => break,
            };

            tokio::time::sleep(Duration::from_millis(1000 / tps)).await;

            expire_ownership_requests(&state, iid);

            // Nothing left to expire: the instance stops its tick.
            let done = match state.instances.get(iid) {
                Some(arc) => {
                    let mut inst = arc.write();
                    let done = inst.pending_ownership.is_empty();

                    if done {
                        inst.ownership_ticking = false;
                    }

                    done
                }
                None => true,
            };

            if done {
                debug!("[Entities] ownership tick of instance {} stopped", iid);
                break;
            }
        }
    });
}

/// Give the entity to `owner`, and refuse every other request pending on it.
fn grant(
    state: &AppState,
    inst_arc: &crate::instance::ArcInstance,
    iid: u8,
    client_id: u16,
    self_player_id: u32,
    eid: u32,
    owner: u32,
) {
    let (updated, refused) = {
        let mut inst = inst_arc.write();

        let mut refused = Vec::new();
        inst.pending_ownership.retain(|_, pending| {
            if pending.entity == eid {
                refused.push(pending.requester);
                false
            } else {
                true
            }
        });

        let updated = inst.entities.get_mut(eid).map(|entity| {
            entity.owner_id = owner;
            entity.clone()
        });

        (updated, refused)
    };

    let Some(data) = updated else {
        fail(state, client_id, iid, RESULT_NOT_FOUND, eid);
        return;
    };

    broadcast(
        state,
        iid,
        build(&Event {
            iid,
            op: OP_OWNERSHIP,
            result: RESULT_OK,
            sub: OW_CHANGED,
            id: eid,
            sender: self_player_id,
            request: 0,
            entity: &data,
        }),
    );

    // Only one request can be granted: the others lose.
    for requester in refused {
        if requester == owner {
            continue;
        }

        let bytes = error_bytes(iid, RESULT_DENIED, eid);
        push_to_player(state, inst_arc, requester, bytes);
    }
}

/// `Refuse [rid]`: a privileged player rejects a request, its requester receives an error.
fn refuse(
    state: &AppState,
    inst_arc: &crate::instance::ArcInstance,
    r: &mut PacketReader,
    iid: u8,
    client_id: u16,
    self_player_id: u32,
    privileged: bool,
) {
    let rid = r.read_u16();

    if !privileged {
        fail(state, client_id, iid, RESULT_DENIED, 0);
        return;
    }

    let pending = inst_arc.write().pending_ownership.remove(&rid);

    let Some(pending) = pending else {
        fail(state, client_id, iid, RESULT_NOT_FOUND, 0);
        return;
    };

    debug!(
        "[Entities] ownership of {} refused to {} by {}",
        pending.entity, pending.requester, self_player_id
    );

    // A refusal reaches the requester as an error: the ownership does not change.
    let bytes = error_bytes(iid, RESULT_DENIED, pending.entity);
    push_to_player(state, inst_arc, pending.requester, bytes);
}

/// Reassign to the instance master every entity whose owner is gone (the owner left or
/// disconnected), and tell the room about it.
pub fn reassign_ownership(state: &AppState, iid: u8) {
    let inst_arc = match state.instances.get(iid) {
        Some(a) => a,
        None => return,
    };

    let updates: Vec<(u32, Entity)> = {
        let mut inst = inst_arc.write();

        let master = match inst.get_master() {
            Some(p) => p.id,
            None => return,
        };

        let present: Vec<u32> = inst.get_players().iter().map(|p| p.id).collect();
        let entities: Vec<u32> = inst.entities.iter().map(|(id, _)| *id).collect();

        // Requests left by a player that is gone, or on an entity that is gone, have no outcome.
        inst.pending_ownership.retain(|_, pending| {
            present.contains(&pending.requester) && entities.contains(&pending.entity)
        });

        let orphans: Vec<u32> = inst
            .entities
            .iter()
            .filter(|(_, e)| !present.contains(&e.owner_id))
            .map(|(id, _)| *id)
            .collect();

        let mut updates = Vec::with_capacity(orphans.len());
        for id in orphans {
            if let Some(entity) = inst.entities.get_mut(id) {
                entity.owner_id = master;
                updates.push((id, entity.clone()));
            }
        }

        updates
    };

    for (id, data) in updates {
        debug!("[Entities] entity {} falls back to its master", id);
        broadcast(
            state,
            iid,
            build(&Event {
                iid,
                op: OP_OWNERSHIP,
                result: RESULT_OK,
                sub: OW_CHANGED,
                id,
                sender: data.owner_id,
                request: 0,
                entity: &data,
            }),
        );
    }
}

/// Reject the ownership requests that nobody answered in time, and tell their requester.
fn expire_ownership_requests(state: &Arc<AppState>, iid: u8) {
    let inst_arc = match state.instances.get(iid) {
        Some(a) => a,
        None => return,
    };

    let expired = {
        let mut inst = inst_arc.write();
        inst.expired_ownership_requests()
    };

    for (rid, pending) in expired {
        debug!(
            "[Entities] ownership request {} of entity {} timed out",
            rid, pending.entity
        );

        let bytes = error_bytes(iid, RESULT_TIMEOUT, pending.entity);
        push_to_player(state, &inst_arc, pending.requester, bytes);
    }
}
