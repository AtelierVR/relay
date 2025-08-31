using System;
using Relay.Avatars;
using Relay.Clients;
using Relay.Instances;

namespace Relay.Players;

public class Player {
	public  ushort         Id;
	public  ushort         ClientId;
	public  byte           InstanceId;
	public  PlayerStatus   Status = PlayerStatus.None;
	private string?        _display;
	public  DateTimeOffset CreatedAt = DateTimeOffset.Now;
	public  PlayerFlags    Flags     = PlayerFlags.None;
	public  Avatar?        Avatar    = null;

	public byte  CustomTps       = 0;
	public float CustomThreshold = 0.0f;

	public PlayerTransform Transforms = new();

	public UserModerated? GetModerated()
		=> Client.User != null
			? Instance.GetModerated(Client.User.Value)
			: null;

	public bool HasPrivilege
		=> Flags.HasFlag(PlayerFlags.HasPrivilege);

	public string Display {
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
}