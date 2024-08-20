namespace Relay.Requests.Instances.Config;

public enum ConfigurationAction : byte
{
    Ready = 0,
    Error = 1,
    WorldData = 2,
    WorldLoaded = 3
}