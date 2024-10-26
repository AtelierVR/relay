namespace Relay.Master.Update;

public class RequestUpdate
{
    public byte max_instances { get; set; }
    public ushort port { get; set; }
    public string use_address { get; set; }
    public RequestClient[] clients { get; set; }
    public RequestInstance[] instances { get; set; }

    public override string ToString() => $"{GetType().Name}[max_instances={max_instances}, clients={clients.Length}, instances={instances.Length}]";
}

public class RequestUser
{
    public uint id { get; set; }
    public string address { get; set; }
}

public class RequestClient
{
    public ushort id { get; set; }
    public string remote { get; set; }
    public string status { get; set; }
    public string platform { get; set; }
    public string engine { get; set; }
    public ulong last_seen { get; set; }
    public RequestUser? user { get; set; }
}


public class RequestPlayer
{
    public ushort id { get; set; }
    public ushort client_id { get; set; }
    public string display { get; set; }
    public uint flags { get; set; }
    public byte status { get; set; }
    public ulong created_at { get; set; }
}

public class RequestInstance
{
    public ushort internal_id { get; set; }
    public uint master_id { get; set; }
    public uint flags { get; set; }
    public RequestPlayer[] players { get; set; }
    public ushort max_players { get; set; }
}
