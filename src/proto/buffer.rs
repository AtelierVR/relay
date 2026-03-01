/// Big-endian packet reader/writer matching C# `Relay.Utils.Buffer` byte-for-byte.
///
/// The wire format is identical to the C# relay:
/// - integers: big-endian
/// - strings: `[u16 length][UTF-8 bytes]`
/// - bytes slices: `[u16 length][bytes]`
/// - Vec3: three consecutive f32 (X, Y, Z)
/// - Quat: four consecutive f32 (X, Y, Z, W)
use bytes::{BufMut, Bytes, BytesMut};
use glam::{Quat, Vec3};

// ─── Writer ─────────────────────────────────────────────────────────────────

/// Builds a binary packet in big-endian format.
#[derive(Debug, Default)]
pub struct PacketWriter {
    buf: BytesMut,
}

impl PacketWriter {
    pub fn new() -> Self {
        Self { buf: BytesMut::new() }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self { buf: BytesMut::with_capacity(cap) }
    }

    /// Consume the writer and return the finished bytes.
    pub fn finish(self) -> Bytes {
        self.buf.freeze()
    }

    /// Return a reference to the accumulated bytes without consuming the writer.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf
    }

    pub fn write_u8(&mut self, v: u8) {
        self.buf.put_u8(v);
    }

    pub fn write_i16(&mut self, v: i16) {
        self.buf.put_i16(v);
    }

    pub fn write_u16(&mut self, v: u16) {
        self.buf.put_u16(v);
    }

    pub fn write_i32(&mut self, v: i32) {
        self.buf.put_i32(v);
    }

    pub fn write_u32(&mut self, v: u32) {
        self.buf.put_u32(v);
    }

    pub fn write_i64(&mut self, v: i64) {
        self.buf.put_i64(v);
    }

    pub fn write_u64(&mut self, v: u64) {
        self.buf.put_u64(v);
    }

    /// Big-endian IEEE-754 single.
    pub fn write_f32(&mut self, v: f32) {
        self.buf.put_f32(v);
    }

    /// Big-endian IEEE-754 double.
    pub fn write_f64(&mut self, v: f64) {
        self.buf.put_f64(v);
    }

    /// `[u16 length][UTF-8 bytes]`  — mirrors `Buffer.Write(string)`.
    pub fn write_string(&mut self, s: &str) {
        let bytes = s.as_bytes();
        self.buf.put_u16(bytes.len() as u16);
        self.buf.put_slice(bytes);
    }

    /// `[u16 length][bytes]`  — mirrors `Buffer.Write(byte[])` when combined with length prefix.
    pub fn write_bytes_prefixed(&mut self, data: &[u8]) {
        self.buf.put_u16(data.len() as u16);
        self.buf.put_slice(data);
    }

    /// Raw byte slice with no length prefix.
    pub fn write_bytes(&mut self, data: &[u8]) {
        self.buf.put_slice(data);
    }

    /// Three big-endian f32: X, Y, Z.
    pub fn write_vec3(&mut self, v: Vec3) {
        self.write_f32(v.x);
        self.write_f32(v.y);
        self.write_f32(v.z);
    }

    /// Four big-endian f32: X, Y, Z, W.
    pub fn write_quat(&mut self, q: Quat) {
        self.write_f32(q.x);
        self.write_f32(q.y);
        self.write_f32(q.z);
        self.write_f32(q.w);
    }

    /// Write a Unix-time ms timestamp (i64).
    pub fn write_timestamp(&mut self, ms: i64) {
        self.write_i64(ms);
    }

    /// Write a `bool` as a single byte (0 or 1).
    pub fn write_bool(&mut self, v: bool) {
        self.write_u8(if v { 1 } else { 0 });
    }

    /// Append another writer's payload.
    pub fn write_writer(&mut self, other: &PacketWriter) {
        self.buf.put_slice(&other.buf);
    }
}

// ─── Reader ─────────────────────────────────────────────────────────────────

/// Reads binary packet data in big-endian format.
#[derive(Debug, Clone)]
pub struct PacketReader {
    data: Bytes,
    offset: usize,
}

impl PacketReader {
    pub fn new(data: Bytes) -> Self {
        Self { data, offset: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.offset)
    }

    pub fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    /// Move the cursor to an absolute position.
    pub fn seek(&mut self, pos: usize) {
        self.offset = pos.min(self.data.len());
    }

