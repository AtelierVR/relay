mod avatar;
mod client;
mod config;
mod constants;
mod handlers;
mod instance;
mod master;
mod player;
mod proto;
mod transport;
mod utils;

#[cfg(test)]
mod tests;

use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;
use quinn::Endpoint;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use crate::{
    client::ClientManager,
    config::Config,
    handlers::context::AppState,
    instance::InstanceManager,
    master::MasterClient,
    transport::{quic_server, tls::make_server_config},
};

#[tokio::main]
async fn main() -> Result<()> {
    // ── Config ────────────────────────────────────────────────────────────
    let config = Arc::new(Config::load().expect("failed to load config"));

    // ── Tracing ───────────────────────────────────────────────────────────
    let filter = if config.debug {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    info!("NoxRelay v{} starting on port {}", constants::VERSION, config.port);

    // ── TLS + QUIC endpoint ───────────────────────────────────────────────
    let server_cfg = make_server_config(&[&config.use_address])?;

    let addr: SocketAddr = format!("0.0.0.0:{}", config.port).parse()?;
    let endpoint = Endpoint::server(server_cfg, addr)?;

    info!("QUIC endpoint bound to {}", endpoint.local_addr()?);

    // ── Shared state ──────────────────────────────────────────────────────
    let clients   = Arc::new(ClientManager::new());
    let instances = Arc::new(InstanceManager::new());
    let master    = Arc::new(MasterClient::new(Arc::clone(&config)));

    // Share the log buffer between AppState and MasterClient.
    let log_buf = Arc::clone(&master.log_buf);

    let state = Arc::new(AppState::new(
        Arc::clone(&clients),
        Arc::clone(&instances),
        Arc::clone(&master),
        Arc::clone(&config),
        log_buf,
    ));

    // ── MasterServer connection ────────────────────────────────────────────
    {
        let master_ref = Arc::clone(&master);
        tokio::spawn(async move {
            master_ref.run(|| {
                info!("[Master] Connection established");
            }).await;
        });
    }

    // ── QUIC accept loop ──────────────────────────────────────────────────
    if let Err(e) = quic_server::run(state, endpoint).await {
        error!("[main] QUIC server error: {e}");
    }

    info!("NoxRelay stopped");
    Ok(())
}
