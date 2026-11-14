use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Avatar data attached to a player.
/// Wire format (in AvatarChanged): [Id: u32][Server: string][Release: u32][Parameters: ...]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Avatar {
    pub id: u32,
    pub server: String,
    pub release: u32,
    /// Keyed parameter blobs (parameter index → bytes).
    pub parameters: HashMap<i32, Vec<u8>>,
}

impl Avatar {
    /// Release sent on the wire: the pinned release, or `0` when none is pinned (the clients
    /// then resolve the preferred release through their asset API). Only `0` means "auto".
    pub fn release_version(&self) -> u32 {
        self.release
    }
}
