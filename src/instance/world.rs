use serde::{Deserialize, Serialize};

/// Describes the virtual world that an instance is running inside.
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
        Self::new(0, crate::constants::SAFE_LOCAL_ADDRESS, u16::MAX)
    }
}
