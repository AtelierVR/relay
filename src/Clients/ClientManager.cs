using Relay.Utils;

namespace Relay.Clients;

public class ClientManager
{
    public static readonly List<Client> Clients = [];

    public static Client? Get(Remote remote)
        => Clients.FirstOrDefault(client => client.Remote.Equals(remote));

    public static Client? Get(ushort id)
        => Clients.FirstOrDefault(client => client.Id == id);

    public static Client Add(Client client)
    {
        Clients.Add(client);
        return client;
    }

    public static Client Remove(Client client)
    {
        Clients.Remove(client);
        return client;
    }

    public static ushort NextId()
    {
        ushort id = 0;
        while (Clients.Any(client => client.Id == id))
            id++;
        return id;
    }
}
