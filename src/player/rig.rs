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
    pub position:     Vec3,
    pub rotation:     Quat,
    pub scale:        Vec3,
    pub velocity:     Vec3,
    pub ang_velocity: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position:     Vec3::ZERO,
            rotation:     Quat::IDENTITY,
            scale:        Vec3::ONE,
            velocity:     Vec3::ZERO,
            ang_velocity: Vec3::ZERO,
        }
    }
}
