using Relay.Instances;
using Relay.Players;
using Relay.Requests.Disconnect;
using Relay.Requests.Instances.Quit;
using Relay.Utils;
using Buffer = Relay.Utils.Buffer;

namespace Relay.Clients
{
    public class Client
    {
        public ushort Id;
        public IRemote Remote;
        public ClientStatus Status;
        public string Platform = "";
        public string Engine = "";
        public DateTimeOffset LastSeen = DateTimeOffset.MinValue;
        public User? User = null;

        public List<Player> GetPlayers()
        {
            List<Player> players = new();
            foreach (var instance in InstanceManager.Instances)
                foreach (var player in instance.Players)
                    if (player.ClientId == Id)
                        players.Add(player);
            return players;
        }

        public Player? GetInstancePlayer(byte instanceId)
        {
            var instance = InstanceManager.Get(instanceId);
            if (instance == null) return null;
            return instance.Players.FirstOrDefault(player => player.ClientId == Id);
        }

        public Client(IRemote remote)
        {
            Id = ClientManager.NextId();
            ClientManager.Add(this);
            Remote = remote;
        }

        public void OnConnect()
        {
            Logger.Log($"{this} connected");
        }

        public void OnDisconnect(string reason)
        {
            foreach (var player in GetPlayers())
                QuitHandler.LeavePlayer(player, QuitType.Normal, null, player);
            Logger.Log($"{this} disconnected - {reason ?? "normal"}");
        }

        public void OnReceive(Buffer buffer)
        {
            // Logger.Debug($"{this} received {buffer}");
        }

        public override string ToString()
            => $"{GetType().Name}[Id={Id}, Remote={Remote}, Status={Status}]";

        public void OnTimeout()
        {
            Logger.Warning($"{this} timed out");
            foreach (var player in GetPlayers())
                QuitHandler.LeavePlayer(player, QuitType.Timeout, null, player);
            DisconnectHandler.SendEvent(this, Messages.ConnectionTimeout);
            ClientManager.Remove(this);
        }
    }
}