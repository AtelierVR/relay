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
        Self {
            msg_type: msg_type.into(),
            data,
            id: None,
        }
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
}

// ─── Status response ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayStatus {
    /// Active client count.
    pub c: u32,
    /// Active instance count.
    pub i: u32,
    /// Max instances.
    pub m: u8,
    /// Engine identifier.
    pub e: String,
    /// Relay version string.
    pub v: String,
    /// Protocol version.
    pub p: u16,
    /// Relay start Unix-ms timestamp.
    pub u: i64,
    /// System specs.
    pub s: SpecsData,
    /// Connection addresses: proto -> "host:port" (e.g. {"quic": "0.0.0.0:30000"}).
    pub a: std::collections::HashMap<String, String>,
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
    #[serde(default)]
    pub version: u16,
}

// ─── relay_sync_instances ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncInstancesReq {
    pub instances: Vec<SyncInstanceData>,
    pub relay_uptime: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncInstanceData {
    pub master_id: u32,
    pub internal_id: u32,
    pub password: Option<String>,
    pub capacity: u16,
    pub player_count: usize,
    pub world: String,
    pub flags: String,
}

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

// ─── relay events ───────────────────────────────────────────────────────────

/// Emitted to the node when a QUIC client connects to the relay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventClientConnected {
    /// Relay-assigned client ID.
    pub id: String,
    /// Remote IP:port.
    pub address: String,
}

/// Emitted to the node when a QUIC client disconnects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventClientDisconnected {
    /// Relay-assigned client ID.
    pub id: String,
    /// Remote IP:port.
    pub address: String,
    /// User identifier at time of disconnect, if authenticated.
    pub user: Option<String>,
}

/// Emitted to the node when a player joins an instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventPlayerJoin {
    pub client_id: String,
    pub player_id: String,
    pub instance_id: String,
    /// User identifier, if authenticated.
    pub user: Option<String>,
    pub display: String,
}

/// Emitted to the node when a player leaves an instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventPlayerLeave {
    pub client_id: String,
    pub player_id: String,
    pub instance_id: String,
    /// User identifier, if known.
    pub user: Option<String>,
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

fn default_limit() -> usize {
    100
}

// ─── has_instance ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HasInstanceReq {
    pub id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HasInstanceResp {
    pub exists: bool,
}

// ─── get_clients ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetClientsReq {
    #[serde(default)]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetClientsResp {
    pub total: u32,
    pub clients: Vec<ClientInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub i: String,         // client ID
    pub a: String,         // address
    pub p: String,         // platform
    pub e: String,         // engine
    pub u: Option<String>, // user identifier (optional)
}

// ─── ping ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingResponse {
    pub time: i64,
    pub received: i64,
}

// ─── command ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRequest {
    pub content: String,
}

// ─── get_instances ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetInstancesReq {
    #[serde(default)]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetInstancesResp {
    pub total: u32,
    pub instances: Vec<InstanceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceInfo {
    pub i: String,          // instance master ID
    pub n: u32,             // internal ID
    pub p: Vec<PlayerInfo>, // players
    pub f: u32,             // flags
    pub w: String,          // world
    pub c: u16,             // capacity
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub i: String,         // player ID
    pub c: String,         // client ID
    pub d: String,         // display name
    pub f: u32,            // flags
    pub u: Option<String>, // user identifier (e.g. "1@hactazia.fr"), None if unauthenticated
}
