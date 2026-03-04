/// Unit tests for `proto::buffer` — verifies that `PacketWriter` and `PacketReader`
/// are byte-for-byte compatible with the C# `Relay.Utils.Buffer` implementation.
use bytes::Bytes;
use glam::{Quat, Vec3};

use crate::proto::buffer::{PacketReader, PacketWriter};

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn roundtrip<F, G, T>(write: F, read: G) -> T
where
    F: Fn(&mut PacketWriter),
    G: Fn(&mut PacketReader) -> T,
{
    let mut w = PacketWriter::new();
    write(&mut w);
    let mut r = PacketReader::new(w.finish());
    read(&mut r)
}

// ─── Scalar types ────────────────────────────────────────────────────────────

#[test]
fn u8_roundtrip() {
    assert_eq!(roundtrip(|w| w.write_u8(0xFF), |r| r.read_u8()), 0xFF_u8);
    assert_eq!(roundtrip(|w| w.write_u8(0), |r| r.read_u8()), 0_u8);
}

#[test]
fn u16_roundtrip_big_endian() {
    // 0x1234 big-endian → [0x12, 0x34]
    let mut w = PacketWriter::new();
    w.write_u16(0x1234);
    let bytes = w.finish();
    assert_eq!(&bytes[..], &[0x12, 0x34], "u16 must be big-endian");

    let mut r = PacketReader::new(bytes);
    assert_eq!(r.read_u16(), 0x1234);
}

#[test]
fn i32_roundtrip() {
    for v in [-1_i32, 0, 1, i32::MAX, i32::MIN] {
        assert_eq!(
            roundtrip(|w| w.write_i32(v), |r| r.read_i32()),
            v,
            "i32 {v}"
        );
    }
}

#[test]
fn u32_roundtrip() {
    for v in [0_u32, 1, u32::MAX] {
        assert_eq!(roundtrip(|w| w.write_u32(v), |r| r.read_u32()), v);
    }
}

#[test]
fn i64_roundtrip() {
    for v in [-1_i64, 0, 1, i64::MAX, i64::MIN, 1_700_000_000_000] {
        assert_eq!(
            roundtrip(|w| w.write_i64(v), |r| r.read_i64()),
            v,
            "i64 {v}"
        );
    }
}

#[test]
fn i64_big_endian_bytes() {
    // 1i64 should be 0x00_00_00_00_00_00_00_01
    let mut w = PacketWriter::new();
    w.write_i64(1);
    let bytes = w.finish();
    assert_eq!(
        &bytes[..],
        &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01],
        "i64 must be big-endian (C# compatible)"
    );
}

#[test]
fn f32_roundtrip() {
    for v in [0.0_f32, 1.0, -1.0, f32::MAX, f32::MIN_POSITIVE, 3.14] {
        let result = roundtrip(|w| w.write_f32(v), |r| r.read_f32());
        assert_eq!(result.to_bits(), v.to_bits(), "f32 {v}");
    }
}

#[test]
fn f32_big_endian_bytes() {
    // 1.0_f32 IEEE 754 = 0x3F800000 big-endian = [0x3F, 0x80, 0x00, 0x00]
    let mut w = PacketWriter::new();
    w.write_f32(1.0);
    let bytes = w.finish();
    assert_eq!(&bytes[..], &[0x3F, 0x80, 0x00, 0x00]);
}

#[test]
fn bool_roundtrip() {
    assert_eq!(roundtrip(|w| w.write_bool(true), |r| r.read_bool()), true);
    assert_eq!(roundtrip(|w| w.write_bool(false), |r| r.read_bool()), false);
    // bool is encoded as a single byte (1 for true, 0 for false)
    let mut w = PacketWriter::new();
    w.write_bool(true);
    assert_eq!(w.finish().as_ref(), &[1_u8]);
}

// ─── String ──────────────────────────────────────────────────────────────────

#[test]
fn string_roundtrip() {
    for s in ["", "hello", "Nox Relay 🦀", "test\nwith\nnewlines"] {
        let result = roundtrip(|w| w.write_string(s), |r| r.read_string().unwrap());
        assert_eq!(result, s, "string \"{s}\"");
    }
}

#[test]
fn string_wire_format() {
    // [u16 length][UTF-8 bytes] — c# Buffer.Write(string)
    let mut w = PacketWriter::new();
    w.write_string("hi");
    let bytes = w.finish();
    // length prefix = 2 (big-endian), then 'h'=0x68, 'i'=0x69
    assert_eq!(&bytes[..], &[0x00, 0x02, 0x68, 0x69]);
}

