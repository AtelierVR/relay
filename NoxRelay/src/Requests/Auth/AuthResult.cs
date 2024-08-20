namespace Relay.Requests.Auth;

public enum AuthResult : byte
{
    Success = 0,
    InvalidToken = 1,
    CannotContactMasterServer = 2,
    Blacklisted = 3,
    NotReady = 4,
    RequireAuth = 5,
    Unknown = 5
}