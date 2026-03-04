use parking_lot::RwLock;
use std::sync::Arc;

use crate::constants::{DEFAULT_RENDER_ENTITY, DEFAULT_THRESHOLD, DEFAULT_TPS};
use crate::player::Player;
use crate::utils::hashing::verify_password;

use super::{
    flags::InstanceFlags, user_moderated::UserModerated, view_group::ViewGroup, world::World,
};

/// A relay-hosted instance (a "room" players can enter).
#[derive(Debug)]
pub struct Instance {
    /// Relay-internal unique ID (0–254; 255 = invalid/unassigned).
    pub internal_id: u8,
    /// Master-server assigned instance ID.
    pub master_id: u32,
    /// Feature flags.
    pub flags: InstanceFlags,
    /// Maximum player count (0 = unlimited).
    pub capacity: u16,
    /// Target ticks-per-second for this instance.
    pub tps: u8,
    /// Transform equality threshold (f32 delta).
    pub threshold: f32,
    /// Maximum entity render distance.
    pub render_entity: f32,
    /// The virtual world this instance belongs to.
    pub world: World,
    /// Argon2-hashed password (only set when `flags` has `USE_PASSWORD`).
    password_hash: Option<String>,
    /// Players currently in this instance.
    players: Vec<Player>,
    /// View-visibility groups.
    view_groups: Vec<ViewGroup>,
    /// Moderation records.
    moderated: Vec<UserModerated>,
    /// Counter for custom view-group IDs (starts at `u32::from(u16::MAX) + 1`).
    next_view_group_id: u32,
    /// Duration of the last tick in milliseconds (for load balancing).
    pub last_tick_duration: f32,
    /// Effective TPS calculated by load balancing (None = not yet calculated).
    effective_tps: Option<u8>,
    /// Effective threshold calculated by load balancing (None = not yet calculated).
    effective_threshold: Option<f32>,
    /// Load balancing enabled override (None = use global config).
    pub load_balancing_enabled: Option<bool>,
}

impl Instance {
    pub fn new(internal_id: u8, master_id: u32) -> Self {
        Self {
            internal_id,
            master_id,
            flags: InstanceFlags::NONE,
            capacity: 0,
            tps: DEFAULT_TPS,
            threshold: DEFAULT_THRESHOLD,
            render_entity: DEFAULT_RENDER_ENTITY,
            world: World::default(),
            password_hash: None,
            players: Vec::new(),
            view_groups: Vec::new(),
            moderated: Vec::new(),
            next_view_group_id: u32::from(u16::MAX) + 1,
            last_tick_duration: 0.0,
            effective_tps: None,
            effective_threshold: None,
            load_balancing_enabled: None,
        }
    }

    // ── Password ───────────────────────────────────────────────────────────

    pub fn set_password(&mut self, raw: Option<String>) {
        match raw {
            None => {
                self.password_hash = None;
                self.flags.remove(InstanceFlags::USE_PASSWORD);
            }
            Some(p) => {
                let hash = crate::utils::hashing::hash_password(&p).expect("argon2 hash failed");
                self.password_hash = Some(hash);
                self.flags.insert(InstanceFlags::USE_PASSWORD);
            }
        }
    }

    pub fn verify_password(&self, raw: &str) -> bool {
        match &self.password_hash {
            None => true,
            Some(hash) => verify_password(raw, hash),
        }
    }

    pub fn has_password(&self) -> bool {
        self.flags.contains(InstanceFlags::USE_PASSWORD)
    }

    // ── Players ────────────────────────────────────────────────────────────

    pub fn add_player(&mut self, player: Player) {
        self.get_or_create_view_group(player.id);
        self.players.push(player);
    }

    pub fn remove_player(&mut self, player_id: u16) -> Option<Player> {
        if let Some(pos) = self.players.iter().position(|p| p.id == player_id) {
            let player = self.players.remove(pos);
            self.cleanup_user(player_id);
            Some(player)
        } else {
            None
        }
    }

    pub fn get_players(&self) -> &[Player] {
        &self.players
    }

    pub fn get_players_mut(&mut self) -> &mut Vec<Player> {
        &mut self.players
    }

    pub fn get_player(&self, player_id: u16) -> Option<&Player> {
        self.players.iter().find(|p| p.id == player_id)
    }