    /// Skip `n` bytes forward.
    pub fn skip(&mut self, n: usize) {
        self.offset = self.data.len().min(self.offset + n);
    }

    // ── Infallible reads (return 0 / default on underflow, matching C# Buffer) ──

    pub fn read_u8(&mut self) -> u8 {
        if self.remaining() < 1 { return 0; }
        let v = self.data[self.offset];
        self.offset += 1;
        v
    }

    pub fn read_i16(&mut self) -> i16 {
        if self.remaining() < 2 { return 0; }
        let v = i16::from_be_bytes([self.data[self.offset], self.data[self.offset + 1]]);
        self.offset += 2;
        v
    }

    pub fn read_u16(&mut self) -> u16 {
        self.read_i16() as u16
    }

    pub fn read_i32(&mut self) -> i32 {
        if self.remaining() < 4 { return 0; }
        let v = i32::from_be_bytes(self.data[self.offset..self.offset + 4].try_into().unwrap());
        self.offset += 4;
        v
    }

    pub fn read_u32(&mut self) -> u32 {
        self.read_i32() as u32
    }

    pub fn read_i64(&mut self) -> i64 {
        if self.remaining() < 8 { return 0; }
        let v = i64::from_be_bytes(self.data[self.offset..self.offset + 8].try_into().unwrap());
        self.offset += 8;
        v
    }

    pub fn read_u64(&mut self) -> u64 {
        self.read_i64() as u64
    }

    /// Big-endian IEEE-754 single.
    pub fn read_f32(&mut self) -> f32 {
        if self.remaining() < 4 { return 0.0; }
        let v = f32::from_be_bytes(self.data[self.offset..self.offset + 4].try_into().unwrap());
        self.offset += 4;
        v
    }

    /// Big-endian IEEE-754 double.
    pub fn read_f64(&mut self) -> f64 {
        if self.remaining() < 8 { return 0.0; }
        let v = f64::from_be_bytes(self.data[self.offset..self.offset + 8].try_into().unwrap());
        self.offset += 8;
        v
    }

    /// `[u16 length][UTF-8 bytes]`  — mirrors `Buffer.ReadString()`.
    pub fn read_string(&mut self) -> Option<String> {
        let len = self.read_u16() as usize;
        if self.remaining() < len { return None; }
        let bytes = &self.data[self.offset..self.offset + len];
        let s = String::from_utf8(bytes.to_vec()).ok()?;
        self.offset += len;
        Some(s)
    }

    /// `[u16 length][bytes]`  — mirrors `Buffer.ReadBytes(ushort)`.
    pub fn read_bytes_prefixed(&mut self) -> Vec<u8> {
        let len = self.read_u16() as usize;
        self.read_bytes(len)
    }

    /// Read exactly `len` raw bytes.
    pub fn read_bytes(&mut self, len: usize) -> Vec<u8> {
        if len == 0 { return vec![]; }
        if self.remaining() < len { return vec![]; }
        let v = self.data[self.offset..self.offset + len].to_vec();
        self.offset += len;
        v
    }

    /// Three big-endian f32 → `Vec3`.
    pub fn read_vec3(&mut self) -> Vec3 {
        if self.remaining() < 12 { return Vec3::ZERO; }
        let x = self.read_f32();
        let y = self.read_f32();
        let z = self.read_f32();
        Vec3::new(x, y, z)
    }

    /// Four big-endian f32 → `Quat` (X, Y, Z, W).
    pub fn read_quat(&mut self) -> Quat {
        if self.remaining() < 16 { return Quat::IDENTITY; }
        let x = self.read_f32();
        let y = self.read_f32();
        let z = self.read_f32();
        let w = self.read_f32();
        Quat::from_xyzw(x, y, z, w)
    }

    /// Read a `bool` as a single byte.
    pub fn read_bool(&mut self) -> bool {
        self.read_u8() != 0
    }

    /// Read a Unix-ms timestamp as i64.
    pub fn read_timestamp(&mut self) -> i64 {
        self.read_i64()
    }

    /// Consume remaining bytes as a sub-`Bytes` slice.
    pub fn read_remaining(&mut self) -> Bytes {
        let slice = self.data.slice(self.offset..);
        self.offset = self.data.len();
        slice
    }
}
