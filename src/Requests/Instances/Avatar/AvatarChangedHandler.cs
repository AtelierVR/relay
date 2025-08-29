using Relay.Clients;
using Relay.Instances;
using Relay.Master;
using Relay.Players;
using Relay.Utils;
using Buffer = Relay.Utils.Buffer;

namespace Relay.Requests.Instances.Avatar;

public class AvatarChangedHandler : Handler
{
    public override void OnReceive(Buffer buffer, Client client)
    {
        if (client.Status != ClientStatus.Authentificated) return;
        buffer.Goto(0);
        var length = buffer.ReadUShort();
        var uid = buffer.ReadUShort();
        var type = buffer.ReadEnum<RequestType>();
        if (type != RequestType.AvatarChanged) return;
        var internalId = buffer.ReadByte();
        var action = buffer.ReadEnum<AvatarChangedAction>();
        if (action == AvatarChangedAction.Failed)
        {
            var reason = buffer.Remaining() > 0 ? buffer.ReadString() ?? "Unknown error" : "Unknown error";
            Logger.Debug($"{client} sent avatar changed action {action} with reason: {reason}");
            return;
        }

        if (action == AvatarChangedAction.Ready)
        {
            Logger.Debug($"{client} sent avatar changed action {action}");
            return;
        }

        var response = new Buffer();
        response.Write(internalId);

        if (action != AvatarChangedAction.Change)
        {
            Logger.Warning($"{client} sent unsupported avatar changed action {action}");
            response.Write(AvatarChangedResult.Unknown);
            response.Write($"Action {action} is not supported yet");
            Request.SendBuffer(client, response, ResponseType.AvatarChanged, uid);
            return;
        }


        var player = client.GetInstancePlayer(internalId);
        if (player == null || player is { Status: < PlayerStatus.Preparing })
        {
            Logger.Warning($"{client} tried to change avatar while not in an instance or not preparing");
            response.Write(AvatarChangedResult.Failed);
            response.Write("You need to be in an instance and/or preparing to change avatar.");
            Request.SendBuffer(client, response, ResponseType.AvatarChanged, uid);
            return;
        }

        var op_player_id = buffer.ReadUShort();
        var op_player = client.GetInstancePlayer(internalId);

        if (op_player == null || op_player is { Status: < PlayerStatus.Preparing })
        {
            Logger.Warning($"{client} tried to change avatar for a player that is not in the instance or not preparing");
            response.Write(AvatarChangedResult.Failed);
            response.Write("Target player is not in the instance and/or not preparing.");
            Request.SendBuffer(client, response, ResponseType.AvatarChanged, uid);
            return;
        }

        var isSelf = op_player.Client == client;

        if (!isSelf && !player.HasPrivilege)
        {
            // only moderators can transform other players
            Logger.Warning($"{player} tried to change avatar for {op_player}");
            response.Write(AvatarChangedResult.Unknown);
            response.Write($"You don't have permission to change other players' avatars");
            return;
        }

        var id = buffer.ReadUInt();
        var server = buffer.ReadString() ?? string.Empty;
        var version = buffer.ReadUShort();

        op_player.avatar = new Avatars.Avatar()
        {
            Id = id,
            Server = server,
            Version = version
        };

        Logger.Debug($"{client} sent avatar changed to {op_player.avatar} for {op_player.ToString()}");
        // send to all players in the instance except the senderF
        foreach (var other in op_player.Instance.Players)
            if (other.Client != client)
                SendAvatarChanged(other.Client, internalId, op_player_id, op_player.avatar);

        response.Write(AvatarChangedResult.Success);
        Request.SendBuffer(client, response, ResponseType.AvatarChanged, uid);
    }

    public static void SendAvatarChanged(Client client, byte instanceId, ushort playerId, Avatars.Avatar avatar, ushort uid = ushort.MinValue)
    {
        var buffer = new Buffer();
        buffer.Write(instanceId);
        buffer.Write(AvatarChangedResult.Changing);
        buffer.Write(playerId);
        buffer.Write(avatar.Id);
        buffer.Write(avatar.Server);
        buffer.Write(avatar.Version);
        Request.SendBuffer(client, buffer, ResponseType.AvatarChanged, uid);
    }
}
