namespace Relay.Clients;

public enum ClientStatus : byte
{
    Disconnected = 0x00,
    Handshaked = 0x01,
    Authentificating = 0x02,
    Authentificated = 0x03,

    HasHandshake = Handshaked | Authentificating | Authentificated
}