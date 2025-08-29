using System;
using System.Threading.Tasks;
using Relay.Clients;
using Relay.Utils;
using Buffer = Relay.Utils.Buffer;

namespace Relay.Requests
{
    public class Request
    {
        public static void OnBuffer(IRemote remote, Buffer buffer)
        {
            var client = ClientManager.Get(remote);
            if (client == null)
            {
                client = new Client { Remote = remote };
                client.OnConnect();
            }
            client.LastSeen = DateTimeOffset.UtcNow;

            client.OnReceive(buffer);
            buffer.Goto(0);
            var length = buffer.ReadUShort();

            if (buffer.length < 5 || buffer.length < length)
            {
                Logger.Warning($"{client} sent invalid buffer");
                return;
            }

            // Check if this is a multipacket
            buffer.Goto(4); // Skip length and uid
            var requestType = buffer.ReadEnum<RequestType>();
            
            if (requestType == RequestType.MultiPacketStart || 
                requestType == RequestType.MultiPacketData || 
                requestType == RequestType.MultiPacketEnd)
            {
                // Handle multipacket directly
                buffer.Goto(0);
                var multiPacketHandler = new Requests.MultiPacket.MultiPacketHandler();
                multiPacketHandler.OnReceive(buffer, client);
                return;
            }

            buffer.Goto(0);
            foreach (var handler in Handler.Handlers)
                handler.OnReceive(buffer, client);
        }

        public static ushort SendBuffer(Client client, Buffer data, ResponseType type, ushort uid = 0x0000)
        {
            var buffer = new Buffer(2);
            buffer.Write(uid);
            buffer.Write(type);
            buffer.Write(data);
            buffer.Goto(0);
            buffer.Write(buffer.length);
            
            if (buffer.length > Constants.MaxPacketSize)
                return SendLargeBuffer(client, data.ToBuffer(), type, uid);
            
            return client.Remote.Send(buffer.ToBuffer(), buffer.length) ? uid : ushort.MaxValue;
        }

        public static ushort SendLargeBuffer(Client client, byte[] data, ResponseType type, ushort uid = 0x0000)
        {
            Logger.Debug($"Fragmenting large packet of {data.Length} bytes for {client}");
            
            var fragments = MultiPacketManager.FragmentData(data, type, uid);
            if (fragments.Count == 0)
            {
                Logger.Warning($"Failed to fragment large packet for {client}");
                return ushort.MaxValue;
            }

            // Send start packet
            var startBuffer = new Buffer(2);
            startBuffer.Write(uid);
            startBuffer.Write(ResponseType.MultiPacketStart);
            startBuffer.Write(fragments[0]);
            startBuffer.Goto(0);
            startBuffer.Write(startBuffer.length);
            
            if (!client.Remote.Send(startBuffer.ToBuffer(), startBuffer.length))
            {
                Logger.Warning($"Failed to send multipacket start to {client}");
                return ushort.MaxValue;
            }

            // Send data packets
            for (int i = 1; i < fragments.Count - 1; i++)
            {
                var dataBuffer = new Buffer(2);
                dataBuffer.Write((ushort)(uid + i)); // Use incremental UIDs for tracking
                dataBuffer.Write(ResponseType.MultiPacketData);
                dataBuffer.Write(fragments[i]);
                dataBuffer.Goto(0);
                dataBuffer.Write(dataBuffer.length);
                
                if (!client.Remote.Send(dataBuffer.ToBuffer(), dataBuffer.length))
                {
                    Logger.Warning($"Failed to send multipacket data {i} to {client}");
                    return ushort.MaxValue;
                }
            }

            // Send end packet
            var endBuffer = new Buffer(2);
            endBuffer.Write((ushort)(uid + fragments.Count - 1));
            endBuffer.Write(ResponseType.MultiPacketEnd);
            endBuffer.Write(fragments[fragments.Count - 1]);
            endBuffer.Goto(0);
            endBuffer.Write(endBuffer.length);
            
            if (!client.Remote.Send(endBuffer.ToBuffer(), endBuffer.length))
            {
                Logger.Warning($"Failed to send multipacket end to {client}");
                return ushort.MaxValue;
            }

            Logger.Debug($"Successfully sent large packet in {fragments.Count} fragments to {client}");
            return uid;
        }

        public static async void Check()
        {
            var config = Config.Load();
            Logger.Debug($"Clients timeout set to {config.GetConnectionTimeout()} seconds");
            while (true)
            {
                foreach (var client in ClientManager.Clients.ToArray())
                    if (client.LastSeen.AddSeconds(config.GetConnectionTimeout()) < DateTime.UtcNow)
                        client.OnTimeout();
                await Task.Delay(1000);
            }
        }
    }
}