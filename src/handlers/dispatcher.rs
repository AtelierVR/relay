/// Central packet dispatcher — routes incoming packets to the correct handler.
///
/// Both stream (bidi) and datagram packets go through the same [`dispatch`]
/// function. The [`Packet`] carries whether it arrived on a stream or as a
/// datagram (via `packet.is_stream()`), and handlers use `packet.reply*()`
/// for stream responses or `client.send()` for push / datagram broadcasts.
use tracing::{debug, warn};

use crate::proto::packet_type::PacketType;

use super::{
    auth, avatar_changed, custom, disconnect, enter, event, handshake, latency, message,
    packet::Packet, player_update, properties, quit, reliable, server_config, sessions, stream,
    teleport, transform, traveling,
};

/// Packets that should not be logged (too frequent or low-value)
const SILENT_PACKETS: &[PacketType] = &[
    PacketType::Latency,
    PacketType::Transform,
    PacketType::Properties,
    PacketType::Event,
];

/// Dispatch any incoming packet — stream or datagram — to the matching handler.
pub async fn dispatch(packet: Packet) {
    let ptype = packet.packet_type;
    let uid = packet.uid;
    let client_id = packet.client_id();
    let is_stream = packet.is_stream();

    if !SILENT_PACKETS.contains(&ptype) {
        debug!(
            "[{}] packet={:?} uid={} client={}",
            if is_stream { "Stream" } else { "Datagram" },
            ptype,
            uid,
            client_id
        );
    }

    match ptype {
        // ── Stream packets ────────────────────────────────────────────────
        PacketType::Disconnect => disconnect::handle(packet),
        PacketType::Handshake => handshake::handle(packet),
        PacketType::Reliable => reliable::handle(packet),
        PacketType::Latency => latency::handle(packet),
        PacketType::Authentication => auth::handle(packet).await,
        PacketType::Enter => enter::handle(packet).await,
        PacketType::Quit => quit::handle(packet).await,
        PacketType::Custom => custom::handle(packet).await,
        PacketType::Traveling => traveling::handle(packet).await,
        PacketType::Teleport => teleport::handle(packet).await,
        PacketType::AvatarChanged => avatar_changed::handle(packet).await,
        PacketType::ServerConfig => server_config::handle(packet).await,
        PacketType::Properties => properties::handle(packet).await,
        PacketType::PlayerUpdate => player_update::handle(packet).await,
        PacketType::Sessions => sessions::handle(packet).await,
        PacketType::Event => event::handle(packet).await,
        PacketType::Message => message::handle(packet).await,

        // ── Datagram packets ──────────────────────────────────────────────
        PacketType::Transform => transform::handle(packet).await,
        PacketType::Stream => stream::handle(packet).await,

        // ── Deprecated / server-only — silently ignore ───────────────────
        PacketType::PasswordRequirement => {
            debug!(
                "[{}] client {}: deprecated packet {:?}",
                if is_stream { "Stream" } else { "Datagram" },
                client_id,
                ptype
            );
        }
        PacketType::Join | PacketType::Leave => {
            warn!(
                "[{}] client {}: server-only packet {:?}",
                if is_stream { "Stream" } else { "Datagram" },
                client_id,
                ptype
            );
        }
    }
}
