use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Avatar data attached to a player.
/// Wire format (in AvatarChanged): [Id: u32][Server: string][Version: u16][Parameters: ...]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Avatar {
    pub id: u32,
    pub server: String,
    pub version: u16,
    /// Keyed parameter blobs (parameter index → bytes).
    pub parameters: HashMap<i32, Vec<u8>>,
}

impl Default for Avatar {
    fn default() -> Self {
        Self {
            id: 0,
            server: String::new(),
            version: u16::MAX,
            parameters: HashMap::new(),
        }
    }
}
