use anyhow::Result;
use quinn::Endpoint;
use std::sync::Arc;
use tracing::{debug, error, info};

use super::connection::handle_connection;
use crate::handlers::context::AppState;

/// Accept-loop: waits for incoming QUIC connections and spawns a handler for each.
pub async fn run(state: Arc<AppState>, endpoint: Endpoint) -> Result<()> {
    info!("[QuicServer] Listening on {}", endpoint.local_addr()?);

    while let Some(incoming) = endpoint.accept().await {
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            match incoming.await {
                Ok(conn) => {
                    let remote = conn.remote_address();
                    debug!("[QuicServer] New connection from {remote}");
                    if let Err(e) = handle_connection(state, conn).await {
                        error!("[QuicServer] Connection error from {remote}: {e}");
                    }
                }
                Err(e) => {
                    error!("[QuicServer] Connection handshake failed: {e}");
                }
            }
        });
    }

    info!("[QuicServer] Accept loop ended");
    Ok(())
}
