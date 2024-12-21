using System;
using Relay.Clients;
using Relay.Instances;

namespace Relay.Players;

public class Player
{
    public ushort Id;
    public uint Uid;
    public ushort ClientId;
    public ushort InstanceId;
    public PlayerStatus Status = PlayerStatus.None;
    private string _display;
    public DateTimeOffset CreatedAt = DateTimeOffset.Now;
    public PlayerFlags Flags = PlayerFlags.None;

    public PlayerTransform Transforms = new();

    public UserModered? Modered => Instance.GetModered(Client.User);
    public bool IsModerator 
        => (Flags & PlayerFlags.IsModerator) != 0
        || (Flags & PlayerFlags.IsOwner) != 0
        || Modered?.IsModerator == true;

    public string Display
    {
        get => string.IsNullOrEmpty(_display) ? Client.User.DisplayName : _display;
        set => _display = value;
    }

    public Player()
    {
        Id = PlayerManager.GetNextId(InstanceId);
    }

    public Client Client => ClientManager.Get(ClientId);
    public Instance Instance => InstanceManager.Get(InstanceId);
    
    public override string ToString() => $"{GetType().Name}[Id={Id}, ClientId={ClientId}, InstanceId={InstanceId}, Status={Status}, Display={Display}]";
}