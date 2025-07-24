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
        var internalId = buffer.ReadByte();
        var player = PlayerManager.GetFromClientInstance(client.Id, internalId);
        if (player is not { Status: > PlayerStatus.None }) return;
        var action = buffer.ReadEnum<QuitType>();
        string? reason = null;
        if (buffer.Remaining() > 2)
            reason = buffer.ReadString();
        LeavePlayer(player, action, reason, player, uid);
    }

    public void LeavePlayer(Player player, QuitType type, string? reason = null, Player? by = null, ushort uid = 0)
    {
        if (player.Status == PlayerStatus.None) return;
        if (type == QuitType.ModerationKick && (by?.Modered is null || !by.Modered.IsModerator)) return;
        if (type == QuitType.VoteKick && by != null) return;
        if (type == QuitType.Timeout && by != player) return;
        if (type == QuitType.Normal && by != player) return;
        Logger.Log($"{player} left the instance");

        Buffer response;

        if (player.Status == PlayerStatus.Ready)
            foreach (var other in player.Instance.Players.Where(other => other.Id != player.Id))
            {
                if (other is not { Status: PlayerStatus.Ready }) continue;
                var allMessage = other.IsModerator
                    || by is not null && by.Id == other.Id;

                // broadcast leave event to other players of the player
                response = new Buffer();
                response.Write(player.InstanceId);
                response.Write(allMessage ? QuitType.Normal : type);
                response.Write(player.Id);
                if (allMessage && (type & QuitType.ModerationAction) != 0)
                {
                    response.Write(by?.Id ?? ushort.MaxValue);
                    if (!string.IsNullOrEmpty(reason))
                        response.Write(reason);
                }
                Request.SendBuffer(other.Client, response, ResponseType.Leave);

                // send leave event to player of all other players
                response = new Buffer();
                response.Write(player.InstanceId);
                response.Write(QuitType.Normal);
                response.Write(other.Id);
                Request.SendBuffer(player.Client, response, ResponseType.Leave);
            }

        // send quit event to the player
        response = new Buffer();
        response.Write(player.InstanceId);
        response.Write(type);
        if (!string.IsNullOrEmpty(reason))
            response.Write(reason);
        Request.SendBuffer(player.Client, response, ResponseType.Quit, uid);

        // remove player
        player.Status = PlayerStatus.None;
        PlayerManager.Remove(player);

        MasterServer.UpdateImediately();
    }
}