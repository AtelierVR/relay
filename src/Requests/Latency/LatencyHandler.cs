using Relay.Packets;
using Relay.Priority;
using Relay.Utils;
using Buffer = Relay.Utils.Buffer;

namespace Relay.Requests.Latency;

public class LatencyHandler : Handler
{
    protected override void OnSetup()
    {
        PacketPriorityManager.SetMinimumPriority(RequestType.Latency, EPriority.High);
        PacketDispatcher.RegisterHandler(RequestType.Latency, OnLatency);
    }

    private static void OnLatency(PacketData data)
    {
        // Reject if not handshake
        if (!data.Client.IsHandshake) return;

        var response = Buffer.New();
        response.Write(data.Payload.ReadDateTime());
        response.Write(DateTime.UtcNow);
        Request.SendBuffer(data.Client, response, ResponseType.Latency, data.Uid, EPriority.High);
    }
}