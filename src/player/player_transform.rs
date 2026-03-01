use std::collections::HashMap;
use super::rig::Transform;

/// Map from rig-part index (u16) to its current `Transform`.
#[derive(Debug, Clone, Default)]
pub struct PlayerTransforms {
    parts: HashMap<u16, Transform>,
}

impl PlayerTransforms {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, rig_id: u16, transform: Transform) {
        self.parts.insert(rig_id, transform);
    }

    pub fn get(&self, rig_id: u16) -> Option<&Transform> {
        self.parts.get(&rig_id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (u16, &Transform)> {
        self.parts.iter().map(|(k, v)| (*k, v))
    }
}
