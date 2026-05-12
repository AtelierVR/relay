/// ```packet
/// [Length: u16][UID: u16][Type: u8 = 0x05]
/// [Action: u8]    <- AuthAction
///
/// Action = RequestChallenge (0x00):
///   (no additional fields)
///
/// Action = ResolveChallenge (0x01):
///   [PublicKeyLength: u16][PublicKey: bytes]
///   [SignatureLength: u16][Signature: bytes]
///   [UserId: u32]
///   [Server: string]
/// ```
use bytes::Bytes;
use tracing::{debug, info, warn};

use crate::{
    client::{client::AuthState, user::User},
    handlers::packet::Packet,
    master::messages::EventClientAuthentified,
    proto::{
        buffer::{PacketReader, PacketWriter},
        header::encode_stream_packet,
        packet_type::PacketType,
    },
    utils::{
        crypto::{random_bytes, sha256_fingerprint, verify_pkcs1v15_sha256},
        hex_fmt,
    },
};

#[repr(u8)]
enum AuthAction {
    RequestChallenge = 0,
    ResolveChallenge = 1,
}

#[repr(u8)]
enum AuthResult {
    Success = 0,
    Challenge = 1,
    MasterError = 2,
    Blacklisted = 3,
    Invalid = 4,
    Signature = 5,
    Unknown = 255,
}

pub async fn handle(mut packet: Packet) {
    let state = packet.state.clone();
    let client_id = packet.client_id();
    let uid = packet.uid;
    let payload = packet.payload.clone();

    debug!(
        "[Auth] client {}: raw payload: {}",
        client_id,
        hex_fmt::fmt_bytes(payload.as_ref(), 32)
    );
    let mut r = PacketReader::new(payload);

    // Guard: must have completed handshake.
    let arc = match state.clients.get(client_id) {
        Some(a) => a,
        None => return,
    };
    {
        let c = arc.read();
        if !c.is_handshaked() || c.is_authenticated() {
            return;
        }
    }

    let action = r.read_u8();

    if action == AuthAction::RequestChallenge as u8 {
        on_request_challenge(&mut packet);
    } else if action == AuthAction::ResolveChallenge as u8 {
        on_resolve_challenge(&mut packet, r).await;
    } else {
        packet.reply_raw(make_response(uid, AuthResult::Unknown, Some("Invalid action."), None));
    }
}

fn on_request_challenge(packet: &mut Packet) {
    let client_id = packet.client_id();
    let uid = packet.uid;
    let challenge = random_bytes(16);
    if let Some(arc) = packet.state.clients.get(client_id) {
        let mut c = arc.write();
        c.challenge = challenge.clone();
        c.auth_state = AuthState::ChallengeIssued;
    }
    debug!("[Auth] client {client_id}: challenge requested");

    let mut w = PacketWriter::new();
    w.write_u8(AuthResult::Challenge as u8);
    w.write_u8(challenge.len() as u8);
    w.write_bytes(&challenge);
    packet.reply_raw(encode_stream_packet(uid, PacketType::Authentication, w.finish().as_ref()));
}

