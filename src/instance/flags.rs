use bitflags::bitflags;

bitflags! {
    /// Flags that describe the properties and access rules of an instance.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct InstanceFlags: u32 {
        const NONE           = 0;
        const IS_PUBLIC      = 1 << 0;
        const USE_PASSWORD   = 1 << 1;
        const USE_WHITELIST  = 1 << 2;
        const AUTHORIZE_BOT  = 1 << 3;
        const ALLOW_OVERLOAD = 1 << 4;
    }
}
