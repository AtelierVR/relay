using Relay.Instances;
using Relay.Players;
using Relay.Requests.Disconnect;
using Relay.Requests.Instances.Quit;
using Relay.Utils;

namespace Relay.Clients;

public class Client {
	public ushort         Id = ushort.MaxValue;
	public Remote         Remote;
	public string         Platform = string.Empty;
	public string         Engine   = string.Empty;
	public DateTimeOffset LastSeen = DateTimeOffset.MinValue;
	public User?          User     = null;

	public bool TryGetUser(out User user) {
		if (User is { Id: > Clients.User.InvalidId } u) {
			user = u;
			return true;
		}

		user = default;
		return false;
	}

	public bool IsAuthenticated
		=> User is { Id: > Clients.User.InvalidId };

	public bool IsAuthenticating
		=> User is { Id: Clients.User.InvalidId };

	public bool IsHandshake = false;

	public List<Player> GetPlayers() {
		List<Player> players = [];
		foreach (var instance in InstanceManager.Instances)
			players.AddRange(instance.Players.Where(player => player.ClientId == Id));
		return players;
	}

	public Player? GetInstancePlayer(byte instanceId) {
		var instance = InstanceManager.Get(instanceId);
		return instance?.Players.FirstOrDefault(player => player.ClientId == Id);
	}

	public Client(Remote remote) {
		Id = ClientManager.NextId();
		ClientManager.Add(this);
		Remote = remote;
	}

	public void OnConnect() {
		Logger.Log($"{this} connected");
	}

	public void OnDisconnect(string? reason) {
		foreach (var player in GetPlayers())
			QuitHandler.LeavePlayer(player, QuitType.Normal, player.HasPrivilege ? reason : null, player);
		Logger.Log($"{this} disconnected{(reason != null ? " - " + reason : string.Empty)}");
		ClientManager.Remove(this);
	}

	public void OnTimeout() {
		foreach (var player in GetPlayers())
			QuitHandler.LeavePlayer(player, QuitType.Timeout, null, player);
		DisconnectHandler.SendEvent(this, Messages.ConnectionTimeout);
		ClientManager.Remove(this);
	}

	public override string ToString()
		=> $"{GetType().Name}[Id={Id}, Remote={Remote}]";
}