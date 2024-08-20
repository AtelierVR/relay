using System.Collections.Generic;
using System.Linq;
using Relay.Clients;

namespace Relay.Players;

public class PlayerManager
{
    public static List<Player> Players { get; set; } = new();

    public static List<Player> GetFromInstance(ushort instanceId) =>
        Players.FindAll(player => player.InstanceId == instanceId);

    public static Player GetFromClientInstance(ushort clientId, ushort instanceId) =>
        Players.Find(player => player.ClientId == clientId && player.InstanceId == instanceId);
    
    public static List<Player> GetFromClient(ushort clientId) => Players.FindAll(player => player.ClientId == clientId);
    
    public static void Add(Player player) => Players.Add(player);
    
    public static ushort GetNextId(ushort instanceId)
    {
        ushort id = 0;
        while (Players.Exists(player => player.Id == id && player.InstanceId == instanceId))
            id++;
        return id;
    }

    public static void Remove(Player player) => Players.Remove(player);
}