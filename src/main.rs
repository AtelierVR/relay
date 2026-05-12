#![allow(dead_code)]

mod avatar;
mod client;
mod commands;
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
    proto::{buffer::PacketWriter, header::encode_datagram, packet_type::PacketType},
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
    let server_cfg = make_server_config(
        &[&config.node_address],
        config.connection_timeout,
        config.keep_alive_interval,
    )?;

    let addr: SocketAddr = format!("0.0.0.0:{}", config.port).parse()?;
    let endpoint = Endpoint::server(server_cfg, addr)?;

    info!("[Relay] QUIC endpoint bound to {}", endpoint.local_addr()?);

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

    // ── Load Balancing periodic updates ────────────────────────────────────
    {
        let state_ref = Arc::clone(&state);
        tokio::spawn(async move {
            info!("[LoadBalance] Starting periodic update task...");
            load_balancing_task(state_ref).await;
        });
    }

    // ── TEST: spam Message(0x02) broadcast ───────────────────────────────
    // {
    //     let state_ref = Arc::clone(&state);
    //     tokio::spawn(async move {
    //         spam_message_task(state_ref).await;
    //     });
    // }

    // ── QUIC accept loop ──────────────────────────────────────────────────
    if let Err(e) = quic_server::run(state, endpoint).await {
        error!("[main] QUIC server error: {e}");
    }

    info!("[Relay] NoxRelay stopped");
    Ok(())
}

/// Periodic task that updates load balancing and broadcasts changes
async fn load_balancing_task(state: Arc<AppState>) {
    use crate::handlers::server_config::{broadcast_config_change, ServerConfigFlags};

    loop {
        // Wait for the configured update interval
        let interval = state.config.load_balancing.update_interval as u64 * 1000;
        tokio::time::sleep(tokio::time::Duration::from_millis(interval)).await;

        // Update adaptive settings for each instance
        for iid in 0..=254u8 {
            if let Some(inst_arc) = state.instances.get(iid) {
                let (new_tps, new_threshold, changed) = {
                    let mut inst = inst_arc.write();
                    inst.update_adaptive_settings(&state.config.load_balancing)
                };

                if changed {
                    let player_count = inst_arc.read().player_count();
                    info!(
                        "[LoadBalance] Instance {} updated: TPS={} threshold={:.4} (players={})",
                        iid, new_tps, new_threshold, player_count
                    );

                    // Broadcast the change to all players in the instance
                    broadcast_config_change(
                        &state,
                        &inst_arc,
                        iid,
                        ServerConfigFlags::TPS | ServerConfigFlags::THRESHOLD,
                    );
                }
            }
        }
    }
}

async fn spam_message_task(state: Arc<AppState>) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100));
    loop {
        interval.tick().await;
        let mut w = PacketWriter::new();
        w.write_string("TestTest");
        let pkt = encode_datagram(0, PacketType::Message, w.finish().as_ref());
        state.clients.for_each(|_, arc| {
            arc.read().send_datagram(pkt.clone());
        });
    }
}
