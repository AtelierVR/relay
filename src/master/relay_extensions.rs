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
    let quic_address = config.use_address.as_ref()
        .map(|addr| addr.clone())
        .unwrap_or_else(|| format!("0.0.0.0:{}", config.port));
    a.insert("quic".to_string(), quic_address);
    RelayStatus {
        i: instances.count() as u32,
        c: clients.count() as u32,
        m: max_instances,
        e: ENGINE.to_owned(),
        v: VERSION.to_owned(),
        p: PROTOCOL_VERSION,
        u: start_time_ms,
        s: get_specs(),
        a,
    }
}
