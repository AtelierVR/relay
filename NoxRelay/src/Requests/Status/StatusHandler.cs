using Relay.Clients;
using Relay.Instances;
using Relay.Master;
using Relay.Utils;
using Buffer = Relay.Utils.Buffer;

namespace Relay.Requests.Status;

public class StatusHandler : Handler
{
    public override void OnReceive(Buffer buffer, Client client)
    {
        if (client.Status == ClientStatus.Disconnected) return;
        buffer.Goto(0);
        var length = buffer.ReadUShort();
        var uid = buffer.ReadUShort();
        var type = buffer.ReadEnum<RequestType>();
        if (type != RequestType.Status) return;
        Logger.Debug($"{client} sent status");
        var pagnation = buffer.ReadByte();
        var response = new Buffer();
        response.Write((byte)0x00);
        response.Write(MasterServer.MasterAddress);
        var pages = GetPages((ushort)(Constants.MaxPacketSize - response.length - 2));
        Logger.Debug($"{client} requested page {pagnation} of {pages.Length}");
        if (pagnation >= pages.Length)
            response.Write((byte)0);
        else response.Write(pages[pagnation]);
        response.Write(pagnation);
        response.Write((byte)pages.Length);
        Request.SendBuffer(client, response, ResponseType.Status, uid);
    }

    private static Buffer[] GetPages(ushort bytesPerPage)
    {
        var liBuffer = new List<Buffer>();
        var buffer = new Buffer(1);
        byte nbinstance = 0;
        foreach (var instance in InstanceManager.Instances)
        {
            var instanceBuffer = new Buffer();
            instanceBuffer.Write(instance.Flags);
            instanceBuffer.Write(instance.InternalId);
            instanceBuffer.Write(instance.MasterId);
            instanceBuffer.Write((ushort)instance.Players.Count);
            instanceBuffer.Write(instance.Capacity);

            if (buffer.length + instanceBuffer.length > bytesPerPage || nbinstance >= byte.MaxValue)
            {
                buffer.Goto(0);
                buffer.Write(nbinstance);
                liBuffer.Add(buffer);
                buffer = new Buffer(1);
                nbinstance = 0;
            }

            buffer.Write(instanceBuffer);
            nbinstance++;
        }

        if (nbinstance > 0)
        {
            buffer.Goto(0);
            buffer.Write(nbinstance);
            liBuffer.Add(buffer);
        }

        return liBuffer.ToArray();
    }
}