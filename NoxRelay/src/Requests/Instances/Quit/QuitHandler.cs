using Relay.Clients;
using Relay.Master;
using Relay.Players;
using Relay.Utils;
using Buffer = Relay.Utils.Buffer;

namespace Relay.Requests.Instances.Quit;

public class QuitHandler : Handler
{
    public override void OnReceive(Buffer buffer, Client client)
    {
        if (client.Status != ClientStatus.Authentificated) return;
        buffer.Goto(0);
        var length = buffer.ReadUShort();
        var uid = buffer.ReadUShort();
        var type = buffer.ReadEnum<RequestType>();
        if (type != RequestType.Quit) return;
        Logger.Debug($"{client} sent quit");
        var instanceId = buffer.ReadUShort();
        var player = PlayerManager.GetFromClientInstance(client.Id, instanceId);
        if (player is not { Status: > PlayerStatus.None }) return;
        var action = buffer.ReadEnum<QuitType>();
        LeavePlayer(player, action, buffer.ReadString(), player, uid);
    }

    public void LeavePlayer(Player player, QuitType type, string? reason = null, Player? by = null, ushort uid = 0)
    {
        if (player.Status == PlayerStatus.None) return;
        if (type == QuitType.ModerationKick && (by?.Modered is null || !by.Modered.IsModerator)) return;
        if (type == QuitType.VoteKick && by != null) return;
        if (type == QuitType.Timeout && by != player) return;
        if (type == QuitType.Normal && by != player) return;
        player.Status = PlayerStatus.None;
        Logger.Log($"{player} left the instance");

        // other players leave event
        Buffer response;
        foreach (var other in player.Instance.Players.Where(other => other != player))
        {
            response = new Buffer();
            response.Write(player.InstanceId);
            response.Write(QuitType.Normal);
            response.Write(player.Id);
            Request.SendBuffer(other.Client, response, ResponseType.Quit);
        }

        // quit event
        response = new Buffer();
        response.Write(player.InstanceId);
        response.Write(type);
        if (reason is not null)
            response.Write(reason);
        Request.SendBuffer(player.Client, response, ResponseType.Quit, uid);

        // leave event
        response = new Buffer();
        response.Write(player.InstanceId);
        response.Write(type);
        response.Write(player.Id);
        foreach (var other in player.Instance.Players.Where(other => other != player))
            Request.SendBuffer(other.Client, response, ResponseType.Leave);

        // remove player
        PlayerManager.Remove(player);

        MasterServer.UpdateImediately();
    }
}