namespace Relay.Requests.Auth;

public enum AuthResult : byte
{
    Success = 0,
    InvalidToken = 1,
    MasterError = 2,
    Blacklisted = 3,
    Unknown = 4
}