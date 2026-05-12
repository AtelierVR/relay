use std::sync::Arc;

use anyhow::Result;
use quinn::Connection;
use tokio::sync::oneshot;
use tracing::{debug, warn};

use crate::{
    client::client::new_client,
    handlers::{context::AppState, dispatcher::dispatch, packet::Packet},
    master::messages::EventClientDisconnected,
    proto::{
        frame::{parse_datagram, read_framed, write_framed},
        header::decode_stream_header,
    },
};

/// Drive a single QUIC connection to completion.
///
/// Spawns three tasks:
/// 1. **bidi_reader** – accepts bidirectional streams, reads request, dispatches, writes response.
/// 2. **datagram_reader** – reads QUIC datagrams (Transform, Voice) and dispatches them.
/// 3. **push_writer** – drains the client's `tx` channel and sends server-initiated packets.
pub async fn handle_connection(state: Arc<AppState>, conn: Connection) -> Result<()> {
    let remote_addr = conn.remote_address().to_string();

    // Allocate client ID and register a new client.
    let client_id = state.next_client_id();
    let (client_arc, mut push_rx) = new_client(client_id, Arc::new(conn.clone()));
    client_arc.write().address = remote_addr.clone();
    state.clients.add(Arc::clone(&client_arc));
    debug!("[Connection] client {client_id} connected from {remote_addr}");
    // client_connected is emitted after the handshake (when platform/engine are known).

    let conn1 = conn.clone();
    let conn2 = conn.clone();
    let conn3 = conn.clone();

    let state1 = Arc::clone(&state);
    let state2 = Arc::clone(&state);

    // ── Task 1: bidi-stream reader ───────────────────────────────────────
    let bidi_task = tokio::spawn(async move {
        loop {
            match conn1.accept_bi().await {
                Ok((mut send, mut recv)) => {
                    let state = Arc::clone(&state1);
                    tokio::spawn(async move {
                        // Read the full framed message.
                        let framed = match read_framed(&mut recv).await {
                            Ok(b) => b,
                            Err(e) => {
                                debug!("[BiStream] read error: {e}");
                                return;
                            }
                        };

                        // Decode header + payload.
                        let (header, payload) = match decode_stream_header(framed) {
                            Ok(hp) => hp,
                            Err(e) => {
                                warn!("[BiStream] header decode error: {e}");
                                return;
                            }
                        };

                        // Look up the client arc.
                        let client_arc = match state.clients.get(client_id) {
                            Some(a) => a,
                            None => {
                                debug!("[BiStream] client {} not found", client_id);
                                return;
                            }
                        };

                        // Create reply channel and packet.
                        let (reply_tx, reply_rx) = oneshot::channel();
                        let packet = Packet::new_stream(
                            Arc::clone(&state),
                            client_arc,
                            header.packet_type,
                            header.uid,
                            payload,
                            reply_tx,
                        );

                        // Dispatch; handler calls packet.reply*() internally.
                        dispatch(packet).await;

                        // Collect reply (empty bytes if handler sent no reply).
                        let response = reply_rx.await.unwrap_or_default();

                        // Write response on the same stream.
                        if let Err(e) = write_framed(&mut send, &response).await {
                            debug!("[BiStream] write error: {e}");
                        }
                        let _ = send.finish();
                    });
                }
                Err(e) => {
                    debug!("[BiStream] accept_bi error: {e}");
                    break;
                }
            }
        }
    });

    // ── Task 2: datagram reader ──────────────────────────────────────────
    let state2c = Arc::clone(&state2);
    let dgram_task = tokio::spawn(async move {
        loop {
            match conn2.read_datagram().await {
                Ok(raw) => {
                    let (header, payload) = match parse_datagram(raw) {
                        Ok(hp) => hp,
                        Err(e) => {
                            warn!("[Datagram] parse error: {e}");
                            continue;
                        }
                    };
                    // Look up the client arc.
                    let client_arc = match state2c.clients.get(client_id) {
                        Some(a) => a,
                        None => continue,
                    };
                    let packet = Packet::new_datagram(
                        Arc::clone(&state2c),
                        client_arc,
                        header.packet_type,
                        header.uid,
                        payload,
                    );
                    dispatch(packet).await;
                }
                Err(e) => {
                    debug!("[Datagram] read error: {e}");
                    break;
                }
            }
        }
    });

    // ── Task 3: push writer ──────────────────────────────────────────────
    let push_task = tokio::spawn(async move {
        while let Some(packet) = push_rx.recv().await {
            // Open a new unidirectional stream per push packet.
            match conn3.open_uni().await {
                Ok(mut send) => {
                    if let Err(e) = write_framed(&mut send, &packet).await {
                        debug!("[Push] write error: {e}");
                    }
                    let _ = send.finish();
                }
                Err(e) => {
                    debug!("[Push] open_uni error: {e}");
                    break;
                }
            }
        }
    });

    // Wait for any task to finish (typically on disconnect).
    tokio::select! {
        _ = bidi_task  => {}
        _ = dgram_task => {}
        _ = push_task  => {}
    }

    // Clean up: leave all instances then remove the client.
    crate::handlers::quit::leave_all_instances(&state, client_id);

    let _ = state.master.emit(
        "client_disconnected",
        EventClientDisconnected {
            id: client_id,
            reason: "disconnected".to_string(),
            kind: "normal".to_string(),
        },
    );

    state.clients.remove(client_id);
    debug!("[Connection] client {client_id} disconnected");

    Ok(())
}
