namespace Relay.Clients;

public struct User
{
    public const uint InvalidId = 0;

    public uint Id;
    public string Username;
    public string DisplayName;
    public string Address;
}