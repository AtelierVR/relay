using System;
using Relay.Clients;
using Relay.Instances;

namespace Relay.Players;

public class Player
{
    public ushort Id;
    public uint Uid;
    public ushort ClientId;
    public byte InstanceId;
    public PlayerStatus Status = PlayerStatus.None;
    private string? _display;
    public DateTimeOffset CreatedAt = DateTimeOffset.Now;
    public PlayerFlags Flags = PlayerFlags.None;

    public PlayerTransform Transforms = new();

    public UserModered? Modered 
        => Client.User != null 
        ? Instance.GetModered(Client.User) 
        : null;

    public bool IsModerator
        => (Flags & PlayerFlags.IsModerator) != 0
        || (Flags & PlayerFlags.IsOwner) != 0
        || Modered?.IsModerator == true;

    public string Display
    {
        get => _display ?? Client.User?.DisplayName ?? $"Player {Id}";
        set => _display = value;
    }

    public Client Client 
        => ClientManager.Get(ClientId);

    public Instance Instance 
        => InstanceManager.Get(InstanceId);

    public override string ToString() => $"{GetType().Name}[Id={Id}, ClientId={ClientId}, InstanceId={InstanceId}, Status={Status}, Display={Display}]";
}