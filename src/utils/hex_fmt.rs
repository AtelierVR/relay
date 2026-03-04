/// Helper utilities for hex formatting in logs.
/// Formats a byte slice as a truncated hex string for logging.
/// Shows first `max_bytes` bytes followed by "..." if truncated.
pub fn to_hex_str(data: &[u8], max_bytes: usize) -> String {
    if data.len() <= max_bytes {
        hex::encode(data)
    } else {
        format!(
            "{}... ({} bytes total)",
            hex::encode(&data[..max_bytes]),
            data.len()
        )
    }
}

/// Formats a byte slice as a compact hex string with prefix.
/// Example: "16 bytes: a1b2c3d4..."
pub fn fmt_bytes(data: &[u8], max_display: usize) -> String {
    format!("{} bytes: {}", data.len(), to_hex_str(data, max_display))
}
