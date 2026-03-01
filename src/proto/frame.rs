/// Length-prefixed framing for QUIC streams and raw parsing for QUIC datagrams.
///
/// Stream wire format (same as C# TCP format):
///   `[Length: u16 BE][UID: u16 BE][Type: u8][payload...]`
///   where `Length` is the **total** packet size including the 2-byte length
///   field itself (i.e. `Length = 5 + payload_len`).
///
/// Datagram format: no length prefix — the QUIC datagram boundary is used.
use anyhow::{anyhow, Result};
use bytes::{BufMut, Bytes, BytesMut};
use quinn::{RecvStream, SendStream};

use super::header::{decode_datagram_header, PacketHeader};

/// Read one packet from a QUIC `RecvStream`.
///
/// Wire: `[total: u16 BE][uid: u16 BE][type: u8][payload...]`
/// `total` is the **inclusive** total length (includes the 2-byte `total` field
/// itself). Minimum valid value is 5 (header-only, empty payload).
///
/// Returns the full packet bytes `[total][uid][type][payload]` so that
/// `decode_stream_header` can parse them normally.
pub async fn read_framed(stream: &mut RecvStream) -> Result<Bytes> {
    // Read the 2-byte inclusive length prefix.
    let mut len_buf = [0u8; 2];
    stream
        .read_exact(&mut len_buf)
        .await
        .map_err(|e| anyhow!("stream read length: {e}"))?;

    let total = u16::from_be_bytes(len_buf) as usize;
    if total < 2 {
        return Ok(Bytes::new());
    }

    // The C# length field is inclusive — the payload that follows is `total - 2`
    // bytes (the remaining header fields + packet body).
    let rest_len = total.saturating_sub(2);
    let mut rest = vec![0u8; rest_len];
    stream
        .read_exact(&mut rest)
        .await
        .map_err(|e| anyhow!("stream read body ({rest_len} bytes): {e}"))?;

    // Reassemble the full packet including the length prefix so that
    // `decode_stream_header` receives `[total][uid][type][payload]`.
    let mut buf = BytesMut::with_capacity(total);
    buf.put_u16(total as u16);
    buf.put_slice(&rest);
    Ok(buf.freeze())
}

/// Write one packet to a QUIC `SendStream`.
///
/// `data` must already be a fully-encoded packet produced by
/// `encode_stream_packet`, which includes the `[total: u16][uid: u16][type: u8]`
/// header. We write it verbatim — no extra length prefix is added.
pub async fn write_framed(stream: &mut SendStream, data: &Bytes) -> Result<()> {
    stream
        .write_all(data)
        .await
        .map_err(|e| anyhow!("stream write: {e}"))?;
    Ok(())
}

/// Parse a QUIC datagram buffer into `(PacketHeader, payload)`.
///
/// Datagrams carry no length prefix — the QUIC layer provides the boundary.
pub fn parse_datagram(buf: Bytes) -> Result<(PacketHeader, Bytes)> {
    decode_datagram_header(buf).map_err(|e| anyhow!("datagram header: {e}"))
}
