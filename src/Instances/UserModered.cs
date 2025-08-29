namespace Relay.Instances;

public class UserModered
{
    public uint UserId;
    public string Address;
    
    public bool IsBlacklisted;
    public string BlacklistReason;
    public DateTimeOffset BlacklistExpiresAt;
    
    
    public bool IsWhitelisted;
    public bool IsModerator;
    
    
}