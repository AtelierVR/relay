use bitflags::bitflags;

bitflags! {
    /// Flags describing the role and properties of a player in an instance.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct PlayerFlags: u32 {
        const NONE               = 0;
        const IS_BOT             = 1 << 0;
        const INSTANCE_MASTER    = 1 << 1;
        const INSTANCE_MODERATOR = 1 << 2;
        const INSTANCE_OWNER     = 1 << 3;
        const GUILD_MODERATOR    = 1 << 4;
        const MASTER_MODERATOR   = 1 << 5;
        const WORLD_OWNER        = 1 << 6;
        const WORLD_MODERATOR    = 1 << 7;
        const AUTH_UNVERIFIED    = 1 << 8;
        const HIDE_IN_LIST       = 1 << 9;
        const HAS_CUSTOM_DISPLAY = 1 << 10;
    }
}

impl PlayerFlags {
    pub const HAS_PRIVILEGE: PlayerFlags = PlayerFlags::from_bits_truncate(
        Self::INSTANCE_MASTER.bits()
            | Self::INSTANCE_MODERATOR.bits()
            | Self::INSTANCE_OWNER.bits()
            | Self::GUILD_MODERATOR.bits()
            | Self::MASTER_MODERATOR.bits()
            | Self::WORLD_OWNER.bits()
            | Self::WORLD_MODERATOR.bits(),
    );

    pub const HAS_HIGH_PRIVILEGE: PlayerFlags = PlayerFlags::from_bits_truncate(
        Self::INSTANCE_OWNER.bits() | Self::MASTER_MODERATOR.bits(),
    );
}