#[test]
fn string_empty_wire() {
    // empty string → [0x00, 0x00] only
    let mut w = PacketWriter::new();
    w.write_string("");
    assert_eq!(&w.finish()[..], &[0x00, 0x00]);
}

// ─── Byte slices ─────────────────────────────────────────────────────────────

#[test]
fn bytes_prefixed_roundtrip() {
    let data = b"payload_data";
    let result = roundtrip(
        |w| w.write_bytes_prefixed(data),
        |r| r.read_bytes_prefixed(),
    );
    assert_eq!(result, data);
}

#[test]
fn bytes_prefixed_empty() {
    let result = roundtrip(|w| w.write_bytes_prefixed(&[]), |r| r.read_bytes_prefixed());
    assert_eq!(result, &[] as &[u8]);
}

#[test]
fn raw_bytes_roundtrip() {
    let data = [1u8, 2, 3, 4, 5];
    let result = roundtrip(|w| w.write_bytes(&data), |r| r.read_bytes(5));
    assert_eq!(result, data);
}

// ─── Vec3 / Quat ─────────────────────────────────────────────────────────────

#[test]
fn vec3_roundtrip() {
    let v = Vec3::new(1.5, -2.0, 0.0);
    let result = roundtrip(|w| w.write_vec3(v), |r| r.read_vec3());
    assert_eq!(result, v);
}

#[test]
fn vec3_zero_roundtrip() {
    let result = roundtrip(|w| w.write_vec3(Vec3::ZERO), |r| r.read_vec3());
    assert_eq!(result, Vec3::ZERO);
}

#[test]
fn quat_roundtrip() {
    let q = Quat::from_xyzw(0.0, 0.0, 0.0, 1.0);
    let result = roundtrip(|w| w.write_quat(q), |r| r.read_quat());
    assert_eq!(result, q);
}

#[test]
fn quat_wire_order() {
    // C# writes X, Y, Z, W — verify the order
    let q = Quat::from_xyzw(1.0, 2.0, 3.0, 4.0);
    let mut w = PacketWriter::new();
    w.write_quat(q);
    let bytes = w.finish();
    // Read them back as four f32s
    let mut r = PacketReader::new(bytes);
    assert_eq!(r.read_f32(), 1.0, "X");
    assert_eq!(r.read_f32(), 2.0, "Y");
    assert_eq!(r.read_f32(), 3.0, "Z");
    assert_eq!(r.read_f32(), 4.0, "W");
}

// ─── Multi-field roundtrip ────────────────────────────────────────────────────

#[test]
fn multi_field_packet() {
    let mut w = PacketWriter::new();
    w.write_u8(0xAB);
    w.write_u16(0x1234);
    w.write_i64(999_999_999_999_i64);
    w.write_string("relay");
    w.write_bool(true);

    let mut r = PacketReader::new(w.finish());
    assert_eq!(r.read_u8(), 0xAB);
    assert_eq!(r.read_u16(), 0x1234);
    assert_eq!(r.read_i64(), 999_999_999_999_i64);
    assert_eq!(r.read_string().unwrap(), "relay");
    assert!(r.read_bool());
}

// ─── Underflow / boundary behaviour ──────────────────────────────────────────

#[test]
fn underflow_returns_zero() {
    // Match C# Buffer: returns 0 / default on underflow, no panic
    let mut r = PacketReader::new(Bytes::new());
    assert_eq!(r.read_u8(), 0);
    assert_eq!(r.read_u16(), 0);
    assert_eq!(r.read_i32(), 0);
    assert_eq!(r.read_i64(), 0);
    assert_eq!(r.read_f32(), 0.0);
}

#[test]
fn string_underflow_returns_none() {
    // A string whose declared length exceeds remaining bytes → None
    let mut w = PacketWriter::new();
    w.write_u16(100); // says 100 bytes follow
                      // but nothing follows
    let mut r = PacketReader::new(w.finish());
    assert!(r.read_string().is_none());
}

#[test]
fn remaining_tracks_reads() {
    let mut w = PacketWriter::new();
    w.write_u8(1);
    w.write_u16(2);
    let bytes = w.finish();
    let mut r = PacketReader::new(bytes);
    assert_eq!(r.remaining(), 3);
    r.read_u8();
    assert_eq!(r.remaining(), 2);
    r.read_u16();
    assert_eq!(r.remaining(), 0);
    assert!(r.is_empty());
}

#[test]
fn skip_advances_cursor() {
    let mut w = PacketWriter::new();
    w.write_u8(0xAA);
    w.write_u8(0xBB);
    w.write_u8(0xCC);
    let mut r = PacketReader::new(w.finish());
    r.skip(1);
    assert_eq!(r.read_u8(), 0xBB);
}