    pub fn get_player_mut(&mut self, player_id: u16) -> Option<&mut Player> {
        self.players.iter_mut().find(|p| p.id == player_id)
    }

    pub fn get_master(&self) -> Option<&Player> {
        self.players.iter().find(|p| {
            p.flags
                .contains(crate::player::PlayerFlags::INSTANCE_MASTER)
        })
    }

    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    pub fn is_full(&self) -> bool {
        if self.flags.contains(InstanceFlags::ALLOW_OVERLOAD) {
            return false;
        }
        self.capacity != 0 && self.players.len() >= self.capacity as usize
    }

    /// Find the lowest u16 player ID not currently in use (starting from 0).
    pub fn next_player_id(&self) -> u16 {
        for id in 0..u16::MAX {
            if !self.players.iter().any(|p| p.id == id) {
                return id;
            }
        }
        u16::MAX
    }

    // ── ViewGroup ──────────────────────────────────────────────────────────

    /// Get or create the automatic view group for a player.
    pub fn get_or_create_view_group(&mut self, player_id: u16) -> &mut ViewGroup {
        let group_id = u32::from(player_id);
        if !self.view_groups.iter().any(|g| g.id == group_id) {
            let mut g = ViewGroup::new(group_id, Some(format!("User_{player_id}")));
            g.add_member(player_id);
            g.add_visible_group(group_id);
            self.view_groups.push(g);
        }
        self.view_groups
            .iter_mut()
            .find(|g| g.id == group_id)
            .unwrap()
    }

    pub fn create_custom_group(&mut self, name: Option<String>) -> &mut ViewGroup {
        let id = self.next_view_group_id;
        self.next_view_group_id += 1;
        self.view_groups.push(ViewGroup::new(id, name));
        self.view_groups.last_mut().unwrap()
    }

    pub fn get_view_group(&self, group_id: u32) -> Option<&ViewGroup> {
        self.view_groups.iter().find(|g| g.id == group_id)
    }

    pub fn get_view_group_mut(&mut self, group_id: u32) -> Option<&mut ViewGroup> {
        self.view_groups.iter_mut().find(|g| g.id == group_id)
    }

    pub fn remove_view_group(&mut self, group_id: u32) -> bool {
        // Cannot remove auto-groups.
        if group_id <= u32::from(u16::MAX) {
            return false;
        }
        if let Some(pos) = self.view_groups.iter().position(|g| g.id == group_id) {
            self.view_groups.remove(pos);
            for g in &mut self.view_groups {
                g.remove_visible_group(group_id);
            }
            true
        } else {
            false
        }
    }

    pub fn get_user_groups(&self, player_id: u16) -> Vec<u32> {
        self.view_groups
            .iter()
            .filter(|g| g.is_member(player_id))
            .map(|g| g.id)
            .collect()
    }

