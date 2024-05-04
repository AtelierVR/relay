namespace Relay.Master.Update;

public class RequestUpdate
{
    public byte max_instances { get; set; }
    public RequestClient[] clients { get; set; }
    public RequestInstance[] instances { get; set; }
}
public class RequestClient
{
    public ushort id;
    public string remote;
    public string status;
    public string platform;
    public string engine;
    public ulong last_seen;
}
public class RequestInstance
{
    public ushort internal_id;
    public uint master_id;
    public uint flags;
    public byte[] players;
    public ushort max_players;
}