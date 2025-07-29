using Relay.Clients;
using Relay.Master;
using Relay.Players;
using Relay.Utils;
using Buffer = Relay.Utils.Buffer;

namespace Relay.Requests.Instances.Traveling;

public class TravelingHandler : Handler
{
    public override void OnReceive(Buffer buffer, Client client)
    {
        if (client.Status != ClientStatus.Authentificated) return;
        buffer.Goto(0);
        var length = buffer.ReadUShort();
        var uid = buffer.ReadUShort();
        var type = buffer.ReadEnum<RequestType>();
        if (type != RequestType.Traveling) return;
        var internalId = buffer.ReadByte();
        var action = buffer.ReadEnum<TravelingAction>();
        var player = client.GetInstancePlayer(internalId);
        if (player is not { Status: > PlayerStatus.None }) return;
        Logger.Debug($"{player} sent traveling action {action}");

        if (action == TravelingAction.Travel)
        {
            Travel(player, uid);
            return;
        }

        // others actions

        var response = new Buffer();
        response.Write(player.InstanceId);
        response.Write(TravelingResults.Unknown);
        response.Write($"Action {action} is not supported yet");
        Request.SendBuffer(client, response, ResponseType.Traveling, uid);
    }

    public void Travel(Player player, ushort uid = 0)
    {
        if (player.Status == PlayerStatus.None) return;

        var world = player.Instance.World;

        var response = new Buffer();
        response.Write(player.InstanceId);
        response.Write(TravelingResults.UseMaster);
        response.Write(world.MasterId);
        response.Write(world.Address);
        response.Write(world.Version);

        Request.SendBuffer(player.Client, response, ResponseType.Traveling, uid);

        player.Status = PlayerStatus.Traveling;

        MasterServer.UpdateImediately();
    }
}