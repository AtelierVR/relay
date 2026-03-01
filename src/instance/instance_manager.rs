use std::sync::Arc;
use dashmap::DashMap;
use parking_lot::RwLock;

use super::instance::{ArcInstance, Instance};

/// Global concurrent registry of instances.
#[derive(Debug, Default)]
pub struct InstanceManager {
    instances: DashMap<u8, ArcInstance>,
}

impl InstanceManager {
    pub fn new() -> Self {
        Self { instances: DashMap::new() }
    }

    /// Insert an instance. Uses `instance.internal_id` as the key.
    pub fn add(&self, instance: Instance) -> ArcInstance {
        let id = instance.internal_id;
        let arc = Arc::new(RwLock::new(instance));
        self.instances.insert(id, Arc::clone(&arc));
        arc
    }

    pub fn remove(&self, id: u8) -> Option<ArcInstance> {
        self.instances.remove(&id).map(|(_, v)| v)
    }

    pub fn get(&self, id: u8) -> Option<ArcInstance> {
        self.instances.get(&id).map(|r| Arc::clone(&r))
    }

    pub fn count(&self) -> usize {
        self.instances.len()
    }

    /// Iterate over all instances.
    pub fn for_each<F: FnMut(u8, &ArcInstance)>(&self, mut f: F) {
        for entry in self.instances.iter() {
            f(*entry.key(), entry.value());
        }
    }

    pub fn all(&self) -> Vec<ArcInstance> {
        self.instances.iter().map(|e| Arc::clone(e.value())).collect()
    }

    /// Find the lowest u8 internal ID not currently in use.
    pub fn next_internal_id(&self) -> u8 {
        for id in 0..u8::MAX {
            if !self.instances.contains_key(&id) {
                return id;
            }
        }
        u8::MAX
    }
}
