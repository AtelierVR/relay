namespace Relay.Requests.Instances.Traveling;

public enum TravelingResults : byte
{
    None = 0,
    UseUrl = 1 << 1,
    UseMaster = 1 << 2,
    Unknown = 1 << 3,
    Ready = 1 << 4,
}

