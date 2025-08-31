namespace Relay.Requests.Instances.ServerConfig;

[Flags]
public enum ServerConfigFlags : byte
{
    None = 0x00,
    Tps = 0x01,
    Threshold = 0x02,
    Capacity = 0x04,
    Password = 0x08,
    Flags = 0x10,
    
    // Combinaisons courantes
    Performance = Tps | Threshold,
    Basic = Tps | Capacity,
    Security = Password | Flags,
    All = Tps | Threshold | Capacity | Password | Flags
}
