use std::ops::{Deref, DerefMut};

use super::{player_flags::PlayerFlags, player_status::PlayerStatus};
use crate::avatar::Avatar;
use crate::instance::entities::{Entity, KIND_PLAYER};

/// A player inside an instance.
///
/// A player is an entity: it embeds the shared [`Entity`] record and derefs to it, so `player.id`,
/// `player.kind`, `player.display`, `player.created_at`, `player.transforms`, ... are the entity
/// fields. Only the player-specific state is declared below.
#[derive(Debug)]
pub struct Player {
    /// Shared entity record.
    pub entity: Entity,
    /// ID of the owning client connection.
    pub client_id: u16,
    /// Internal instance ID.
    pub instance_id: u8,
    /// Player lifecycle status.
    pub status: PlayerStatus,
    /// Role and state flags (the entity behaviour flags live in `entity.flags`).
    pub flags: PlayerFlags,
    /// Currently equipped avatar.
    pub avatar: Option<Avatar>,
    /// Per-player TPS override (0 = use instance default).
    pub custom_tps: u8,
    /// Per-player threshold override. (0.0 = use instance default).
    pub custom_threshold: f32,
}

impl Player {
    pub fn new(id: u32, client_id: u16, instance_id: u8) -> Self {
        let entity = Entity {
            id,
            kind: KIND_PLAYER.to_string(),
            // A player owns itself.
            owner_id: id,
            ..Entity::new()
        };

        Self {
            entity,
            client_id,
            instance_id,
            status: PlayerStatus::None,
            flags: PlayerFlags::NONE,
            avatar: None,
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

impl Deref for Player {
    type Target = Entity;

    fn deref(&self) -> &Entity {
        &self.entity
    }
}

impl DerefMut for Player {
    fn deref_mut(&mut self) -> &mut Entity {
        &mut self.entity
    }
}
