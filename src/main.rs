#![allow(dead_code)]

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
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

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

    // ── Log buffer setup ──────────────────────────────────────────────────
    let log_buf = Arc::new(parking_lot::Mutex::new(
        crate::utils::log_buffer::LogBuffer::new(10000),
    ));
    let log_forwarder = Arc::new(parking_lot::Mutex::new(None));

    // ── Tracing with LogBufferLayer ───────────────────────────────────────
    let filter = if config.debug {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    let log_layer = crate::utils::log_layer::LogBufferLayer::new(
        Arc::clone(&log_buf),
        Arc::clone(&log_forwarder),
    );

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .with(log_layer)
        .init();

    info!(
        "[Relay] NoxRelay v{} starting on port {}",
        constants::VERSION,
        config.port
    );

    // ── TLS + QUIC endpoint ───────────────────────────────────────────────
    let server_cfg = make_server_config(&[&config.use_address])?;

    let addr: SocketAddr = format!("0.0.0.0:{}", config.port).parse()?;
    let endpoint = Endpoint::server(server_cfg, addr)?;

    info!("[Relay] QUIC endpoint bound to {}", endpoint.local_addr()?);

    // ── Gateway discovery ─────────────────────────────────────────────────
    info!(
        "[Relay] Discovering node gateway from: {}",
        config.node_gateway
    );
    let discovered_gateway = utils::node_discovery::find_node_gateway(&config.node_gateway)
        .await
        .unwrap_or_else(|e| {
            info!(
                "[Relay] Gateway discovery failed: {}, using config value",
                e
            );
            config.node_gateway.clone()
        });

    if discovered_gateway != config.node_gateway {
        info!(
            "[Relay] Discovered gateway: {} (original: {})",
            discovered_gateway, config.node_gateway
        );
    }

    // Create modified config with discovered gateway
    let mut config_modified = (*config).clone();
    config_modified.node_gateway = discovered_gateway;
    let config = Arc::new(config_modified);

    // ── Shared state ──────────────────────────────────────────────────────
    let clients = Arc::new(ClientManager::new());
    let instances = Arc::new(InstanceManager::new());

    let master = Arc::new(MasterClient::new(
        Arc::clone(&config),
        Arc::clone(&clients),
        Arc::clone(&instances),
        Arc::clone(&log_buf),
        Arc::clone(&log_forwarder),
    ));

    let state = Arc::new(AppState::new(
        Arc::clone(&clients),
        Arc::clone(&instances),
        Arc::clone(&master),
        Arc::clone(&config),
        Arc::clone(&log_buf),
    ));

    // ── MasterServer connection ────────────────────────────────────────────
    {
        let master_ref = Arc::clone(&master);
        tokio::spawn(async move {
            info!("[Master] Starting connection...");
            master_ref.run().await;
        });
    }

    // ── QUIC accept loop ──────────────────────────────────────────────────
    if let Err(e) = quic_server::run(state, endpoint).await {
        error!("[main] QUIC server error: {e}");
    }

    info!("[Relay] NoxRelay stopped");
    Ok(())
}
