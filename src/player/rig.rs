use std::collections::HashMap;

use bitflags::bitflags;
use glam::{Quat, Vec3};

bitflags! {
    /// Bitmask indicating which transform fields are present in a Transform packet.
    /// Matches C# `TransformFlags : byte`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct TransformFlags: u8 {
        const NONE           = 0;
        const POSITION       = 1 << 0;
        const ROTATION       = 1 << 1;
        const SCALE          = 1 << 2;
        const VELOCITY       = 1 << 3;
        const ANG_VELOCITY   = 1 << 4;
        const RESET          = 1 << 5;
    }
}

impl TransformFlags {
    pub const ALL: TransformFlags = TransformFlags::from_bits_truncate(0x1F);
    pub const RIGIDBODY: TransformFlags = TransformFlags::from_bits_truncate(0b0001_1000);
    pub const TRANSFORM: TransformFlags = TransformFlags::from_bits_truncate(0b0000_0111);
}

/// Full spatial transform for one rig part.
#[derive(Debug, Clone, Copy)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    pub velocity: Vec3,
    pub ang_velocity: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            velocity: Vec3::ZERO,
            ang_velocity: Vec3::ZERO,
        }
    }
}

/// Map from rig-part index (`u16`) to its current `(flags, Transform)`.
///
/// `flags` accumulates which transform fields have ever been set (some clients only send a part of
/// the transform, e.g. the position without the rotation); `RESET` is stripped before storing.
/// Used for a player's rig parts and for a networked entity's parts alike.
#[derive(Debug, Clone, Default)]
pub struct PartTransforms {
    parts: HashMap<u16, (TransformFlags, Transform)>,
}

impl PartTransforms {
    pub fn new() -> Self {
        Self::default()
    }

    /// Store a transform update of one part.
    /// `reset` clears previously accumulated flags (mirrors C# `TransformFlags.Reset` behaviour).
    pub fn set(&mut self, rig: u16, flags: TransformFlags, transform: Transform, reset: bool) {
        if reset {
            self.parts.insert(rig, (flags, transform));
        } else {
            let entry = self
                .parts
                .entry(rig)
                .or_insert((TransformFlags::NONE, Transform::default()));
            entry.0 |= flags;
            entry.1 = transform;
        }
    }

    /// Current transform of a part (its fields that were never sent keep their identity value).
    pub fn get(&self, rig: u16) -> Option<Transform> {
        self.parts.get(&rig).map(|(_, tr)| *tr)
    }

    /// Accumulated flags of a part: which transform fields are known.
    pub fn flags(&self, rig: u16) -> TransformFlags {
        self.parts
            .get(&rig)
            .map(|(flags, _)| *flags)
            .unwrap_or(TransformFlags::NONE)
    }

    /// Forget a part.
    pub fn remove(&mut self, rig: u16) -> bool {
        self.parts.remove(&rig).is_some()
    }

    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }

    pub fn len(&self) -> usize {
        self.parts.len()
    }

    /// Iterate `(rig_id, accumulated_flags, transform)` for all stored parts.
    pub fn iter(&self) -> impl Iterator<Item = (u16, TransformFlags, Transform)> + '_ {
        self.parts.iter().map(|(k, (f, tr))| (*k, *f, *tr))
    }
}
