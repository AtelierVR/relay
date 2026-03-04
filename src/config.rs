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

    /// WebSocket URL of the MasterServer.
    #[serde(default = "default_master_gateway")]
    pub master_gateway: String,

    /// Public address advertised to clients (e.g. "1.2.3.4:23032").
    #[serde(default = "default_use_address")]
    pub use_address: String,

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

    /// Enable debug-level logging.
    #[serde(default)]
    pub debug: bool,
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
fn default_master_gateway() -> String {
    DEFAULT_MASTER_GATEWAY.to_owned()
}
fn default_use_address() -> String {
    DEFAULT_USE_ADDRESS.to_owned()
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
