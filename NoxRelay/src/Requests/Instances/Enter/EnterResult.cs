namespace Relay.Requests.Instances.Enter;

public enum EnterResult : byte
{
    Success = 0,
    NotFound = 1,
    Full = 2,
    Blacklisted = 3,
    NotWhitelisted = 4,
    InvalidGame = 5,
    IncorrectPassword = 6,
    Unknown = 7,
    Refused = 8,
    InvalidPseudonyme = 9,
}
