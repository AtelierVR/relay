using System;
using Relay.Clients;
using Relay.Utils;
using Buffer = Relay.Utils.Buffer;

namespace Relay.Requests.Latency;

public class LantencyHandler : Handler
{
    public override void OnReceive(Buffer buffer, Client client)
    {
        if (client.Status == ClientStatus.Disconnected) return;
        buffer.Goto(0);
        var length = buffer.ReadUShort();
        var uid = buffer.ReadUShort();
        var type = buffer.ReadEnum<RequestType>();
        if (type != RequestType.Latency) return;
        Logger.Debug($"{client} sent latency");

        var initial = buffer.ReadDateTime();
        var response = new Buffer();
        response.Write(initial);
        response.Write(DateTime.UtcNow);
        Request.SendBuffer(client, response, ResponseType.Latency, uid);
    }
}