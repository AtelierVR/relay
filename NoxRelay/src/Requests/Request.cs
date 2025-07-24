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
            Logger.Debug($"{buffer} sent to {client}");
            return client.Remote.Send(buffer.ToBuffer(), buffer.length) ? uid : ushort.MaxValue;
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