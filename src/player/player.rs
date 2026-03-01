use std::time::SystemTime;

use super::{
    player_flags::PlayerFlags, player_status::PlayerStatus, player_transform::PlayerTransforms,
};
use crate::avatar::Avatar;

/// A player inside an instance.
#[derive(Debug)]
pub struct Player {
    /// Relay-assigned player ID (unique within the instance).
    pub id: u16,
    /// ID of the owning client connection.
    pub client_id: u16,
    /// Internal instance ID.
    pub instance_id: u8,
    /// Player lifecycle status.
    pub status: PlayerStatus,
    /// Optional custom display name (overrides user.display_name).
    pub display: Option<String>,
    /// Unix millisecond timestamp when the player entered the instance.
    pub created_at: i64,
    /// Role and state flags.
    pub flags: PlayerFlags,
    /// Currently equipped avatar.
    pub avatar: Option<Avatar>,
    /// Per-rig-part transforms.
    pub transforms: PlayerTransforms,
    /// Per-player TPS override (0 = use instance default).
    pub custom_tps: u8,
    /// Per-player threshold override.
    pub custom_threshold: f32,
}

impl Player {
    pub fn new(id: u16, client_id: u16, instance_id: u8) -> Self {
        let now_ms = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        Self {
            id,
            client_id,
            instance_id,
            status: PlayerStatus::None,
            display: None,
            created_at: now_ms,
            flags: PlayerFlags::NONE,
            avatar: None,
            transforms: PlayerTransforms::new(),
            custom_tps: 0,
            custom_threshold: 0.0,
        }
    }

    pub fn is_master(&self) -> bool {
        self.flags.contains(PlayerFlags::INSTANCE_MASTER)
    }

    pub fn has_privilege(&self) -> bool {
        self.flags.intersects(PlayerFlags::HAS_PRIVILEGE)
    }

    pub fn has_high_privilege(&self) -> bool {
        self.flags.intersects(PlayerFlags::HAS_HIGH_PRIVILEGE)
    }

    pub fn is_ready(&self) -> bool {
        self.status == PlayerStatus::Ready
    }
}
