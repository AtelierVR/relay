using Newtonsoft.Json.Linq;
using Relay.Clients;
using Relay.Players;
using Relay.Requests.Instances.Enter;
using Relay.Requests.Instances.Transform;
using Relay.Utils;
using Buffer = Relay.Utils.Buffer;

namespace Relay.Requests.Instances.Config;

public class ConfigHandler : Handler
{
    public override void OnReceive(Buffer buffer, Client client)
    {
        if (client.Status != ClientStatus.Authentificated) return;
        buffer.Goto(0);
        var length = buffer.ReadUShort();
        var uid = buffer.ReadUShort();
        var type = buffer.ReadEnum<RequestType>();
        if (type != RequestType.Configuration) return;
        Logger.Debug($"{client} sent configuration");
        var internalId = buffer.ReadByte();
        var action = buffer.ReadEnum<ConfigurationAction>();
        var player = PlayerManager.GetFromClientInstance(client.Id, internalId);
        Logger.Debug($"{player} sent configuration {action}");
        if (player is not { Status: PlayerStatus.Configuration }) return;

        var response = new Buffer();
        response.Write(internalId);
        switch (action)
        {
            case ConfigurationAction.Ready:
                ReadyHandler(response, player, uid);
                break;
            case ConfigurationAction.WorldData:
                WorldDataHandler(response, player, uid);
                break;
            case ConfigurationAction.WorldLoaded:
                break;
            case ConfigurationAction.Error:
                break;
        }
    }

    private static void ReadyHandler(Buffer buffer, Player player, ushort uid)
    {
        player.Status = PlayerStatus.Ready;
        var players = PlayerManager.GetFromInstance(player.InstanceId);
        foreach (var other in players.Where(other => other != player))
        {
            if (other is not { Status: PlayerStatus.Ready }) continue;
            EnterHandler.SendJoin(other.Client, player);
            EnterHandler.SendJoin(player.Client, other);
            foreach (var transform in other.Transforms.GetPairs())
                TransformHandler.SendTransform(player.Client, player.InstanceId, other.Id, transform.Key, transform.Value);
        }
        Logger.Log($"{player} is ready");
    }

    private static void WorldDataHandler(Buffer buffer, Player player, ushort uid)
    {
        var instance = player.Instance;
        buffer.Write(ConfigurationAction.WorldData);
        buffer.Write(instance.World.MasterId);
        buffer.Write(instance.World.Address);
        buffer.Write(instance.World.Version);
        Request.SendBuffer(player.Client, buffer, ResponseType.Configuration, uid);
    }
}