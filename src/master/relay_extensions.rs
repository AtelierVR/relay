use std::sync::Arc;

use crate::{
    client::ClientManager,
    config::Config,
    constants::{ENGINE, PROTOCOL_VERSION, VERSION},
    instance::InstanceManager,
    utils::system_specs::get_specs,
};

use super::messages::RelayStatus;

/// Build the relay status snapshot for the MasterServer.
pub fn build_status(
    clients: &Arc<ClientManager>,
    instances: &Arc<InstanceManager>,
    max_instances: u8,
    start_time_ms: i64,
    config: &Arc<Config>,
) -> RelayStatus {
    let mut a = std::collections::HashMap::new();
    let quic_address = config
        .use_address
        .clone()
        .unwrap_or_else(|| format!("0.0.0.0:{}", config.port));
    a.insert("quic".to_string(), quic_address);
    RelayStatus {
        instance_count: instances.count() as u32,
        client_count: clients.count() as u32,
        max_instances,
        engine: ENGINE.to_owned(),
        version: VERSION.to_owned(),
        protocol_version: PROTOCOL_VERSION,
        start_time_ms,
        specs: get_specs(),
        addresses: a,
    }
}
