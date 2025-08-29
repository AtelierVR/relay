
namespace Relay.Avatars;

public class Avatar
{
    public uint Id = 0;
    public string Server = string.Empty;
    public ushort Version = ushort.MaxValue;

    public override string ToString()
        => $"{GetType().Name}[Id={Id}, Server={Server}, Version={Version}]";
}

