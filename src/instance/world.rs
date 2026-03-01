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
        Self { master_id, address: address.into(), version }
    }

    /// `"{master_id}@{address}:{version}"`
    pub fn to_identifier(&self) -> String {
        format!("{}@{}:{}", self.master_id, self.address, self.version)
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new(0, "::", u16::MAX)
    }
}
