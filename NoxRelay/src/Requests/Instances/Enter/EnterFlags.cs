namespace Relay.Requests.Instances.Enter;

public enum EnterFlags: byte
{
    None = 0,
    AsBot = 1,
    UsePseudonyme = 2,
    UsePassword = 4,
    HideInList = 8
}