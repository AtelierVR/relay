use std::collections::HashSet;

/// Manages visibility relationships between players within an instance.
///
/// - Auto-groups: `id <= u16::MAX`, one per player, created automatically on join.
/// - Custom groups: `id > u16::MAX`, created by the instance/server logic.
#[derive(Debug, Clone)]
pub struct ViewGroup {
    /// Unique group ID within the instance.
    pub id: u32,
    /// Optional human-readable name.
    pub name: Option<String>,
    /// Player IDs that are members of this group.
    pub members: HashSet<u16>,
    /// IDs of groups whose members this group can see.
    pub visible_groups: HashSet<u32>,
}

impl ViewGroup {
    pub fn new(id: u32, name: Option<String>) -> Self {
        Self {
            id,
            name,
            members: HashSet::new(),
            visible_groups: HashSet::new(),
        }
    }

    pub fn is_auto_group(&self) -> bool {
        self.id <= u32::from(u16::MAX)
    }

    pub fn is_custom_group(&self) -> bool {
        !self.is_auto_group()
    }

    pub fn add_member(&mut self, player_id: u16) -> bool {
        self.members.insert(player_id)
    }

    pub fn remove_member(&mut self, player_id: u16) -> bool {
        self.members.remove(&player_id)
    }

    pub fn is_member(&self, player_id: u16) -> bool {
        self.members.contains(&player_id)
    }

    pub fn add_visible_group(&mut self, group_id: u32) -> bool {
        self.visible_groups.insert(group_id)
    }

    pub fn remove_visible_group(&mut self, group_id: u32) -> bool {
        self.visible_groups.remove(&group_id)
    }

    /// Returns `true` if this group can see `target_group_id`.
    pub fn can_see_group(&self, target_group_id: u32) -> bool {
        self.visible_groups.contains(&target_group_id)
    }
}
