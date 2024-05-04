using System.Linq;

namespace Relay.Master.Update;

public class ResponseUpdate
{
    public string relay { get; set; }
    public ushort next_update { get; set; }
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
    public uint WorldId => uint.Parse(world_ref.Split('@')[0].Split(';')[0]);
    public string ServerAddress => world_ref.Split('@')[1];
    public ushort Version => ushort.Parse(world_ref.Split('@')[0].Split(';').FirstOrDefault(s => s.StartsWith("v="))
        ?.Split('=')[1] ?? ushort.MaxValue.ToString());
}