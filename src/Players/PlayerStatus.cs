namespace Relay.Players;

public enum PlayerStatus : byte
{
    None = 0,
    NeedPassword = 1,
    Preparing = 2,
    Traveling = 3,
    Ready = 4,
}