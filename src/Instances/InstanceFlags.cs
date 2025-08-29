namespace Relay.Instances;

[Flags]
public enum InstanceFlags: uint
{
    None = 0,
    IsPublic = 1 << 0,
    UsePassword = 1 << 1,
    UseWhitelist = 1 << 2,
    AuthorizeBot = 1 << 3,
    AllowOverload = 1 << 4
}