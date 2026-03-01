use serde::{Deserialize, Serialize};

use crate::utils::system_specs::SpecsData;

// ─── Generic envelope ───────────────────────────────────────────────────────

/// The JSON envelope exchanged with the MasterServer WebSocket.
///
/// `{ "type": "...", "data": {...}, "id": "uuid" }`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsMessage<T> {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

impl<T: Serialize> WsMessage<T> {
    pub fn new(msg_type: impl Into<String>, data: T) -> Self {
        Self { msg_type: msg_type.into(), data, id: None }
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
}

// ─── Status response ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayStatus {
    /// Instance list.
    pub i: Vec<RelayInstance>,
    /// Connected client list.
    pub c: Vec<RelayClient>,
    /// Max instances.
    pub m: u8,
    /// Relay version string.
    pub v: String,
    /// Protocol version.
    pub p: u16,
    /// Relay start Unix-ms timestamp.
    pub u: i64,
    /// System specs.
    pub s: SpecsData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayInstance {
    /// Internal ID.
    pub i: u8,
    /// Master-server ID.
    pub m: u32,
    /// Players.
    pub p: Vec<RelayPlayer>,
    /// Flag names (snake_case strings).
    pub f: Vec<String>,
    /// World identifier string.
    pub w: String,
    /// Capacity.
    pub c: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayPlayer {
    /// Player ID.
    pub p: u16,
    /// Client ID.
    pub c: u16,
    /// Display name.
    pub d: String,
    /// Flag names.
    pub f: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayClient {
    /// Client ID.
    pub i: u16,
    /// Platform.
    pub p: String,
    /// Engine.
    pub e: String,
    /// Remote address string.
    pub a: String,
    /// User identifier (`"{id}@{server}"`) if authenticated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub u: Option<String>,
}

// ─── resolve_user ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveUserRequest {
    pub user_id: u32,
    pub server: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveUserResponse {
    pub result: String,
    pub user: Option<ResolveUserInfo>,
    #[serde(default)]
    pub expire_at: i64,
    pub reason: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveUserInfo {
    pub id: u32,
    pub username: String,
    pub display: String,
    pub server: String,
}

// ─── request_instances ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestInstancesReq {
    pub count: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestInstancesResp {
    pub success: bool,
    pub error: Option<String>,
    #[serde(default)]
    pub instances: Vec<InstanceSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceSpec {
    pub id: u32,
    pub password: Option<String>,
    pub capacity: u16,
    pub world: Option<WorldSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSpec {
    pub id: u32,
    pub address: String,
    pub version: u16,
}

// ─── relay_sync_instances ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncInstancesResp {
    pub success: bool,
    pub error: Option<String>,
    pub invalid_instances: Option<Vec<u32>>,
    pub synced_count: i32,
    pub conflicts_resolved: i32,
}

// ─── relay_connected ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayConnected {
    pub id: u32,
    pub master_address: String,
}

// ─── drop_instance ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DropInstance {
    pub instance_id: u32,
    pub reason: String,
    pub message: Option<String>,
}

// ─── logs request ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogsRequest {
    #[serde(default)]
    pub since: i64,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize { 100 }
