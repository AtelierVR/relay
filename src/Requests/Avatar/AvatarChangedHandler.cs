using Relay.Clients;
using Relay.Instances;
using Relay.Packets;
using Relay.Players;
using Relay.Priority;
using Relay.Utils;
using Buffer = Relay.Utils.Buffer;

namespace Relay.Requests.Instances.Avatar;

public class AvatarChangedHandler : Handler
{
    protected override void OnSetup()
    {
        PacketPriorityManager.SetMinimumPriority(RequestType.AvatarChanged, EPriority.High);
        PacketDispatcher.RegisterHandler(RequestType.AvatarChanged, OnAuthentification);
    }

    public static void OnAuthentification(PacketData data)
    {
        var iid = data.Payload.ReadByte();
        var instance = InstanceManager.Get(iid);
        if (instance == null)
        {
            SendResponse(data.Client, iid, data.Uid, AvatarChangedResult.Failed, "Invalid instance");
            return;
        }

        var players = instance.GetPlayers();

        var player = players.FirstOrDefault(p => p.ClientId == data.Client.Id);
        if (player == null || !player.IsReady())
        {
            SendResponse(data.Client, iid, data.Uid, AvatarChangedResult.Failed, "You are not a valid player");
            return;
        }

        var pid = data.Payload.ReadUShort();
        Player? opPlayer;

        if (pid != ushort.MaxValue && player.Id != pid)
        {
            opPlayer = players.FirstOrDefault(p => p.Id == pid);
            if (opPlayer == null || !opPlayer.IsReady())
            {
                SendResponse(data.Client, iid, data.Uid, AvatarChangedResult.Failed, "Player not found");
                return;
            }

            if (!player.IsAllowed(opPlayer))
            {
                SendResponse(data.Client, iid, data.Uid, AvatarChangedResult.Failed, "You are not allowed to change this player's avatar");
                return;
            }
        }
        else opPlayer = player;

        opPlayer.Avatar = new Avatars.Avatar()
        {
            Id = data.Payload.ReadUInt(),
            Server = data.Payload.ReadString() ?? string.Empty,
            Version = data.Payload.ReadUShort(),
            Parameters = []
        };

        Logger.Debug($"{opPlayer} changed avatar to {opPlayer.Avatar}");

        SendResponse(data.Client, iid, data.Uid, AvatarChangedResult.Success);
        foreach (var c in players.Where(c => c.ClientId != data.Client.Id))
            SendAvatarChanged(c.Client, iid, opPlayer.Id, opPlayer.Avatar);
    }

    public static void SendResponse(Client client, byte internalId, ushort uid, AvatarChangedResult result, string? reason = null)
    {
        var response = Buffer.New();
        response.Write(internalId);
        response.Write(result);
        if (reason != null)
            response.Write(reason);
        Request.SendBuffer(client, response, ResponseType.AvatarChanged, uid, EPriority.High);
    }

    public static void SendAvatarChanged(Client client, byte instanceId, ushort playerId, Avatars.Avatar avatar, ushort uid = ushort.MinValue)
    {
        var buffer = Buffer.New();
        buffer.Write(instanceId);
        buffer.Write(AvatarChangedResult.Changing);
        buffer.Write(playerId);
        buffer.Write(avatar.Id);
        buffer.Write(avatar.Server);
        buffer.Write(avatar.Version);
        Request.SendBuffer(client, buffer, ResponseType.AvatarChanged, uid, EPriority.Normal);
    }
}
