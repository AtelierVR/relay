use std::sync::{atomic::AtomicU16, Arc};

use bytes::Bytes;
use parking_lot::Mutex;
use tracing::warn;

use crate::{
    client::{client::ArcClient, ClientManager},
    config::Config,
    instance::{ArcInstance as ArcInst, InstanceManager},
    master::MasterClient,
    utils::log_buffer::LogBuffer,
};

// ─── AppState ────────────────────────────────────────────────────────────────

/// Global application state shared across all connection tasks.
#[derive(Clone)]
pub struct AppState {
    pub clients: Arc<ClientManager>,
    pub instances: Arc<InstanceManager>,
    pub master: Arc<MasterClient>,
    pub config: Arc<Config>,
    pub log_buf: Arc<Mutex<LogBuffer>>,
    next_client_id: Arc<AtomicU16>,
}

impl AppState {
    pub fn new(
        clients: Arc<ClientManager>,
        instances: Arc<InstanceManager>,
        master: Arc<MasterClient>,
        config: Arc<Config>,
        log_buf: Arc<Mutex<LogBuffer>>,
    ) -> Self {
        Self {
            clients,
            instances,
            master,
            config,
            log_buf,
            next_client_id: Arc::new(AtomicU16::new(0)),
        }
    }

    /// Allocate the next unique client ID.
    pub fn next_client_id(&self) -> u16 {
        // Try to find a free ID; fall back to AtomicU16 counter.
        self.clients.next_id()
    }
}

// ─── IncomingPacket ──────────────────────────────────────────────────────────

/// A fully-parsed incoming packet ready for dispatch.
#[derive(Debug, Clone)]
pub struct IncomingPacket {
    pub uid: u16,
    pub packet_type: crate::proto::packet_type::PacketType,
    pub payload: Bytes,
    pub client_id: u16,
}

// ─── Guard helpers ───────────────────────────────────────────────────────────

/// Require the client to have completed the handshake. Returns `None` and logs if not.
pub fn require_handshake(state: &AppState, client_id: u16) -> Option<ArcClient> {
    let arc = state.clients.get(client_id)?;
    if !arc.read().is_handshaked() {
        warn!(
            "[Context] client {}: packet rejected, no handshake",
            client_id
        );
        return None;
    }
    Some(arc)
}

/// Require the client to be fully authenticated.
pub fn require_auth(state: &AppState, client_id: u16) -> Option<ArcClient> {
    let arc = state.clients.get(client_id)?;
    if !arc.read().is_authenticated() {
        warn!(
            "[Context] client {}: packet rejected, not authenticated",
            client_id
        );
        return None;
    }
    Some(arc)
}

/// Require the client to be authenticated and present in the given instance.
pub fn require_instance_player(
    state: &AppState,
    client_id: u16,
    instance_id: u8,
) -> Option<(ArcClient, ArcInst)> {
    let client_arc = require_auth(state, client_id)?;
    let inst_arc = state.instances.get(instance_id)?;
    let player_exists = {
        let inst = inst_arc.read();
        inst.get_players().iter().any(|p| p.client_id == client_id)
    };
    if !player_exists {
        warn!(
            "[Context] client {}: not found in instance {}",
            client_id, instance_id
        );
        return None;
    }
    Some((client_arc, inst_arc))
}

// ─── Broadcast helpers ───────────────────────────────────────────────────────

/// Send `packet` to every client in `instance_id`, optionally excluding one.
pub fn broadcast_instance(state: &AppState, instance_id: u8, packet: Bytes, exclude: Option<u16>) {
    let inst_arc = match state.instances.get(instance_id) {
        Some(a) => a,
        None => return,
    };

    let player_client_ids: Vec<u16> = {
        let inst = inst_arc.read();
        inst.get_players()
            .iter()
            .map(|p| p.client_id)
            .filter(|id| exclude != Some(*id))
            .collect()
    };

    for cid in player_client_ids {
        if let Some(arc) = state.clients.get(cid) {
            let c = arc.read();
            if !c.try_push(packet.clone()) {
                warn!(
                    "[Context] client {}: push channel full, dropping broadcast packet",
                    cid
                );
            }
        }
    }
}

/// Send `packet` only to clients that can see `target_player_id` in the instance.
pub fn broadcast_visible(
    state: &AppState,
    instance_id: u8,
    target_player_id: u16,
    packet: Bytes,
    exclude: Option<u16>,
) {
    let inst_arc = match state.instances.get(instance_id) {
        Some(a) => a,
        None => return,
    };

    let recipients: Vec<u16> = {
        let inst = inst_arc.read();
        inst.get_players()
            .iter()
            .filter(|p| {
                (exclude != Some(p.client_id)) && inst.can_user_see_user(p.id, target_player_id)
            })
            .map(|p| p.client_id)
            .collect()
    };

    for cid in recipients {
        if let Some(arc) = state.clients.get(cid) {
            let c = arc.read();
            if !c.try_push(packet.clone()) {
                warn!(
                    "[Context] client {}: push channel full, dropping visible-broadcast packet",
                    cid
                );
            }
        }
    }
}
