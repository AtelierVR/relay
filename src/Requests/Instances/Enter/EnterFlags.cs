namespace Relay.Requests.Instances.Enter;

public enum EnterFlags : byte
{
    None = 0,
    AsBot = 1 << 0,
    UsePseudonyme = 1 << 1,
    UsePassword = 1 << 2,
    HideInList = 1 << 3,
}
