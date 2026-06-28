use figment::{
    providers::{Env, Format, Json},
    Figment,
};
use serde::{Deserialize, Serialize};

use crate::constants::*;

/// Server configuration loaded from config.json then overridden by NOX_* env vars.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// UDP/TCP listen port (QUIC binds here).
    #[serde(default = "default_port")]
    pub port: u16,

    /// WebSocket URL of the Node server.
    #[serde(default = "default_node_gateway")]
    pub node_gateway: String,

    /// Public Node server address advertised to clients (e.g. "hactazia.fr:53032").
    #[serde(default = "default_node_address")]
    pub node_address: String,

    /// Public address of this relay advertised to clients (e.g. "hactazia.fr:23000").
    /// If not set, defaults to "0.0.0.0:{port}".
    #[serde(default)]
    pub use_address: Option<String>,

    /// Bearer token for MasterServer authentication.
    #[serde(default)]
    pub token: String,

    /// Maximum number of instances this relay will host.
    #[serde(default = "default_max_instances")]
    pub max_instances: u8,

    /// Seconds before an idle client is disconnected.
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout: u16,

    /// Seconds between keep-alive pings.
    #[serde(default = "default_keep_alive_interval")]
    pub keep_alive_interval: u16,

    /// Interval in seconds at which clients must re-send unchanged LocalEmit properties so late
    /// joiners receive up-to-date values. Set to 0 to disable forced re-sends.
    #[serde(default = "default_property_resend_interval")]
    pub property_resend_interval: u8,

    /// Enable debug-level logging.
    #[serde(default)]
    pub debug: bool,

    /// Load balancing configuration.
    #[serde(default)]
    pub load_balancing: LoadBalancingConfig,
}

/// Configuration for adaptive load balancing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancingConfig {
    /// Enable adaptive load balancing.
    #[serde(default = "default_lb_enabled")]
    pub enabled: bool,

    /// Update interval in seconds.
    #[serde(default = "default_lb_update_interval")]
    pub update_interval: u16,

    /// Minimum TPS (never go below this).
    #[serde(default = "default_lb_min_tps")]
    pub min_tps: u8,

    /// Weight for player count factor (0.0-1.0).
    /// In the perf-first model this is a secondary modulator — keep it low.
    #[serde(default = "default_lb_player_weight")]
    pub player_weight: f32,

    /// Weight for performance factor (0.0-1.0).
    /// The primary driver of the TPS curve. Should be dominant.
    #[serde(default = "default_lb_perf_weight")]
    pub perf_weight: f32,

    /// Performance threshold (tick time / budget ratio to trigger reduction).
    #[serde(default = "default_lb_perf_threshold")]
    pub perf_threshold: f32,

    /// Player load tiers: [(max_players, tps_multiplier)].
    #[serde(default = "default_lb_tiers")]
    pub tiers: Vec<LoadTier>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadTier {
    /// Maximum player count for this tier.
    pub max_players: usize,
    /// TPS multiplier (0.0-1.0).
    pub tps_factor: f32,
}

impl Default for LoadBalancingConfig {
    fn default() -> Self {
        Self {
            enabled: default_lb_enabled(),
            update_interval: default_lb_update_interval(),
            min_tps: default_lb_min_tps(),
            player_weight: default_lb_player_weight(),
            perf_weight: default_lb_perf_weight(),
            perf_threshold: default_lb_perf_threshold(),
            tiers: default_lb_tiers(),
        }
    }
}

impl Config {
    /// Load from `config.json` (optional) then overlay `NOX_*` environment variables.
    pub fn load() -> anyhow::Result<Self> {
        let figment = Figment::new();

        // Try to merge config.json if it exists (optional)
        let figment = if std::path::Path::new("config.json").exists() {
            figment.merge(Json::file("config.json"))
        } else {
            figment
        };

        // Always merge environment variables (these can override or provide config)
        let cfg = figment
            .merge(Env::prefixed("NOX_").map(|k| k.as_str().to_lowercase().into()))
            .extract()?;
        Ok(cfg)
    }
}

fn default_port() -> u16 {
    DEFAULT_PORT
}
fn default_node_gateway() -> String {
    DEFAULT_NODE_GATEWAY.to_owned()
}
fn default_node_address() -> String {
    DEFAULT_NODE_ADDRESS.to_owned()
}
fn default_max_instances() -> u8 {
    DEFAULT_MAX_INSTANCES
}
fn default_connection_timeout() -> u16 {
    DEFAULT_CONNECTION_TIMEOUT
}
fn default_keep_alive_interval() -> u16 {
    DEFAULT_KEEP_ALIVE_INTERVAL
}
fn default_property_resend_interval() -> u8 {
    DEFAULT_PROPERTY_RESEND_INTERVAL
}

fn default_lb_enabled() -> bool {
    true
}
fn default_lb_update_interval() -> u16 {
    3 // seconds - check more frequently for faster response
}
fn default_lb_min_tps() -> u8 {
    5
}
fn default_lb_player_weight() -> f32 {
    0.15 // Secondary — player count only nudges when perf is healthy
}
fn default_lb_perf_weight() -> f32 {
    0.85 // Primary — real tick performance is the main driver
}
fn default_lb_perf_threshold() -> f32 {
    0.80 // 80% of tick budget - more aggressive
}
fn default_lb_tiers() -> Vec<LoadTier> {
    vec![
        LoadTier {
            max_players: 10,
            tps_factor: 1.00, // No player penalty up to 10
        },
        LoadTier {
            max_players: 20,
            tps_factor: 0.95,
        },
        LoadTier {
            max_players: 35,
            tps_factor: 0.88,
        },
        LoadTier {
            max_players: 50,
            tps_factor: 0.80,
        },
        LoadTier {
            max_players: 70,
            tps_factor: 0.72,
        },
        LoadTier {
            max_players: usize::MAX,
            tps_factor: 0.65, // Floor for player-count-only reduction
        },
    ]
}
