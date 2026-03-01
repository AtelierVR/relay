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
    auth, avatar_changed, custom, disconnect, enter, event, handshake, latency,
    password_requirement, player_update, properties, quit, reliable, segmentation,
    server_config, sessions, teleport, transform, traveling, voice,
};

/// Dispatch a **stream** (bidi) packet.
/// Returns the bytes to send back on the same bidi stream, or `Bytes::new()` for no reply.
pub async fn dispatch_stream(
    state: &AppState,
    client_id: u16,
    header: PacketHeader,
    payload: Bytes,
) -> Bytes {
    let PacketHeader { uid, packet_type } = header;

    debug!(">> stream {:?} uid={uid} client={client_id}", packet_type);

    match packet_type {
        PacketType::Disconnect          => disconnect::handle(state, client_id, uid, payload),
        PacketType::Handshake           => handshake::handle(state, client_id, uid, payload),
        PacketType::Segmentation        => segmentation::handle(state, client_id, uid, payload),
        PacketType::Reliable            => reliable::handle(state, client_id, uid, payload),
        PacketType::Latency             => latency::handle(state, client_id, uid, payload),
        PacketType::Authentication      => auth::handle(state, client_id, uid, payload).await,
        PacketType::Enter               => enter::handle(state, client_id, uid, payload).await,
        PacketType::Quit                => quit::handle(state, client_id, uid, payload).await,
        PacketType::Custom              => custom::handle(state, client_id, uid, payload).await,
        PacketType::PasswordRequirement => password_requirement::handle(state, client_id, uid, payload).await,
        PacketType::Traveling           => traveling::handle(state, client_id, uid, payload).await,
        PacketType::Teleport            => teleport::handle(state, client_id, uid, payload).await,
        PacketType::AvatarChanged       => avatar_changed::handle(state, client_id, uid, payload).await,
        PacketType::ServerConfig        => server_config::handle(state, client_id, uid, payload).await,
        PacketType::Properties          => {
            properties::handle(state, client_id, uid, payload).await;
            Bytes::new()
        }
        PacketType::PlayerUpdate        => player_update::handle(state, client_id, uid, payload).await,
        PacketType::Sessions            => sessions::handle(state, client_id, uid, payload).await,
        PacketType::Event               => {
            event::handle(state, client_id, uid, payload).await;
            Bytes::new()
        }
        // Server-only broadcast types — silently ignore if received from client.
        PacketType::Join | PacketType::Leave => {
            warn!("client {client_id}: received server-only packet {:?}", packet_type);
            Bytes::new()
        }
        // Datagrams arriving on stream path — forward.
        PacketType::Transform => {
            warn!("client {client_id}: Transform arrived on stream path");
            Bytes::new()
        }
        PacketType::Voice => {
            warn!("client {client_id}: Voice arrived on stream path");
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

    debug!(">> dgram  {:?} uid={uid} client={client_id}", packet_type);

    match packet_type {
        PacketType::Transform => transform::handle(state, client_id, uid, payload).await,
        PacketType::Voice     => voice::handle(state, client_id, uid, payload).await,
        _ => {
            debug!("client {client_id}: unexpected packet type {:?} on datagram path", packet_type);
        }
    }
}
