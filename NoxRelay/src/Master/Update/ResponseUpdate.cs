using System;
using System.Linq;

namespace Relay.Master.Update;

public class ResponseUpdate
{
    public string relay { get; set; }
    public ushort next_update { get; set; }
    public string server { get; set; }
    public ResponseInstance[] instances { get; set; }
}

public class ResponseInstance
{
    public uint master_id { get; set; }
    public ushort capacity { get; set; }
    public uint flags { get; set; }
    public string password { get; set; }

    // <id>[;v=<version>]@<server_address>
    public string world_ref { get; set; }
    public uint WorldId() => uint.Parse(world_ref.Split('@')[0].Split(';')[0]);
    public ushort Version() => ushort.Parse(world_ref.Split('@')[0].Split(';').FirstOrDefault(s => s.StartsWith("v="))
        ?.Split('=')[1] ?? ushort.MaxValue.ToString());

    public string WorldServer() =>
        world_ref.Split('@').Length == 2 ?
            (string.IsNullOrEmpty(world_ref.Split('@')[1]) || world_ref.Split('@')[1] != "::" ? null : world_ref.Split('@')[1])
            : null;
}