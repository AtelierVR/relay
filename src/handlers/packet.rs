use std::sync::Arc;

use bytes::Bytes;
use tokio::sync::oneshot;

use crate::{
    client::client::ArcClient,
    handlers::context::AppState,
    proto::{header::encode_stream_packet, packet_type::PacketType},
};

/// A fully-parsed incoming packet passed to handlers.
///
/// For stream (bidi) packets, `reply*` methods write the response back on
/// the same bidi stream via an internal oneshot channel.
/// For datagram packets there is no reply path; `reply*` calls are no-ops.
pub struct Packet {
    /// Shared application state.
    pub state: Arc<AppState>,
    /// The sending client's shared handle.
    pub client: ArcClient,
    /// Packet type (used for routing in the dispatcher).
    pub packet_type: PacketType,
    /// Request UID for response correlation.
    pub uid: u16,
    /// Decoded payload bytes (header stripped).
    pub payload: Bytes,
    /// Present for bidi-stream packets; absent for datagrams.
    reply_tx: Option<oneshot::Sender<Bytes>>,
}

impl Packet {
    /// Create a stream (bidi) packet with a reply channel.
    pub fn new_stream(
        state: Arc<AppState>,
        client: ArcClient,
        packet_type: PacketType,
        uid: u16,
        payload: Bytes,
        reply_tx: oneshot::Sender<Bytes>,
    ) -> Self {
        Self {
            state,
            client,
            packet_type,
            uid,
            payload,
            reply_tx: Some(reply_tx),
        }
    }

    /// Create a datagram packet (no reply path).
    pub fn new_datagram(
        state: Arc<AppState>,
        client: ArcClient,
        packet_type: PacketType,
        uid: u16,
        payload: Bytes,
    ) -> Self {
        Self {
            state,
            client,
            packet_type,
            uid,
            payload,
            reply_tx: None,
        }
    }

    /// Returns `true` if this packet arrived on a bidi stream (has a reply channel).
    #[inline]
    pub fn is_stream(&self) -> bool {
        self.reply_tx.is_some()
    }

    /// Return the relay-assigned client ID.
    #[inline]
    pub fn client_id(&self) -> u16 {
        self.client.read().id
    }

    /// Encode `payload` as a stream packet and send it back on the bidi stream.
    ///
    /// Only the first call has any effect; subsequent calls are no-ops.
    pub fn reply(&mut self, payload: Bytes, ptype: PacketType, uid: u16) {
        let encoded = encode_stream_packet(uid, ptype, payload.as_ref());
        self.reply_raw(encoded);
    }

    /// Convenience: reply using the incoming packet's own UID.
    pub fn reply_self(&mut self, payload: Bytes, ptype: PacketType) {
        let uid = self.uid;
        self.reply(payload, ptype, uid);
    }

    /// Send a pre-encoded packet back on the bidi stream.
    ///
    /// Use this when the response bytes are already fully encoded (e.g. produced
    /// by `encode_stream_packet`). Only the first call has any effect.
    pub fn reply_raw(&mut self, encoded: Bytes) {
        if let Some(tx) = self.reply_tx.take() {
            let _ = tx.send(encoded);
        }
    }
}
