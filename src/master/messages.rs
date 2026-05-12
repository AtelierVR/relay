use serde::{Deserialize, Serialize};

use crate::utils::system_specs::SpecsData;

// ─── Generic envelope ───────────────────────────────────────────────────────

/// The JSON envelope exchanged with the MasterServer WebSocket.
///
/// `{ "type": "...", "payload": {...}, "id": "uuid" }`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsMessage<T> {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub payload: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

impl<T: Serialize> WsMessage<T> {
    pub fn new(msg_type: impl Into<String>, payload: T) -> Self {
        Self {
            msg_type: msg_type.into(),
            payload,
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

/// Emitted to the node when a QUIC client completes handshake (platform/engine are known).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventClientConnected {
    /// Relay-assigned client ID.
    pub id: u16,
    /// Remote IP:port.
    pub address: String,
    /// Client platform (e.g. "windows", "android").
    pub platform: String,
    /// Client engine identifier (e.g. "unity").
    pub engine: String,
    /// Unix millisecond timestamp when the client connected.
    pub connected_at: i64,
}

/// Emitted to the node when a client authenticates successfully.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventClientAuthentified {
    /// Relay-assigned client ID.
    pub id: u16,
    /// NoxIdentifier of the authenticated user (e.g. "1@hactazia.fr").
    pub user: String,
}

/// Emitted to the node when a QUIC client disconnects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventClientDisconnected {
    /// Relay-assigned client ID.
    pub id: u16,
    /// Disconnect reason string (e.g. "normal", "timeout", "kicked").
    pub reason: String,
    /// Disconnect type category.
    #[serde(rename = "type")]
    pub kind: String,
}

/// Emitted to the node when a player joins an instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventPlayerJoin {
    pub client_id: u16,
    pub player_id: u16,
    pub display: String,
    /// Relay-internal instance slot (0–254).
    pub internal_id: u8,
    pub flags: u32,
    /// Unix millisecond timestamp when the player joined.
    pub joined_at: i64,
}

/// Emitted to the node when a player leaves an instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventPlayerLeave {
    pub player_id: u16,
    /// Relay-internal instance slot (0–254).
    pub internal_id: u8,
    /// Quit type ("normal", "timeout", "moderation_kick", "vote_kick", "configuration_error", "unknown_error").
    #[serde(rename = "type")]
    pub kind: String,
    pub reason: String,
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
    pub i: u16,            // client ID
    pub a: String,         // address
    pub p: String,         // platform
    pub e: String,         // engine
    pub u: Option<String>, // user identifier (optional)
    pub t: i64,            // connected_at (Unix ms)
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
    pub i: u32,             // internal ID (relay-local slot 0–254)
    pub n: u32,             // node ID (master/DB ID)
    pub f: u32,             // flags
    pub p: u32,             // player count (all, including HIDE_IN_LIST)
    pub w: String,          // world
    pub c: u16,             // capacity
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub i: u16,            // player ID
    pub c: u16,            // client ID
    pub d: String,         // display name
    pub f: u32,            // flags
    pub u: Option<String>, // user identifier (e.g. "1@hactazia.fr"), None if unauthenticated
    pub j: i64,            // joined_at (Unix ms)
}

// ─── get_players ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPlayersReq {
    /// Internal instance ID (matches InstanceInfo.i).
    pub i: u32,
    #[serde(default)]
    pub l: usize,
    #[serde(default)]
    pub o: usize,
    /// When true, include players with the HIDE_IN_LIST flag. Default false.
    #[serde(default)]
    pub a: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPlayersResp {
    /// Total number of visible (or all, if all=true) players in the instance.
    pub t: u32,
    /// The requested page of players.
    pub i: Vec<PlayerInfo>,
}
