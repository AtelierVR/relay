
namespace Relay.Requests.Handshake;

[Flags]
public enum HandshakeFlags : byte
{
    None = 0,
    IsOffline = 1 << 0
}
