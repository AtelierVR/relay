/// No-op: QUIC streams are reliable by definition — this packet is accepted
/// and silently discarded.
use crate::handlers::packet::Packet;

pub fn handle(_packet: Packet) {}
