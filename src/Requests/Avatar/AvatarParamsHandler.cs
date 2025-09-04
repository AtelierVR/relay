using Relay.Clients;
using Relay.Instances;
using Relay.Packets;
using Relay.Players;
using Relay.Priority;
using Relay.Utils;
using Buffer = Relay.Utils.Buffer;

namespace Relay.Requests.Instances.Avatar;

public class AvatarParamsHandler : Handler
{
    protected override void OnSetup()
    {
        PacketPriorityManager.SetMinimumPriority(RequestType.AvatarParams, EPriority.High);
        PacketDispatcher.RegisterHandler(RequestType.AvatarParams, OnAvatarParams);
    }

    public static void OnAvatarParams(PacketData data)
    {
        var iid = data.Payload.ReadByte();
        var instance = InstanceManager.Get(iid);
        if (instance == null) return;

        var players = instance.GetPlayers();

        var player = players.FirstOrDefault(p => p.ClientId == data.Client.Id);
        if (player == null || !player.IsReady()) return;

        var pid = data.Payload.ReadUShort();
        Player? targetPlayer;

        if (pid != ushort.MaxValue && player.Id != pid)
        {
            targetPlayer = players.FirstOrDefault(p => p.Id == pid);
            if (targetPlayer == null || !targetPlayer.IsReady()) return;
            if (!player.IsAllowed(targetPlayer)) return;
        }
        else targetPlayer = player;

        if (targetPlayer.Avatar == null) return;
        var parameterCount = data.Payload.ReadByte();
        var changedParameters = new Dictionary<int, byte[]>();

        for (int i = 0; i < parameterCount; i++)
        {
            var parameterId = data.Payload.ReadInt();
            var payloadSize = data.Payload.ReadUShort();
            var payload = data.Payload.ReadBytes(payloadSize);
            changedParameters[parameterId] = payload;
            targetPlayer.Avatar.Parameters[parameterId] = payload;
        }

        foreach (var c in players.Where(c => c.ClientId != data.Client.Id))
            SendAvatarParams(c.Client, iid, targetPlayer.Id, changedParameters);
    }

    public static void SendAvatarParams(Client client, byte instanceId, ushort playerId, Dictionary<int, byte[]> changedParameters, ushort uid = ushort.MinValue)
    {
        var buffer = Buffer.New();
        buffer.Write(instanceId);
        buffer.Write(playerId);

        buffer.Write((byte)changedParameters.Count);
        foreach (var (parameterId, payload) in changedParameters)
        {
            buffer.Write(parameterId);
            buffer.Write((ushort)payload.Length);
            buffer.Write(payload);
        }

        Request.SendBuffer(client, buffer, ResponseType.AvatarParams, uid, EPriority.Normal);
    }
}
