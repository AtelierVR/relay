using System;

namespace Relay.Instances;

[Flags]
public enum InstanceFlags: uint
{
    None = 0,
    IsPublic = 1,
    IsDefault = 2,
    UsePassword = 4,
    UseWhitelist = 8,
    AuthorizeBot = 16,
    UseMods = 32,
    EnableCrossInventory = 64,
    EnableCustomAvatar = 128,
    AllowWorldModification = 256,
    AllowFly = 512,
    AllowProps = 1024,
    VanishBlocked = 2048,
    GhostBlocked = 4096,
    GroupModeration = 8192
}