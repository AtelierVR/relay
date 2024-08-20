using Relay.Clients;
using Relay.Players;
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
        var instanceId = buffer.ReadUShort();
        var player = PlayerManager.GetFromClientInstance(client.Id, instanceId);
        if (player is not { Status: PlayerStatus.Configuration }) return;
        var action = buffer.ReadEnum<ConfigurationAction>();
        var response = new Buffer();
        response.Write(instanceId);
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