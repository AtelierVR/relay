using Relay.Clients;
using Relay.Utils;
using Buffer = Relay.Utils.Buffer;

namespace Relay.Requests.Disconnect;

public class DisconnectHandler : Handler
{
    public override void OnReceive(Buffer buffer, Client client)
    {
        if (client.Status == ClientStatus.Disconnected) return;
        buffer.Goto(0);
        var length = buffer.ReadUShort();
        var uid = buffer.ReadUShort();
        var type = buffer.ReadEnum<RequestType>();
        if (type != RequestType.Disconnect) return;
        var reason = buffer.ReadString();
        SendEvent(client, "Good Bye!");
    }

    public void SendEvent(Client client, string reason)
    {
        if (client.Status == ClientStatus.Disconnected) return;
        var buffer = new Buffer();
        if (!string.IsNullOrEmpty(reason))
            buffer.Write(reason);
        Request.SendBuffer(client, buffer, ResponseType.Disconnect);
        client.Status = ClientStatus.Disconnected;
        client.OnDisconnect(reason);
        ClientManager.Remove(client);
    }
}