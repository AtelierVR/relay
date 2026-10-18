#![allow(dead_code)]

mod avatar;
mod cli;
mod client;
mod commands;
mod config;
mod constants;
mod handlers;
mod instance;
mod master;
mod player;
mod proto;
mod rate;
mod shutdown;
mod transfer;
mod transport;
mod utils;

#[cfg(test)]
mod tests;

use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;
use quinn::Endpoint;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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
    // The filter layer is reloadable so the `debug` command can flip the level
    // (info <-> debug) at runtime without restarting the relay.
    let filter_layer = crate::utils::logging::build(config.debug);

    let log_layer = crate::utils::log_layer::LogBufferLayer::new(
        Arc::clone(&log_buf),
        Arc::clone(&log_forwarder),
    );

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(tracing_subscriber::fmt::layer().with_writer(crate::cli::LogMakeWriter))
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

    // Build CommandRegistry now that state exists
    {
        let command_context = crate::commands::CommandContext::new(
            Arc::clone(&config),
            Arc::clone(&clients),
            Arc::clone(&instances),
            master.start_time_ms,
            vec![],
            Arc::clone(&state),
        );
        state
            .commands
            .lock()
            .unwrap()
            .replace(crate::commands::CommandRegistry::new(command_context));
    }

    // Wire AppState into MasterClient for command execution
    *master.state.lock() = Some(Arc::clone(&state));

    // ── Local command console ──────────────────────────────────────────────
    // Shares the master's CommandRegistry with the process stdin: interactive TTY gets a
    // prompt and history, a piped stdin is still read (without a prompt).
    crate::cli::spawn(Arc::clone(&state), Arc::clone(&state.shutdown));

    // ── MasterServer connection (skipped in standalone) ────────────────────
    if config.is_standalone() {
        info!("[Standalone] master connection disabled, self-hosting instance(s)");
        if config.instances.is_empty() {
            let instance = crate::instance::Instance::new(0, 0, Some(Arc::clone(&state)));
            state.instances.add(instance);
        } else {
            for (i, spec) in config.instances.iter().enumerate() {
                let mut instance =
                    crate::instance::Instance::new(i as u8, 0, Some(Arc::clone(&state)));
                instance.capacity = spec.capacity;
                instance.set_password(spec.password.clone());
                if let Some(world) = &spec.world {
                    match world {
                        crate::config::WorldConfig::Identifier(id) => {
                            match parse_world_identifier(id) {
                                Some(w) => instance.world = crate::instance::TravelTarget::Node(w),
                                None => warn!(
                                    "[Standalone] instance {i}: invalid world identifier '{id}'"
                                ),
                            }
                        }
                        crate::config::WorldConfig::Bundle { url, hash, size } => {
                            match decode_hash(hash) {
                                Some(h) => {
                                    instance.world = crate::instance::TravelTarget::Bundle(
                                        crate::instance::world::WorldBundle {
                                            url: url.clone(),
                                            hash: h,
                                            size: *size,
                                        },
                                    );
                                }
                                None => {
                                    warn!("[Standalone] instance {i}: invalid world hash '{hash}'")
                                }
                            }
                        }
                    }
                }
                state.instances.add(instance);
                info!("[Standalone] instance {i} hosted");
            }
        }
    } else {
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

    // ── Transfer enforcement ───────────────────────────────────────────────
    // A `Transfer` is only a request: clients that do not apply it are removed once the grace
    // period expires (`transfer_timeout`, 0 = never enforce).
    {
        let state_ref = Arc::clone(&state);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
            loop {
                interval.tick().await;
                crate::transfer::enforce(&state_ref);
            }
        });
    }

    // ── TEST: spam Message(0x02) broadcast ───────────────────────────────
    // {
    //     let state_ref = Arc::clone(&state);
    //     tokio::spawn(async move {
    //         spam_message_task(state_ref).await;
    //     });
    // }

    // ── QUIC accept loop (with graceful Ctrl+C shutdown) ───────────────────
    let accept_endpoint = endpoint.clone();
    let accept_state = Arc::clone(&state);

    tokio::select! {
        result = quic_server::run(accept_state, accept_endpoint) => {
            if let Err(e) = result {
                error!("[main] QUIC server error: {e}");
            }
        }
        signal = shutdown_signal() => {
            run_shutdown(&state, &endpoint, &format!("{signal} received")).await;
        }
        _ = state.shutdown.notified() => {
            run_shutdown(&state, &endpoint, "Stop requested").await;
        }
    }

    info!("[Relay] NoxRelay stopped");
    Ok(())
}

/// Disconnects every client and closes the endpoint, while a second signal still forces an
/// immediate exit.
async fn run_shutdown(state: &Arc<AppState>, endpoint: &Endpoint, reason: &str) {
    info!("[Relay] {reason} — disconnecting all clients and shutting down...");

    // A second signal forces an immediate exit instead of waiting.
    tokio::spawn(async {
        let second = shutdown_signal().await;
        warn!("[Relay] Second {second} received — forcing exit");
        std::process::exit(130);
    });

    crate::shutdown::graceful_shutdown(state, endpoint, "Server is shutting down").await;
}

/// Resolves when the user requests a shutdown: `Ctrl+C` on every platform, and
/// additionally `Ctrl+Break` on Windows (handy when Ctrl+C is swallowed by the
/// parent process/host).
async fn shutdown_signal() -> &'static str {
    #[cfg(windows)]
    {
        let mut ctrl_break = match tokio::signal::windows::ctrl_break() {
            Ok(handler) => handler,
            Err(e) => {
                warn!("[Relay] failed to install Ctrl+Break handler: {e}");
                let _ = tokio::signal::ctrl_c().await;
                return "Ctrl+C";
            }
        };

        tokio::select! {
            _ = tokio::signal::ctrl_c() => "Ctrl+C",
            _ = ctrl_break.recv() => "Ctrl+Break",
        }
    }

    #[cfg(not(windows))]
    {
        let _ = tokio::signal::ctrl_c().await;
        "Ctrl+C"
    }
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

/// Decode a hex string into bytes (even length, valid hex), or `None` otherwise.
fn decode_hash(hex: &str) -> Option<Vec<u8>> {
    let hex = hex.trim();
    if hex.is_empty() || hex.len() % 2 != 0 {
        return None;
    }

    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect()
}

/// Parse a world identifier string (`"[type:]id[?v=version][@address]"`) into a `World`.
fn parse_world_identifier(raw: &str) -> Option<crate::instance::world::World> {
    let mut rest = raw.trim();

    let address = match rest.rsplit_once('@') {
        Some((before, addr)) => {
            rest = before;
            addr.to_string()
        }
        None => String::new(),
    };

    let mut version = 0u16;
    if let Some((before, query)) = rest.split_once('?') {
        rest = before;
        for kv in query.split('&') {
            if let Some(v) = kv.strip_prefix("v=") {
                version = v.parse().unwrap_or(version);
            }
        }
    }

    // Drop an optional single-char `type:` prefix.
    let id = match rest.split_once(':') {
        Some((t, id)) if t.len() == 1 => id,
        _ => rest,
    };

    let master_id = id.parse::<u32>().ok()?;
    let address = if address.is_empty() {
        crate::constants::SAFE_LOCAL_ADDRESS.to_string()
    } else {
        address
    };
    Some(crate::instance::world::World::new(
        master_id, address, version,
    ))
}
