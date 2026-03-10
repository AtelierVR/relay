use std::time::Instant;

use bytes::Bytes;
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::mpsc;
use quinn::Connection;

use super::user::User;
use crate::constants::CLIENT_TX_BUFFER;
use crate::proto::{
    header::{encode_datagram, encode_stream_packet},
    packet_type::PacketType,
};

/// Whether to send via a reliable QUIC stream or an unreliable datagram.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendType {
    /// Reliable: pushed to the outgoing uni-stream channel.
    Stream,
    /// Best-effort: sent as a QUIC datagram.
    Datagram,
}

/// An enum indicating the authentication phase of a client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthState {
    /// No handshake performed yet.
    None,
    /// Handshake accepted; awaiting auth.
    Handshaked,
    /// Challenge was issued; awaiting signature response.
    ChallengeIssued,
    /// Fully authenticated.
    Authenticated,
}

/// A connected QUIC client.
#[derive(Debug)]
pub struct Client {
    /// Relay-assigned numeric client ID.
    pub id: u16,
    /// Client platform string (e.g. `"PC"`, `"Quest"`).
    pub platform: String,
    /// Client engine string (e.g. `"Unity"`, `"Unreal"`).
    pub engine: String,
    /// Time of the last received packet.
    pub last_seen: Instant,
    /// Authenticated user, if auth succeeded.
    pub user: Option<User>,
    /// Random challenge bytes sent for RSA challenge-response.
    pub challenge: Vec<u8>,
    /// Current authentication state.
    pub auth_state: AuthState,
    /// Outgoing packet sender (push path).
    pub tx: mpsc::Sender<Bytes>,
    /// QUIC connection for sending datagrams.
    pub conn: Arc<Connection>,
}

impl Client {
    pub fn new(id: u16, tx: mpsc::Sender<Bytes>, conn: Arc<Connection>) -> Self {
        Self {
            id,
            platform: String::new(),
            engine: String::new(),
            last_seen: Instant::now(),
            user: None,
            challenge: Vec::new(),
            auth_state: AuthState::None,
            tx,
            conn,
        }
    }

    pub fn is_handshaked(&self) -> bool {
        self.auth_state != AuthState::None
    }

    pub fn is_authenticated(&self) -> bool {
        self.auth_state == AuthState::Authenticated
    }

    pub fn is_authenticating(&self) -> bool {
        matches!(self.auth_state, AuthState::ChallengeIssued)
    }

    /// Try to send a packet via the push channel. Returns `false` if the channel is full.
    pub fn try_push(&self, packet: Bytes) -> bool {
        self.tx.try_send(packet).is_ok()
    }

    /// Send a datagram (for broadcasts). Returns `true` on success.
    pub fn send_datagram(&self, packet: Bytes) -> bool {
        self.conn.send_datagram(packet).is_ok()
    }

    /// Encode and send a packet via the specified transport.
    ///
    /// - [`SendType::Stream`]: encodes as a stream packet and pushes to the
    ///   outgoing push channel (opened as a QUIC uni-stream by the push writer).
    /// - [`SendType::Datagram`]: encodes as a datagram and sends immediately.
    ///
    /// Returns `true` on success.
    pub fn send(&self, payload: Bytes, ptype: PacketType, uid: u16, send_type: SendType) -> bool {
        match send_type {
            SendType::Stream => {
                self.try_push(encode_stream_packet(uid, ptype, payload.as_ref()))
            }
            SendType::Datagram => {
                self.send_datagram(encode_datagram(uid, ptype, payload.as_ref()))
            }
        }
    }
}

/// A thread-safe shared reference to a client.
pub type ArcClient = Arc<RwLock<Client>>;

/// Create a new `ArcClient` and return the push receiver as well.
pub fn new_client(id: u16, conn: Arc<Connection>) -> (ArcClient, mpsc::Receiver<Bytes>) {
    let (tx, rx) = mpsc::channel(CLIENT_TX_BUFFER);
    let client = Arc::new(RwLock::new(Client::new(id, tx, conn)));
    (client, rx)
}
