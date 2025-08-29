namespace Relay.Requests.Auth;

public enum AuthFlags : byte
{
    None = 0,
    UseIntegrity = 1 << 0,
    UseGuest = 1 << 1
}