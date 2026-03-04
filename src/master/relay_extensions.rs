use std::sync::Arc;

use crate::{
    client::ClientManager,
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
    port: u16,
) -> RelayStatus {
    let mut a = std::collections::HashMap::new();
    a.insert("quic".to_string(), format!("0.0.0.0:{}", port));
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
