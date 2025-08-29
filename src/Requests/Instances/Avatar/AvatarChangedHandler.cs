using Relay.Clients;
using Relay.Master;
using Relay.Players;
using Relay.Requests;
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
        var player = client.GetInstancePlayer(internalId);
        if (player is not { Status: > PlayerStatus.None }) return;
        Logger.Debug($"{player} sent avatar changed action {action}");

        if (action == AvatarChangedAction.Changing)
        {
            Change(player, uid);
            return;
        }

        if (action == AvatarChangedAction.Ready)
        {
            Ready(player, uid);
            return;
        }

        if (action == AvatarChangedAction.Failed)
        {
            var reason = buffer.Remaining() > 0 ? buffer.ReadString() ?? "Unknown error" : "Unknown error";
            Failed(player, uid, reason);
            return;
        }

        // Unsupported action
        var response = new Buffer();
        response.Write(player.InstanceId);
        response.Write(AvatarChangedResults.Unknown);
        response.Write($"Action {action} is not supported yet");
        Request.SendBuffer(client, response, ResponseType.AvatarChanged, uid);
    }

    public void Change(Player player, ushort uid = 0)
    {
        var response = new Buffer();
        response.Write(player.InstanceId);

        if (player.Status == PlayerStatus.None)
        {
            response.Write(AvatarChangedResults.Unknown);
            response.Write("Player is not ready to change avatar");
            Request.SendBuffer(player.Client, response, ResponseType.AvatarChanged, uid);
            return;
        }

        // For now, we'll respond with a simple success message
        // In a real implementation, you might need to handle avatar data, URLs, etc.
        response.Write(AvatarChangedResults.Ready);
        Request.SendBuffer(player.Client, response, ResponseType.AvatarChanged, uid);

        Logger.Debug($"Avatar change initiated for {player}");
        MasterServer.UpdateImediately();
    }

    public void Ready(Player player, ushort uid = 0)
    {
        var response = new Buffer();
        response.Write(player.InstanceId);

        if (player.Status == PlayerStatus.None)
        {
            response.Write(AvatarChangedResults.Unknown);
            response.Write("Player is not ready");
            Request.SendBuffer(player.Client, response, ResponseType.AvatarChanged, uid);
            return;
        }

        response.Write(AvatarChangedResults.Ready);
        Request.SendBuffer(player.Client, response, ResponseType.AvatarChanged, uid);
        
        Logger.Debug($"Avatar change completed for {player}");
        MasterServer.UpdateImediately();
    }

    public void Failed(Player player, ushort uid = 0, string reason = "Unknown error")
    {
        var response = new Buffer();
        response.Write(player.InstanceId);
        response.Write(AvatarChangedResults.Unknown);
        response.Write(reason);
        Request.SendBuffer(player.Client, response, ResponseType.AvatarChanged, uid);
        
        Logger.Debug($"Avatar change failed for {player}: {reason}");
        MasterServer.UpdateImediately();
    }
}
