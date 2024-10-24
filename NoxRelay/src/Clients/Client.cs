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
        public Platfrom Platform = Platfrom.None;
        public Engine Engine = Engine.None;
        public DateTimeOffset LastSeen = DateTimeOffset.MinValue;
        public User? User = null;

        public List<Player> Players => PlayerManager.GetFromClient(Id);

        public Client()
        {
            Id = ClientManager.NextId();
            ClientManager.Add(this);
        }

        public void OnConnect()
        {
            Logger.Log($"{this} connected");
        }

        public void OnDisconnect(string reason)
        {
            Players.ForEach(player => Handler.Get<QuitHandler>().LeavePlayer(player, QuitType.Normal, null, player));
            Logger.Log($"{this} disconnected");
        }

        public void OnReceive(Buffer buffer)
        {
            Logger.Debug($"{this} received {buffer}");
        }

        public override string ToString() => $"{GetType().Name}[Id={Id}, Remote={Remote}, Status={Status}]";

        public void OnTimeout()
        {
            Logger.Warning($"{this} timed out");
            Players.ForEach(player => Handler.Get<QuitHandler>().LeavePlayer(player, QuitType.Timeout, null, player));
            Handler.Get<DisconnectHandler>().SendEvent(this, Messages.ConnectionTimeout);
            ClientManager.Remove(this);
        }
    }
}