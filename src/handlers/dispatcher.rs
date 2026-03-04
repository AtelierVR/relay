/// Central packet dispatcher — routes incoming packets to the correct handler.
///
/// Stream packets produce an optional response `Bytes` that the bidi task sends back.
/// Datagram packets are fire-and-forget (no response bytes returned).
use bytes::Bytes;
use tracing::{debug, warn};

use crate::{
    handlers::context::AppState,
    proto::{header::PacketHeader, packet_type::PacketType},
};

use super::{
    auth, avatar_changed, custom, disconnect, enter, event, handshake, latency, player_update,
    properties, quit, reliable, server_config, sessions, teleport, transform, traveling, voice,
};

/// Packets that should not be logged (too frequent or low-value)
const SILENT_PACKETS: &[PacketType] = &[
    PacketType::Latency,
    PacketType::Transform,
    PacketType::Properties,
];

/// Dispatch a **stream** (bidi) packet.
/// Returns the bytes to send back on the same bidi stream, or `Bytes::new()` for no reply.
pub async fn dispatch_stream(
    state: &AppState,
    client_id: u16,
    header: PacketHeader,
    payload: Bytes,
) -> Bytes {
    let PacketHeader { uid, packet_type } = header;

    if !SILENT_PACKETS.contains(&packet_type) {
        debug!(
            "[Stream] packet={:?} uid={} client={}",
            packet_type, uid, client_id
        );
    }

    match packet_type {
        PacketType::Disconnect => disconnect::handle(state, client_id, uid, payload),
        PacketType::Handshake => handshake::handle(state, client_id, uid, payload),
        PacketType::Reliable => reliable::handle(state, client_id, uid, payload),
        PacketType::Latency => latency::handle(state, client_id, uid, payload),
        PacketType::Authentication => auth::handle(state, client_id, uid, payload).await,
        PacketType::Enter => enter::handle(state, client_id, uid, payload).await,
        PacketType::Quit => quit::handle(state, client_id, uid, payload).await,
        PacketType::Custom => custom::handle(state, client_id, uid, payload).await,
        PacketType::Traveling => traveling::handle(state, client_id, uid, payload).await,
        PacketType::Teleport => teleport::handle(state, client_id, uid, payload).await,
        PacketType::AvatarChanged => avatar_changed::handle(state, client_id, uid, payload).await,
        PacketType::ServerConfig => server_config::handle(state, client_id, uid, payload).await,
        PacketType::Properties => {
            properties::handle(state, client_id, uid, payload).await;
            Bytes::new()
        }
        PacketType::PlayerUpdate => player_update::handle(state, client_id, uid, payload).await,
        PacketType::Sessions => sessions::handle(state, client_id, uid, payload).await,
        PacketType::Event => {
            event::handle(state, client_id, uid, payload).await;
            Bytes::new()
        }
        // Unsupported or deprecated packets — silently ignore.
        PacketType::Segmentation | PacketType::PasswordRequirement => {
            debug!(
                "[Stream] client {}: received deprecated packet {:?}",
                client_id, packet_type
            );
            Bytes::new()
        }
        // Server-only broadcast types — silently ignore if received from client.
        PacketType::Join | PacketType::Leave => {
            warn!(
                "[Stream] client {}: received server-only packet {:?}",
                client_id, packet_type
            );
            Bytes::new()
        }
        // Datagrams arriving on stream path — forward.
        PacketType::Transform => {
            warn!(
                "[Stream] client {}: Transform packet arrived on stream path",
                client_id
            );
            Bytes::new()
        }
        PacketType::Voice => {
            warn!(
                "[Stream] client {}: Voice packet arrived on stream path",
                client_id
            );
            Bytes::new()
        }
    }
}

/// Dispatch a **datagram** packet. These are fire-and-forget.
pub async fn dispatch_datagram(
    state: &AppState,
    client_id: u16,
    header: PacketHeader,
    payload: Bytes,
) {
    let PacketHeader { uid, packet_type } = header;

    if !SILENT_PACKETS.contains(&packet_type) {
        debug!(
            "[Datagram] packet={:?} uid={} client={}",
            packet_type, uid, client_id
        );
    }

    match packet_type {
        PacketType::Transform => transform::handle(state, client_id, uid, payload).await,
        PacketType::Voice => voice::handle(state, client_id, uid, payload).await,
        _ => {
            debug!(
                "[Datagram] client {}: unexpected packet type {:?} on datagram path",
                client_id, packet_type
            );
        }
    }
}
