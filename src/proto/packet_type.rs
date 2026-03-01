/// All packet types exchanged between clients and the relay.
///
/// Values 0x00–0x0F are bidirectional (client ↔ server).
/// Values 0x10–0x11 are server-initiated broadcasts.
/// Values 0x12–0x15 are bidirectional.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PacketType {
    Disconnect          = 0x00,
    Handshake           = 0x01,
    Segmentation        = 0x02,
    Reliable            = 0x03,
    Latency             = 0x04,
    Authentication      = 0x05,
    Enter               = 0x06,
    Quit                = 0x07,
    Custom              = 0x08,
    PasswordRequirement = 0x09,
    Traveling           = 0x0A,
    Transform           = 0x0B,
    Teleport            = 0x0C,
    AvatarChanged       = 0x0D,
    ServerConfig        = 0x0E,
    Properties          = 0x0F,
    Join                = 0x10, // server → client broadcast
    Leave               = 0x11, // server → client broadcast
    PlayerUpdate        = 0x12,
    Sessions            = 0x13,
    Voice               = 0x14,
    Event               = 0x15,
}

impl TryFrom<u8> for PacketType {
    type Error = u8;

    fn try_from(v: u8) -> Result<Self, u8> {
        match v {
            0x00 => Ok(Self::Disconnect),
            0x01 => Ok(Self::Handshake),
            0x02 => Ok(Self::Segmentation),
            0x03 => Ok(Self::Reliable),
            0x04 => Ok(Self::Latency),
            0x05 => Ok(Self::Authentication),
            0x06 => Ok(Self::Enter),
            0x07 => Ok(Self::Quit),
            0x08 => Ok(Self::Custom),
            0x09 => Ok(Self::PasswordRequirement),
            0x0A => Ok(Self::Traveling),
            0x0B => Ok(Self::Transform),
            0x0C => Ok(Self::Teleport),
            0x0D => Ok(Self::AvatarChanged),
            0x0E => Ok(Self::ServerConfig),
            0x0F => Ok(Self::Properties),
            0x10 => Ok(Self::Join),
            0x11 => Ok(Self::Leave),
            0x12 => Ok(Self::PlayerUpdate),
            0x13 => Ok(Self::Sessions),
            0x14 => Ok(Self::Voice),
            0x15 => Ok(Self::Event),
            other => Err(other),
        }
    }
}

/// Which QUIC transport channel to use for a given packet type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    /// Bidirectional QUIC stream — reliable, ordered, arbitrary size.
    BiStream,
    /// QUIC datagram — unreliable, unordered, MTU-limited.
    Datagram,
}

impl PacketType {
    /// Returns the preferred QUIC channel for this packet type.
    pub fn channel(self) -> Channel {
        match self {
            // High-frequency, loss-tolerant data uses datagrams.
            PacketType::Transform | PacketType::Voice => Channel::Datagram,
            _ => Channel::BiStream,
        }
    }
}
