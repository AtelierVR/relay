namespace Relay.Requests.Auth;

public enum AuthFlags : byte
{
    None = 0,
    UseIntegrity = 1,
    UseUnAuthenticated = 2
}