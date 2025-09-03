using System;
using Relay.Avatars;
using Relay.Clients;
using Relay.Instances;
using Relay.Requests.Instances.Enter;
using Relay.Requests.Instances.Quit;

namespace Relay.Players;

public class Player
{
	public ushort Id;
	public ushort ClientId;
	public byte InstanceId;
	public PlayerStatus Status = PlayerStatus.None;
	private string? _display;
	public DateTimeOffset CreatedAt = DateTimeOffset.Now;
	public PlayerFlags Flags = PlayerFlags.None;
	public Avatar? Avatar = null;

	public byte CustomTps = 0;
	public float CustomThreshold = 0.0f;

	public PlayerTransform Transforms = new();

	public UserModerated? GetModerated()
		=> Client.User != null
			? Instance.GetModerated(Client.User.Value)
			: null;

	public bool HasPrivilege
		=> Flags.HasFlag(PlayerFlags.HasPrivilege);

	public string Display
	{
		get
			=> (string.IsNullOrEmpty(_display) ? null : _display)
				?? Client.User?.DisplayName ?? $"Player {Id}";
		init => _display = value;
	}

	public Client Client
		=> ClientManager.Get(ClientId)!;

	public Instance Instance
		=> InstanceManager.Get(InstanceId)!;

	public bool HasHighPrivilege
		=> Flags.HasFlag(PlayerFlags.HasHigthPrivilege);

	public override string ToString()
		=> $"{GetType().Name}[Id={Id}, ClientId={ClientId}, InstanceId={InstanceId}, Status={Status}, Display={Display}]";

	public void Leave(QuitType type, string? reason = null, ushort uid = 0)
	{
		QuitHandler.LeavePlayer(this, type, HasPrivilege ? reason : null, this, uid);
		Instance.RemovePlayer(this);
	}

	public void Enter(Instance instance, ushort uid = ushort.MinValue)
	{
		InstanceId = instance.InternalId;
		Instance.AddPlayer(this);
		Status = PlayerStatus.Preparing;
		EnterHandler.SendEnter(this, uid);
		foreach (var other in instance.GetPlayers())
		{
			if (other.ClientId == ClientId) continue;
			EnterHandler.SendJoin(Client, other);
			EnterHandler.SendJoin(other.Client, this);
		}
	}
}