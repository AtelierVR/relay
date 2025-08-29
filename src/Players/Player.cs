using System;
using Relay.Avatars;
using Relay.Clients;
using Relay.Instances;

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
    public Avatar? avatar = null;

    public byte CustomTps = 0;
    public float CustomThreshold = 0.0f;

    public PlayerTransform Transforms = new();

    public UserModered? Modered 
        => Client.User != null 
        ? Instance.GetModered(Client.User) 
        : null;

    public bool HasPrivilege 
        => Flags.HasFlag(PlayerFlags.HasPrivilege);

    public string Display
    {
        get => (string.IsNullOrEmpty(_display) ? null : _display)
            ?? Client.User?.DisplayName ?? $"Player {Id}";
        set => _display = value;
    }

    public Client Client 
        => ClientManager.Get(ClientId);

    public Instance Instance 
        => InstanceManager.Get(InstanceId);

    public override string ToString() 
        => $"{GetType().Name}[Id={Id}, ClientId={ClientId}, InstanceId={InstanceId}, Status={Status}, Display={Display}]";
}