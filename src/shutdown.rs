//! Graceful shutdown: notify every connected client, then close the QUIC endpoint.
//!
//! Used by the `Ctrl+C` handler so clients receive a `Disconnect` packet with a
//! reason instead of only seeing a dead transport (which the Unity client would
//! treat as an unexpected disconnection and try to reconnect).

use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use quinn::{Endpoint, VarInt};
use tokio::time::timeout;
use tracing::{debug, info, warn};

use crate::{
    client::client::ArcClient,
    handlers::context::AppState,
    master::messages::EventClientDisconnected,
    proto::{
        buffer::PacketWriter, frame::write_framed, header::encode_stream_packet,
        packet_type::PacketType,
    },
};

/// Time allowed for the `Disconnect` notifications to leave the socket before
/// the connections are closed.
const FLUSH_DELAY: Duration = Duration::from_millis(300);
/// Per-client timeout for opening/writing the notification stream.
const NOTIFY_TIMEOUT: Duration = Duration::from_millis(500);
/// Max time to wait for the endpoint to become idle once closed.
const IDLE_TIMEOUT: Duration = Duration::from_secs(3);
/// QUIC application error code used in `CONNECTION_CLOSE`.
const SHUTDOWN_ERROR_CODE: u32 = 0x1;

/// Encode a server-initiated `Disconnect` packet carrying `reason`.
fn build_disconnect(reason: &str) -> Bytes {
    let mut w = PacketWriter::new();
    if !reason.is_empty() {
        w.write_string(reason);
    }
    encode_stream_packet(0, PacketType::Disconnect, w.finish().as_ref())
}

/// Best-effort notification on a dedicated uni-stream (reliable transport).
async fn notify_client(client: &ArcClient, packet: &Bytes) {
    // Clone the connection so the client lock is not held across the await.
    let conn = Arc::clone(&client.read().conn);
    let write = async {
        if let Ok(mut send) = conn.open_uni().await {
            if write_framed(&mut send, packet).await.is_ok() {
                let _ = send.finish();
            }
        }
    };
    if timeout(NOTIFY_TIMEOUT, write).await.is_err() {
        debug!("[Shutdown] timed out while notifying a client");
    }
}

/// Disconnect every client, close all QUIC connections and the endpoint.
///
/// This is best-effort by design: if a client is unreachable we still close
/// everything and let the process exit promptly.
pub async fn graceful_shutdown(state: &Arc<AppState>, endpoint: &Endpoint, reason: &str) {
    // Snapshot the clients so we do not hold iterators across awaits.
    let clients: Vec<(u16, ArcClient)> = {
        let mut list = Vec::new();
        state
            .clients
            .for_each(|id, arc| list.push((id, Arc::clone(arc))));
        list
    };

    info!(
        "[Shutdown] {} client(s) connected — sending disconnect notice",
        clients.len()
    );

    let packet = build_disconnect(reason);
    for (id, client) in &clients {
        let user = {
            let c = client.read();
            c.user
                .as_ref()
                .map(|u| format!("{}@{} (\"{}\")", u.id, u.address, u.display_name))
        };
        info!(
            "[Shutdown] disconnecting client {id}{} | reason={reason}",
            user.map(|u| format!(" | user={u}")).unwrap_or_default()
        );
        notify_client(client, &packet).await;
    }

    // Let the (reliable) notification streams flush before we tear down.
    tokio::time::sleep(FLUSH_DELAY).await;

    // Close every connection with an application error code, then the endpoint.
    for (id, client) in &clients {
        let conn = Arc::clone(&client.read().conn);
        conn.close(VarInt::from_u32(SHUTDOWN_ERROR_CODE), b"relay shutdown");
        debug!("[Shutdown] connection for client {id} closed");

        let _ = state.master.emit(
            "client_disconnected",
            EventClientDisconnected {
                id: *id,
                reason: reason.to_string(),
                kind: "shutdown".to_string(),
            },
        );
    }

    // Closing the endpoint also stops the accept loop in `quic_server::run`.
    endpoint.close(VarInt::from_u32(SHUTDOWN_ERROR_CODE), b"relay shutdown");
    if timeout(IDLE_TIMEOUT, endpoint.wait_idle()).await.is_err() {
        warn!(
            "[Shutdown] endpoint did not become idle within {}s",
            IDLE_TIMEOUT.as_secs()
        );
    } else {
        info!("[Shutdown] all connections closed");
    }
}
