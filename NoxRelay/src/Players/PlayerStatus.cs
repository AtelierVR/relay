namespace Relay.Players;

public enum PlayerStatus : byte
{
    None = 0,
    NeedPassword = 1,
    Configuration = 2,
    Ready = 3
}