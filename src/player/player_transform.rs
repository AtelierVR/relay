use super::rig::{Transform, TransformFlags};
use std::collections::HashMap;

/// Map from rig-part index (u16) to its current `(flags, Transform)`.
/// `flags` accumulates which fields have ever been set (Reset is stripped before storing).
#[derive(Debug, Clone, Default)]
pub struct PlayerTransforms {
    parts: HashMap<u16, (TransformFlags, Transform)>,
}

impl PlayerTransforms {
    pub fn new() -> Self {
        Self::default()
    }

    /// Store a transform update.
    /// `reset` clears previously accumulated flags (mirrors C# `TransformFlags.Reset` behaviour).
    pub fn set(&mut self, rig_id: u16, flags: TransformFlags, transform: Transform, reset: bool) {
        if reset {
            self.parts.insert(rig_id, (flags, transform));
        } else {
            let entry = self
                .parts
                .entry(rig_id)
                .or_insert((TransformFlags::NONE, Transform::default()));
            entry.0 |= flags;
            entry.1 = transform;
        }
    }

    pub fn get(&self, rig_id: u16) -> Option<Transform> {
        self.parts.get(&rig_id).map(|(_, tr)| *tr)
    }

    /// Iterate `(rig_id, accumulated_flags, transform)` for all stored parts.
    pub fn iter(&self) -> impl Iterator<Item = (u16, TransformFlags, Transform)> + '_ {
        self.parts.iter().map(|(k, (f, tr))| (*k, *f, *tr))
    }
}
