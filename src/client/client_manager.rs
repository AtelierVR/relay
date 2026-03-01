use std::sync::Arc;
use dashmap::DashMap;
use parking_lot::RwLock;

use super::client::{ArcClient, Client};

/// Global concurrent registry of connected clients.
#[derive(Debug, Default)]
pub struct ClientManager {
    clients: DashMap<u16, ArcClient>,
}

impl ClientManager {
    pub fn new() -> Self {
        Self { clients: DashMap::new() }
    }

    /// Insert a client. The client's `id` is used as the key.
    pub fn add(&self, client: ArcClient) {
        let id = client.read().id;
        self.clients.insert(id, client);
    }

    /// Remove and return a client by id.
    pub fn remove(&self, id: u16) -> Option<ArcClient> {
        self.clients.remove(&id).map(|(_, v)| v)
    }

    /// Look up a client by id.
    pub fn get(&self, id: u16) -> Option<ArcClient> {
        self.clients.get(&id).map(|r| Arc::clone(&r))
    }

    /// Iterate over all connected clients.
    pub fn for_each<F: FnMut(u16, &ArcClient)>(&self, mut f: F) {
        for entry in self.clients.iter() {
            f(*entry.key(), entry.value());
        }
    }

    pub fn count(&self) -> usize {
        self.clients.len()
    }

    /// Find the lowest u16 ID not currently in use, starting from 0.
    pub fn next_id(&self) -> u16 {
        for id in 0..u16::MAX {
            if !self.clients.contains_key(&id) {
                return id;
            }
        }
        u16::MAX
    }
}
