use bytes::{Buf, BufMut, Bytes, BytesMut};

use super::packet_type::PacketType;
use crate::constants::STREAM_HEADER_SIZE;

/// Decoded packet header, shared between stream and datagram paths.
#[derive(Debug, Clone, Copy)]
pub struct PacketHeader {
    /// Client-assigned request UID used for response correlation.
    pub uid: u16,
    pub packet_type: PacketType,
}

/// Decode a stream packet header.
///
/// Wire format: [Length: u16 BE][UID: u16 BE][Type: u8]
///
/// Returns `(header, payload_bytes)` or an error string.
pub fn decode_stream_header(mut data: Bytes) -> Result<(PacketHeader, Bytes), String> {
    if data.len() < STREAM_HEADER_SIZE {
        return Err(format!(
            "stream header too short: {} < {}",
            data.len(),
            STREAM_HEADER_SIZE
        ));
    }

    let length = data.get_u16() as usize;
    let uid = data.get_u16();
    let type_byte = data.get_u8();

    let packet_type =
        PacketType::try_from(type_byte).map_err(|b| format!("unknown packet type: 0x{:02X}", b))?;

    // Payload is everything after the 5-byte header, up to `length`.
    let payload_len = length.saturating_sub(STREAM_HEADER_SIZE);
    if data.len() < payload_len {
        return Err(format!(
            "payload truncated: {} < {}",
            data.len(),
            payload_len
        ));
    }

    let payload = data.split_to(payload_len);
    Ok((PacketHeader { uid, packet_type }, payload))
}

/// Decode a datagram packet header.
///
/// Wire format: [Length: u16 BE][UID: u16 BE][Type: u8]  — identical to stream.
pub fn decode_datagram_header(data: Bytes) -> Result<(PacketHeader, Bytes), String> {
    decode_stream_header(data)
}

/// Encode a stream packet: prepend [Length: u16 BE][UID: u16 BE][Type: u8] to `payload`.
pub fn encode_stream_packet(uid: u16, packet_type: PacketType, payload: &[u8]) -> Bytes {
    let total = STREAM_HEADER_SIZE + payload.len();
    let mut buf = BytesMut::with_capacity(total);
    buf.put_u16(total as u16);
    buf.put_u16(uid);
    buf.put_u8(packet_type as u8);
    buf.put_slice(payload);
    buf.freeze()
}

/// Encode a datagram packet: [Length: u16 BE][UID: u16 BE][Type: u8][payload] — identical to stream.
pub fn encode_datagram(uid: u16, packet_type: PacketType, payload: &[u8]) -> Bytes {
    encode_stream_packet(uid, packet_type, payload)
}
