
namespace Relay.Requests.Handshake;

public enum HandshakeFlags : byte
{
    None = 0,
    IsOffline = 1 << 0
}