    /// Check whether `viewer_id` can see `target_id` via shared ViewGroup visibility.
    pub fn can_user_see_user(&self, viewer_id: u16, target_id: u16) -> bool {
        if viewer_id == target_id {
            return true;
        }
        let viewer_groups = self.get_user_groups(viewer_id);
        let target_groups = self.get_user_groups(target_id);
        for vg_id in &viewer_groups {
            if let Some(vg) = self.get_view_group(*vg_id) {
                for tg_id in &target_groups {
                    if vg.can_see_group(*tg_id) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Remove all ViewGroup memberships for a player; delete empty auto-groups.
    pub fn cleanup_user(&mut self, player_id: u16) {
        for g in &mut self.view_groups {
            g.remove_member(player_id);
        }
        let auto_id = u32::from(player_id);
        if let Some(pos) = self
            .view_groups
            .iter()
            .position(|g| g.id == auto_id && g.members.is_empty())
        {
            self.view_groups.remove(pos);
        }
    }

    // ── Moderation ────────────────────────────────────────────────────────

    pub fn get_moderated(&self, user_id: u32, address: &str) -> Option<&UserModerated> {
        self.moderated
            .iter()
            .find(|m| m.user_id == user_id && m.address == address)
    }

    pub fn get_moderated_by_user(&self, user_id: u32, address: &str) -> Option<&UserModerated> {
        self.get_moderated(user_id, address)
    }

    // ── Load Balancing ────────────────────────────────────────────────────

    /// Calculate adaptive TPS and threshold based on load.
    /// Updates internal effective_tps/threshold and returns whether values changed.
    /// Returns (effective_tps, effective_threshold, changed).
    pub fn update_adaptive_settings(
        &mut self,
        config: &crate::config::LoadBalancingConfig,
    ) -> (u8, f32, bool) {
        let enabled = self.load_balancing_enabled.unwrap_or(config.enabled);

        if !enabled {
            let changed = self.effective_tps != Some(self.tps)
                || self.effective_threshold != Some(self.threshold);
            self.effective_tps = Some(self.tps);
            self.effective_threshold = Some(self.threshold);
            return (self.tps, self.threshold, changed);
        }

        let player_count = self.players.len();

        // Factor 1: Player count based on configured tiers
        let player_factor = config
            .tiers
            .iter()
            .find(|tier| player_count <= tier.max_players)
            .map(|tier| tier.tps_factor)
            .unwrap_or(0.60); // Fallback if no tier matches

        // Factor 2: Performance (tick duration vs budget)
        let tick_budget = 1000.0 / self.tps as f32;
        let perf_factor = if self.last_tick_duration > tick_budget * config.perf_threshold {
            // Exceeded threshold, reduce proportionally
            (tick_budget * config.perf_threshold / self.last_tick_duration).max(0.5)
        } else {
            1.0
        };

        // Combine factors with configured weights
        let combined = player_factor * config.player_weight + perf_factor * config.perf_weight;

        // Calculate effective values
        let new_tps = ((self.tps as f32 * combined).max(config.min_tps as f32) as u8).min(self.tps);
        let new_threshold = self.threshold * (1.0 + (1.0 - combined) * 0.5);

        // Check if values changed
        let changed = self.effective_tps != Some(new_tps)
            || (self.effective_threshold.is_some()
                && (self.effective_threshold.unwrap() - new_threshold).abs() > 0.001);

        self.effective_tps = Some(new_tps);
        self.effective_threshold = Some(new_threshold);

        (new_tps, new_threshold, changed)
    }

    /// Get current effective TPS (taking load balancing into account).
    pub fn get_effective_tps(&self, config: &crate::config::LoadBalancingConfig) -> u8 {
        if let Some(tps) = self.effective_tps {
            tps
        } else {
            // Not yet calculated, compute now (but don't store)
            let enabled = self.load_balancing_enabled.unwrap_or(config.enabled);
            if !enabled {
                self.tps
            } else {
                let player_count = self.players.len();
                let player_factor = config
                    .tiers
                    .iter()
                    .find(|tier| player_count <= tier.max_players)
                    .map(|tier| tier.tps_factor)
                    .unwrap_or(0.60);
                let tick_budget = 1000.0 / self.tps as f32;
                let perf_factor = if self.last_tick_duration > tick_budget * config.perf_threshold {
                    (tick_budget * config.perf_threshold / self.last_tick_duration).max(0.5)
                } else {
                    1.0
                };
                let combined =
                    player_factor * config.player_weight + perf_factor * config.perf_weight;
                ((self.tps as f32 * combined).max(config.min_tps as f32) as u8).min(self.tps)
            }
        }
    }

    /// Get current effective threshold (taking load balancing into account).
    pub fn get_effective_threshold(&self, _config: &crate::config::LoadBalancingConfig) -> f32 {
        if let Some(threshold) = self.effective_threshold {
            threshold
        } else {
            // Not yet calculated
            self.threshold
        }
    }

    /// Get the effective TPS for a specific player.
    /// Returns custom TPS if set, otherwise adaptive TPS.
    pub fn get_player_tps(
        &self,
        player_id: u16,
        config: &crate::config::LoadBalancingConfig,
    ) -> u8 {
        if let Some(player) = self.get_player(player_id) {
            if player.custom_tps != 0 {
                return player.custom_tps;
            }
        }
        self.get_effective_tps(config)
    }

    /// Get the effective threshold for a specific player.
    /// Returns custom threshold if set, otherwise adaptive threshold.
    pub fn get_player_threshold(
        &self,
        player_id: u16,
        config: &crate::config::LoadBalancingConfig,
    ) -> f32 {
        if let Some(player) = self.get_player(player_id) {
            if player.custom_threshold != 0.0 {
                return player.custom_threshold;
            }
        }
        self.get_effective_threshold(config)
    }
}

/// Thread-safe wrapper.
pub type ArcInstance = Arc<RwLock<Instance>>;
