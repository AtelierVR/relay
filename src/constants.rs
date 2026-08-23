// Protocol and server constants.
pub const PROTOCOL_VERSION: u16 = 2;
pub const ENGINE: &str = "nox";
pub const VERSION: &str = "0.1.0";

// Default network settings.
pub const DEFAULT_PORT: u16 = 23032;
pub const DEFAULT_NODE_GATEWAY: &str = "http://127.0.0.1:3042";
pub const DEFAULT_NODE_ADDRESS: &str = "127.0.0.1:23032";

// Special world address meaning "use the relay's own public address".
pub const SAFE_LOCAL_ADDRESS: &str = "::";

// Packet sizing.
pub const MAX_PACKET_SIZE: usize = 1029; // 1024 payload + 5 header bytes
pub const STREAM_HEADER_SIZE: usize = 5; // [Length: u16][UID: u16][Type: u8]
pub const DGRAM_HEADER_SIZE: usize = 5; // [Length: u16][UID: u16][Type: u8] — same as stream

// Session timeouts (seconds).
pub const DEFAULT_CONNECTION_TIMEOUT: u16 = 15;
pub const DEFAULT_KEEP_ALIVE_INTERVAL: u16 = 5;
/// Interval (seconds) at which the server asks clients to re-send unchanged LocalEmit properties.
/// 0 = feature disabled.
pub const DEFAULT_PROPERTY_RESEND_INTERVAL: u8 = 5;

// Instance limits.
pub const DEFAULT_MAX_INSTANCES: u8 = 3;

// Default instance settings.
pub const DEFAULT_TPS: u8 = 60;
pub const DEFAULT_THRESHOLD: f32 = 0.001;
pub const DEFAULT_RENDER_ENTITY: f32 = 500.0;

// Master server periodic intervals (ms).
pub const MASTER_PING_INTERVAL_MS: u64 = 15_000;
pub const MASTER_SPECS_INTERVAL_MS: u64 = 500;

// Outgoing packet channel buffer capacity per client.
pub const CLIENT_TX_BUFFER: usize = 256;
