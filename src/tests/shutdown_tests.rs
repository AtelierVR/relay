/// Tests for `shutdown::graceful_shutdown`.
///
/// C# spec: a server-initiated `Disconnect` packet just carries an optional
/// reason string, so clients must receive it before the transport is closed.
use std::sync::Arc;

use crate::{
    client::client::new_client,
    proto::{buffer::PacketReader, frame::read_framed, packet_type::PacketType},
    shutdown::graceful_shutdown,
};

use super::helpers::{connect_pair, decode_stream, make_state};

#[tokio::test]
async fn graceful_shutdown_notifies_and_closes_clients() {
    let state = make_state();

    let (server_ep, server_conn, client_ep, client_conn) = connect_pair().await;

    // Register the server-side connection as a client of the relay.
    let (client_arc, _push_rx) = new_client(7, Arc::new(server_conn));
    state.clients.add(client_arc);
    assert_eq!(state.clients.count(), 1);

    // The client reads the notification while the server shuts down.
    let read = async {
        let mut recv = client_conn
            .accept_uni()
            .await
            .expect("client should receive a uni-stream");
        read_framed(&mut recv).await.expect("framed disconnect packet")
    };

    let ((), framed) = tokio::join!(
        graceful_shutdown(&state, &server_ep, "Server is shutting down"),
        read
    );

    // ── Packet payload ───────────────────────────────────────────────────
    let (uid, ptype, payload) = decode_stream(framed);
    assert_eq!(ptype, PacketType::Disconnect, "must be a Disconnect packet");
    assert_eq!(uid, 0, "server-initiated packets use uid 0");

    let mut r = PacketReader::new(payload);
    assert_eq!(
        r.read_string().as_deref(),
        Some("Server is shutting down"),
        "reason must be forwarded to the client"
    );

    // ── Connection closed with the shutdown error code ───────────────────
    let err = client_conn.closed().await;
    match err {
        quinn::ConnectionError::ApplicationClosed(close) => {
            assert_eq!(
                close.error_code,
                quinn::VarInt::from_u32(1),
                "unexpected application error code"
            );
        }
        other => panic!("expected ApplicationClosed, got {other:?}"),
    }

    client_ep.close(0u32.into(), b"test done");
    server_ep.close(0u32.into(), b"test done");
}
