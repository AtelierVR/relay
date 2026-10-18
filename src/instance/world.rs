use serde::{Deserialize, Serialize};

/// Describes the virtual world that an instance is running inside,
/// resolved through the master node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct World {
    /// Master-server world ID.
    pub master_id: u32,
    /// Deploy address of the world (e.g. `"nox.example.com"`).
    pub address: String,
    /// World version number.
    pub version: u16,
}

impl World {
    pub fn new(master_id: u32, address: impl Into<String>, version: u16) -> Self {
        Self {
            master_id,
            address: address.into(),
            version,
        }
    }

    /// `"{master_id}?v={version}@{address}"`
    pub fn to_identifier(&self) -> String {
        format!("{}?v={}@{}", self.master_id, self.version, self.address)
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new(0, crate::constants::SAFE_LOCAL_ADDRESS, 0)
    }
}

/// Standalone world bundle served to clients through the `UseBundle` travel path
/// (no master node to resolve the world from).
#[derive(Debug, Clone)]
pub struct WorldBundle {
    /// Bundle download URL.
    pub url: String,
    /// 32-byte bundle hash.
    pub hash: Vec<u8>,
    /// Bundle size in bytes (0 = unknown).
    pub size: u32,
}

/// Where the clients of an instance must travel to: either a world resolved through
/// their master node (`UseNode`) or a self-hosted bundle they download themselves
/// (`UseBundle`).
#[derive(Debug, Clone)]
pub enum TravelTarget {
    /// Master world the client resolves through its node.
    Node(World),
    /// Self-hosted bundle the client downloads itself.
    Bundle(WorldBundle),
}

impl Default for TravelTarget {
    fn default() -> Self {
        Self::Node(World::default())
    }
}

impl TravelTarget {
    /// `"{master_id}?v={version}@{address}"` for a node world, or the bundle URL for a bundle.
    pub fn to_identifier(&self) -> String {
        match self {
            TravelTarget::Node(world) => world.to_identifier(),
            TravelTarget::Bundle(bundle) => bundle.url.clone(),
        }
    }
}
