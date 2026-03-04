use std::sync::Arc;

use anyhow::Result;
use quinn::Connection;
use tracing::{debug, warn};

use crate::{
    client::client::new_client,
    handlers::{
        context::AppState,
        dispatcher::{dispatch_datagram, dispatch_stream},
    },
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
    let (client_arc, mut push_rx) = new_client(client_id);
    {
        let c = client_arc.write();
        // Store the remote address string for relay_extensions.
        drop(c);
    }
    state.clients.add(Arc::clone(&client_arc));
    debug!("[Connection] client {client_id} connected from {remote_addr}");

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

                        // Dispatch and collect response.
                        let response = dispatch_stream(&state, client_id, header, payload).await;

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
                    // Datagrams are fire-and-forget; response (if any) goes via push channel.
                    dispatch_datagram(&state2c, client_id, header, payload).await;
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
    state.clients.remove(client_id);
    debug!("[Connection] client {client_id} disconnected");

    Ok(())
}
