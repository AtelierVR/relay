//! Per-instance entity registry.
//!
//! The relay manages entities generically: an entity is identified by a `kind` (a player, a prop, a
//! mirror, ...) plus a relay-assigned id. Props are just one kind, and a player *is* an entity:
//! [`Player`](crate::player::Player) embeds [`Entity`] and derefs to it, so every field shared by a
//! player and a networked entity is declared here, once.
//!
//! The entity structure (kind / flags / owner) is carried by the `Entities` packet and the payload
//! by the `Properties` channel. The transform is carried by the `Transform` channel: its last known
//! value is kept per part (rig id) so a `Sync` can replay it, together with the flags telling which
//! fields the client actually sent.

use std::collections::HashMap;
use std::time::SystemTime;

use crate::player::rig::PartTransforms;

/// Kind of a player entity.
pub const KIND_PLAYER: &str = "player";

/// State of a single entity known by the relay.
///
/// A player is an entity: [`Player`](crate::player::Player) embeds this record and derefs to it, so
/// `player.id`, `player.display`, `player.transforms`, ... are these fields.
#[derive(Debug, Clone)]
pub struct Entity {
    /// Unique entity id within the instance.
    pub id: u32,
    /// Entity kind ([`KIND_PLAYER`] for a player, `"prop"`, `"mirror"`, ...).
    pub kind: String,
    /// Entity behaviour flags (unused by a player).
    pub flags: u8,
    /// Id of the player that owns the entity (`0` = pre-placed by the world; a player owns itself).
    pub owner_id: u32,
    /// Optional human-readable name: a player's display name, a prop's label, ...
    pub display: Option<String>,
    /// Unix millisecond timestamp when the entity entered the instance / was created.
    pub created_at: i64,
    /// Last known transform of each part (rig id), together with the flags telling which transform
    /// fields the client actually sent. An entity is multipart: a single-part entity only has the
    /// base part (`0`), a prop may have one entry per part.
    pub transforms: PartTransforms,
}

impl Entity {
    pub fn new() -> Self {
        Self {
            id: 0,
            kind: String::new(),
            flags: 0,
            owner_id: 0,
            display: None,
            created_at: now_ms(),
            transforms: PartTransforms::new(),
        }
    }
}

impl Default for Entity {
    fn default() -> Self {
        Self::new()
    }
}

/// Unix millisecond timestamp of now.
fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// Entity registry of an instance.
///
/// Players and networked entities share the same id space (a player is an entity), so the ids are
/// allocated by the instance (`Instance::next_entity_id`) and stored here as-is.
#[derive(Debug)]
pub struct InstanceEntities {
    entities: HashMap<u32, Entity>,
}

impl Default for InstanceEntities {
    fn default() -> Self {
        Self::new()
    }
}

impl InstanceEntities {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
        }
    }

    /// Iterate over every entity.
    pub fn iter(&self) -> impl Iterator<Item = (&u32, &Entity)> {
        self.entities.iter()
    }

    pub fn get(&self, id: u32) -> Option<&Entity> {
        self.entities.get(&id)
    }

    pub fn get_mut(&mut self, id: u32) -> Option<&mut Entity> {
        self.entities.get_mut(&id)
    }

    /// Whether an entity with the given id exists.
    pub fn contains(&self, id: u32) -> bool {
        self.entities.contains_key(&id)
    }

    /// Insert (or replace) an entity under a given id.
    pub fn insert(&mut self, id: u32, mut data: Entity) {
        data.id = id;
        self.entities.insert(id, data);
    }

    /// Replace the structure (kind / flags / owner) of an existing entity, keeping its transform.
    /// Returns whether the entity existed.
    pub fn update(&mut self, id: u32, kind: String, flags: u8, owner_id: u32) -> bool {
        if let Some(entity) = self.entities.get_mut(&id) {
            entity.kind = kind;
            entity.flags = flags;
            entity.owner_id = owner_id;
            true
        } else {
            false
        }
    }

    pub fn remove(&mut self, id: u32) -> Option<Entity> {
        self.entities.remove(&id)
    }

    pub fn owner_of(&self, id: u32) -> Option<u32> {
        self.entities.get(&id).map(|e| e.owner_id)
    }

    pub fn kind_of(&self, id: u32) -> Option<&str> {
        self.entities.get(&id).map(|e| e.kind.as_str())
    }
}