async fn on_resolve_challenge(packet: &mut Packet, mut r: PacketReader) {
    let client_id = packet.client_id();
    let uid = packet.uid;
    // Read challenge stored in client.
    let challenge = {
        match packet.state.clients.get(client_id) {
            Some(arc) => arc.read().challenge.clone(),
            None => return,
        }
    };

    if challenge.is_empty() {
        return packet.reply_raw(make_response(uid, AuthResult::Unknown, Some("No challenge requested."), None));
    }

    let public_key = r.read_bytes_prefixed();
    if public_key.is_empty() {
        return packet.reply_raw(make_response(uid, AuthResult::Unknown, Some("No public key provided."), None));
    }

    let signature = r.read_bytes_prefixed();
    if signature.is_empty() {
        return packet.reply_raw(make_response(uid, AuthResult::Unknown, Some("No signature provided."), None));
    }

    debug!(
        "[Auth] client {}: challenge: {}",
        client_id,
        hex_fmt::fmt_bytes(challenge.as_slice(), 32)
    );
    debug!(
        "[Auth] client {}: public_key: {}",
        client_id,
        hex_fmt::fmt_bytes(&public_key, 64)
    );
    debug!(
        "[Auth] client {}: signature: {}",
        client_id,
        hex_fmt::fmt_bytes(&signature, 64)
    );

    if !verify_pkcs1v15_sha256(&challenge, &signature, &public_key) {
        warn!("[Auth] client {}: invalid signature", client_id);
        warn!(
            "[Auth] client {}: verification failed for challenge: {}",
            client_id,
            hex_fmt::to_hex_str(challenge.as_slice(), 16)
        );
        warn!(
            "[Auth] client {}: signature (first 32 bytes): {}",
            client_id,
            hex_fmt::to_hex_str(&signature[..signature.len().min(32)], 32)
        );
        warn!(
            "[Auth] client {}: public_key (first 64 bytes): {}",
            client_id,
            hex_fmt::to_hex_str(&public_key[..public_key.len().min(64)], 64)
        );
        return packet.reply_raw(make_response(uid, AuthResult::Signature, Some("Invalid signature."), None));
    }

    let user_id = r.read_u32();
    let server = r.read_string().unwrap_or_default();
    let fingerprint = sha256_fingerprint(&public_key);

    let resolve = match packet.state
        .master
        .resolve_user(user_id, server.clone(), fingerprint)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!("[Auth] client {client_id}: master error: {e}");
            return packet.reply_raw(make_response(uid, AuthResult::MasterError, Some("Failed to contact master server."), None));
        }
    };

    match resolve.result.as_str() {
        "success" => {
            if let Some(info) = resolve.user {
                let user = User {
                    id: info.id,
                    display_name: info.display,
                    address: info.server,
                };
                if let Some(arc) = packet.state.clients.get(client_id) {
                    let mut c = arc.write();
                    c.user = Some(user.clone());
                    c.auth_state = AuthState::Authenticated;
                }
                info!(
                    "[Auth] client {} authenticated: user={}@{} display={}",
                    client_id, user.id, user.address, user.display_name
                );
                let user_identifier = format!("{}@{}", user.id, user.address);
                let _ = packet.state.master.emit("client_authentified", EventClientAuthentified {
                    id: client_id,
                    user: user_identifier,
                });
                let mut w = PacketWriter::new();
                w.write_u8(AuthResult::Success as u8);
                w.write_u32(user.id);
                w.write_string(&user.address);
                w.write_string(&user.display_name);
                return packet.reply_raw(encode_stream_packet(uid, PacketType::Authentication, w.finish().as_ref()));
            }
            packet.reply_raw(make_response(uid, AuthResult::Unknown, Some("No user data in response."), None));
        }
        "blacklisted" => {
            if let Some(arc) = packet.state.clients.get(client_id) {
                arc.write().user = None;
            }
            let mut w = PacketWriter::new();
            w.write_u8(AuthResult::Blacklisted as u8);
            w.write_i64(resolve.expire_at);
            w.write_string(resolve.reason.as_deref().unwrap_or("Blacklisted"));
            packet.reply_raw(encode_stream_packet(uid, PacketType::Authentication, w.finish().as_ref()));
        }
        _ => {
            if let Some(arc) = packet.state.clients.get(client_id) {
                arc.write().user = None;
            }
            packet.reply_raw(make_response(
                uid,
                AuthResult::Invalid,
                resolve.reason.as_deref().or(Some("Invalid user.")),
                None,
            ));
        }
    }
}

fn make_response(
    uid: u16,
    result: AuthResult,
    reason: Option<&str>,
    _extra: Option<&[u8]>,
) -> Bytes {
    let mut w = PacketWriter::new();
    w.write_u8(result as u8);
    if let Some(r) = reason {
        w.write_string(r);
    }
    encode_stream_packet(uid, PacketType::Authentication, w.finish().as_ref())
}
