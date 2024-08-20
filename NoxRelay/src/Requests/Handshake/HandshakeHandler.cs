using Relay.Clients;
using Relay.Requests.Disconnect;
using Relay.Utils;
using Buffer = Relay.Utils.Buffer;

namespace Relay.Requests.Handshake;

public class HandshakeHandler : Handler
{
    public override void OnReceive(Buffer buffer, Client client)
    {
        buffer.Goto(0);
        var length = buffer.ReadUShort();
        var uid = buffer.ReadUShort();
        var type = buffer.ReadEnum<RequestType>();
        if (type != RequestType.Handshake) return;
        Logger.Debug($"{client} sent handshake");

        var protocol = buffer.ReadUShort();
        var engine = buffer.ReadEnum<Engine>();
        var platform = buffer.ReadEnum<Platfrom>();
        
        if (protocol != Constants.ProtocolVersion)
        {
            Get<DisconnectHandler>().SendEvent(client,
                string.Format(Messages.IncompatibleProtocol, protocol, Constants.ProtocolVersion)
            );
            return;
        }

        client.Engine = engine;
        client.Platform = platform;
        
        if (client.Status == ClientStatus.Disconnected)
            client.Status = ClientStatus.Handshaked;

        var response = new Buffer();
        response.Write(Constants.ProtocolVersion);
        response.Write(client.Id);
        response.Write(client.Status);
        response.Write(client.Remote.Address.GetAddressBytes());
        response.Write(client.Remote.Port);
        Request.SendBuffer(client, response, ResponseType.Handshake, uid);
    }
}