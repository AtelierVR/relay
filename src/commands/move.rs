use super::{find_client, Command, CommandContext};
use crate::{
    proto::{header::encode_stream_packet, packet_type::PacketType},
    transfer,
};
use tracing::{info, warn};

/// `move <client> <room> [reason]` — ask a client to switch to another room of this relay.
///
/// Same `Transfer` packet as a relay change, with `TransferOp::Room`: the transport is kept and
/// only the room membership is rebuilt (same world only).
pub struct MoveCommand;

impl Command for MoveCommand {
    fn name(&self) -> &str {
        "move"
    }

    fn description(&self) -> &str {
        "Move a client to another room: move <id|ip:port> <room> [reason]"
    }

    fn execute(&self, context: &CommandContext, args: &[&str]) {
        if args.len() < 2 {
            info!("Usage: move <id|ip:port> <room> [reason]");
            return;
        }

        let Some((id, client)) = find_client(context, args[0]) else {
            warn!("No client matching '{}'", args[0]);
            return;
        };

        let Ok(iid) = args[1].parse::<u8>() else {
            warn!("Invalid room id '{}' (expected a number)", args[1]);
            return;
        };

        let Some(instance) = context.state.instances.get(iid) else {
            warn!("No room with id {iid}");
            return;
        };

        let reason = args[2..].join(" ");

        // Refuse a no-op: the client is already there.
        let current = instance
            .read()
            .get_players()
            .iter()
            .any(|p| p.client_id == id);
        if current {
            warn!("[Move] client {id} is already in room {iid}");
            return;
        }

        let payload = transfer::room_payload(iid, &reason);
        let packet = encode_stream_packet(0, PacketType::Transfer, payload.as_ref());

        if client.read().try_push(packet) {
            // The request is now enforceable: the client cannot stay in its room after the timeout.
            transfer::schedule(&client, Some(iid), context.config.transfer_timeout);
            info!(
                "[Move] client {id} -> room {iid}: {}",
                if reason.is_empty() {
                    "no reason"
                } else {
                    reason.as_str()
                }
            );
        } else {
            warn!("[Move] failed to queue the move packet for client {id}");
        }
    }
}
