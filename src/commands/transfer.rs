use super::{find_client, Command, CommandContext};
use crate::{
    proto::{header::encode_stream_packet, packet_type::PacketType},
    transfer,
};
use tracing::{info, warn};

/// `transfer <client> <host:port> <token> [reason]` — ask a client to reconnect to another relay,
/// carrying the token it must present in its next handshake.
pub struct TransferCommand;

impl Command for TransferCommand {
    fn name(&self) -> &str {
        "transfer"
    }

    fn description(&self) -> &str {
        "Move a client to another relay: transfer <id|ip:port> <host:port> <token> [reason]"
    }

    fn execute(&self, context: &CommandContext, args: &[&str]) {
        if args.len() < 3 {
            info!("Usage: transfer <id|ip:port> <host:port> <token> [reason]");
            return;
        }

        let Some((id, client)) = find_client(context, args[0]) else {
            warn!("No client matching '{}'", args[0]);
            return;
        };

        let address = args[1];
        let token = parse_token(args[2]);
        let reason = args[3..].join(" ");

        if token.is_empty() {
            warn!("[Transfer] the token cannot be empty");
            return;
        }

        let user = client.read().user.clone();
        let Some(user) = user else {
            warn!("[Transfer] client {id} is not authenticated");
            return;
        };

        let payload = transfer::server_payload(address, &token, &reason);
        let packet = encode_stream_packet(0, PacketType::Transfer, payload.as_ref());
        if client.read().try_push(packet) {
            // The request is now enforceable: a client that stays is removed after the timeout.
            transfer::schedule(&client, None, context.config.transfer_timeout);

            info!(
                "[Transfer] client {} (user={}@{}) -> {} ({} token byte(s)): {}",
                id,
                user.id,
                user.address,
                address,
                token.len(),
                if reason.is_empty() {
                    "no reason"
                } else {
                    reason.as_str()
                }
            );
        } else {
            warn!("[Transfer] failed to queue the transfer packet for client {id}");
        }
    }
}

/// Tokens are opaque client-unique byte sequences. The console takes them as hex when the argument
/// looks like it, and falls back to its UTF-8 bytes otherwise.
fn parse_token(raw: &str) -> Vec<u8> {
    let hex_part = raw
        .strip_prefix("0x")
        .or_else(|| raw.strip_prefix("0X"))
        .unwrap_or(raw);

    if !hex_part.is_empty() && hex_part.len().is_multiple_of(2) {
        if let Ok(bytes) = hex::decode(hex_part) {
            return bytes;
        }
    }

    raw.as_bytes().to_vec()
}
