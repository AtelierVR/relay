/// Current lifecycle status of a player in an instance.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PlayerStatus {
    #[default]
    None = 0,
    NeedPassword = 1,
    Preparing = 2,
    Traveling = 3,
    Ready = 4,
}

impl TryFrom<u8> for PlayerStatus {
    type Error = u8;
    fn try_from(v: u8) -> Result<Self, u8> {
        match v {
            0 => Ok(Self::None),
            1 => Ok(Self::NeedPassword),
            2 => Ok(Self::Preparing),
            3 => Ok(Self::Traveling),
            4 => Ok(Self::Ready),
            other => Err(other),
        }
    }
}
