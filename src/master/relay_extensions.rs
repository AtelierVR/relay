use std::sync::Arc;

use crate::{
    client::ClientManager,
    constants::{PROTOCOL_VERSION, VERSION},
    instance::InstanceManager,
    player::PlayerFlags,
    instance::InstanceFlags,
    utils::system_specs::get_specs,
};

use super::messages::{RelayClient, RelayInstance, RelayPlayer, RelayStatus};

/// Build the full relay status snapshot for the MasterServer.
pub fn build_status(
    clients: &Arc<ClientManager>,
    instances: &Arc<InstanceManager>,
    max_instances: u8,
    start_time_ms: i64,
) -> RelayStatus {
    let instance_list = instances
        .all()
        .iter()
        .map(|arc| {
            let inst = arc.read();
            to_relay_instance(&inst)
        })
        .collect();

    let client_list = {
        let mut list = Vec::new();
        clients.for_each(|id, arc| {
            let c = arc.read();
            list.push(RelayClient {
                i: c.id,
                p: c.platform.clone(),
                e: c.engine.clone(),
                a: String::new(), // QUIC: remote addr filled by connection layer
                u: c.user.as_ref().map(|u| u.to_identifier()),
            });
        });
        list
    };

    RelayStatus {
        i: instance_list,
        c: client_list,
        m: max_instances,
        v: VERSION.to_owned(),
        p: PROTOCOL_VERSION,
        u: start_time_ms,
        s: get_specs(),
    }
}

pub fn to_relay_instance(inst: &crate::instance::Instance) -> RelayInstance {
    RelayInstance {
        i: inst.internal_id,
        m: inst.master_id,
        p: inst.get_players().iter().map(to_relay_player).collect(),
        f: flags_to_strings(inst.flags),
        w: inst.world.to_identifier(),
        c: inst.capacity,
    }
}

pub fn to_relay_player(player: &crate::player::Player) -> RelayPlayer {
    RelayPlayer {
        p: player.id,
        c: player.client_id,
        d: player.display.clone().unwrap_or_default(),
        f: player_flags_to_strings(player.flags),
    }
}

fn flags_to_strings(flags: InstanceFlags) -> Vec<String> {
    let all = [
        (InstanceFlags::IS_PUBLIC,      "is_public"),
        (InstanceFlags::USE_PASSWORD,   "use_password"),
        (InstanceFlags::USE_WHITELIST,  "use_whitelist"),
        (InstanceFlags::AUTHORIZE_BOT,  "authorize_bot"),
        (InstanceFlags::ALLOW_OVERLOAD, "allow_overload"),
    ];
    all.iter()
        .filter(|(f, _)| flags.contains(*f))
        .map(|(_, s)| s.to_string())
        .collect()
}

fn player_flags_to_strings(flags: PlayerFlags) -> Vec<String> {
    let all = [
        (PlayerFlags::IS_BOT,             "is_bot"),
        (PlayerFlags::INSTANCE_MASTER,    "instance_master"),
        (PlayerFlags::INSTANCE_MODERATOR, "instance_moderator"),
        (PlayerFlags::INSTANCE_OWNER,     "instance_owner"),
        (PlayerFlags::GUILD_MODERATOR,    "guild_moderator"),
        (PlayerFlags::MASTER_MODERATOR,   "master_moderator"),
        (PlayerFlags::WORLD_OWNER,        "world_owner"),
        (PlayerFlags::WORLD_MODERATOR,    "world_moderator"),
        (PlayerFlags::AUTH_UNVERIFIED,    "auth_unverified"),
        (PlayerFlags::HIDE_IN_LIST,       "hide_in_list"),
        (PlayerFlags::HAS_CUSTOM_DISPLAY, "has_custom_display"),
    ];
    all.iter()
        .filter(|(f, _)| flags.contains(*f))
        .map(|(_, s)| s.to_string())
        .collect()
}
