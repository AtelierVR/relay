using Relay.Clients;
using Relay.Players;
using Relay.Utils;

namespace Relay.Instances;

public class Instance
{
    public readonly byte InternalId;
    public uint MasterId;
    public InstanceFlags Flags;
    public ushort Capacity = 0;
    public byte MaxTps = 24;
    private List<UserModered> _modereds = [];
    public World World;

    public UserModered? GetModered(uint userId, string address)
        => _modereds.FirstOrDefault(m => m.UserId == userId || m.Address == address);

    public UserModered? GetModered(User user) 
        => GetModered(user.Id, user.Address);

    private string? _password;

    public string? Password
    {
        get => Flags.HasFlag(InstanceFlags.UsePassword) ? _password : null;
        set
        {
            if (string.IsNullOrEmpty(value))
            {
                Flags &= ~InstanceFlags.UsePassword;
                _password = null;
            }
            else
            {
                Flags |= InstanceFlags.UsePassword;
                _password = value;
            }
        }
    }

    public bool VerifyPassword(string password) 
        => string.IsNullOrEmpty(Password) || (!string.IsNullOrEmpty(password) && Hashing.Verify(password, Password));

    public Player[] Players
        => PlayerManager.GetFromInstance(InternalId);

    public Instance()
    {
        Flags = InstanceFlags.None;
        InternalId = InstanceManager.GetNextInternalId();
        InstanceManager.Add(this);
    }

    public override string ToString() 
        => $"{GetType().Name}[InternalId={InternalId}, MasterId={MasterId}, PlayerCount={Players.Length}/{Capacity}]";

    public Player[] GetPlayers()
        => _players.ToArray();
}