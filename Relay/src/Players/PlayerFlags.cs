using System;

namespace Relay.Players;

[Flags]
public enum PlayerFlags : uint
{
    None = 0,
    IsBot = 1,
    InstanceMaster = 2,
    InstanceModerator = 4,
    InstanceOwner = 8,
    GuildModerator = 16,
    MasterModerator = 32,
    WorldOwner = 64,
    WorldModerator = 128,
    AuthUnverified = 256
}