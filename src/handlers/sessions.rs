/// Sessions handler — returns paginated instance list.
///
/// Request:  [page: u8]
/// Response: [instances_on_page: u8][...instance_records][page: u8][total_pages: u8]
///
/// Each instance record: [flags: u32][iid: u8][master_id: u32][player_count: u16][capacity: u16]
use bytes::Bytes;
use tracing::debug;

use crate::{
    constants::MAX_PACKET_SIZE,
    handlers::context::AppState,
    proto::{
        buffer::{PacketReader, PacketWriter},
        header::encode_stream_packet,
        packet_type::PacketType,
    },
    utils::hex_fmt,
};

// One instance record size (fixed): flags(4) + iid(1) + master_id(4) + count(2) + capacity(2) = 13 bytes
const RECORD_SIZE: usize = 13;

pub async fn handle(state: &AppState, client_id: u16, uid: u16, payload: Bytes) -> Bytes {
    debug!(
        "[Sessions] client {}: raw payload: {}",
        client_id,
        hex_fmt::fmt_bytes(payload.as_ref(), 32)
    );
    // Must have at least handshaked.
    let is_ok = state
        .clients
        .get(client_id)
        .map(|a| a.read().is_handshaked())
        .unwrap_or(false);
    if !is_ok {
        return Bytes::new();
    }

    let mut r = PacketReader::new(payload);
    let page = r.read_u8();
    debug!("[Sessions] client {client_id}: requested page {page}");

    // Collect all instance records.
    let records: Vec<(u32, u8, u32, u16, u16)> = {
        let mut list = Vec::new();
        state.instances.for_each(|_, inst_arc| {
            let inst = inst_arc.read();
            list.push((
                inst.flags.bits(),
                inst.internal_id,
                inst.master_id,
                inst.player_count() as u16,
                inst.capacity,
            ));
        });
        list
    };

    // Paginate: bytes_per_page = MAX_PACKET_SIZE - header(5) - count(1) - page(1) - total(1) = MAX_PACKET_SIZE - 8
    let bytes_per_page = MAX_PACKET_SIZE.saturating_sub(8);
    let per_page = (bytes_per_page / RECORD_SIZE).max(1);
    type SessionRecordChunk = [(u32, u8, u32, u16, u16)];
    let pages: Vec<&SessionRecordChunk> = records.chunks(per_page).collect();
    let total_pages = pages.len() as u8;

    let mut w = PacketWriter::new();
    if page as usize >= pages.len() {
        w.write_u8(0);
    } else {
        let slice = pages[page as usize];
        w.write_u8(slice.len() as u8);
        for (flags, iid, master_id, count, cap) in slice {
            w.write_u32(*flags);
            w.write_u8(*iid);
            w.write_u32(*master_id);
            w.write_u16(*count);
            w.write_u16(*cap);
        }
    }
    w.write_u8(page);
    w.write_u8(total_pages);

    encode_stream_packet(uid, PacketType::Sessions, w.finish().as_ref())
}
