using Relay.Clients;
using Relay.Packets;
using Relay.Priority;
using Relay.Requests.Instances.Quit;
using Relay.Utils;
using Buffer = Relay.Utils.Buffer;

namespace Relay.Requests.Disconnect;

public class DisconnectHandler : Handler
{
    protected override void OnSetup()
    {
        PacketPriorityManager.SetMinimumPriority(RequestType.Disconnect, EPriority.Critical);
        PacketDispatcher.RegisterHandler(RequestType.Disconnect, OnDisconnect);
    }

    public static void OnDisconnect(PacketData data)
    {
        // Reject if not handshaked
        if (!data.Client.IsHandshake) return;

        if (data.Payload.Remaining() > 0)
        {
            var reason = data.Payload.ReadString() ?? string.Empty;
            Logger.Warning($"{data.Client} disconnected: {reason}");
        }
        SendEvent(data.Client);
    }

    public static void SendEvent(Client client, string? reason = null, QuitType type = QuitType.Normal)
    {
        var buffer = Buffer.New();
        if (reason != null)
            buffer.Write(reason);
        Request.SendBuffer(client, buffer, ResponseType.Disconnect, priority: EPriority.Critical);
    }
}