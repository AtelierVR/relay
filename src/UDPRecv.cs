using System;
using System.Net;
using System.Net.Sockets;
using Relay.Requests;
using Relay.Utils;
using Buffer = Relay.Utils.Buffer;

namespace Relay
{
    public static class UdpRecv
    {
        private static UdpClient? _udpServer;

        public static void Start()
        {
            var config = Config.Load();
            _udpServer = new UdpClient(config.GetPort());
            var remoteEp = new IPEndPoint(IPAddress.Any, config.GetPort());
            Logger.Log($"UDP Server started on port {config.GetPort()}");
            while (true)
                try
                {
                    var data = _udpServer.Receive(ref remoteEp);
                    var buffer = BufferPool.Instance.Rent();
                    buffer.Write(data);
                    buffer.Goto(0);
                    buffer.length = (ushort)data.Length;
                    Request.OnBuffer(new UdpRemote(remoteEp, _udpServer), buffer);
                    BufferPool.Instance.Return(buffer);
                }
                catch (Exception e)
                {
                    Logger.Error($"Error on connection: {e}");
                }
        }
    }

    public class UdpRemote : IRemote
    {
        private readonly IPEndPoint _client;
        private readonly UdpClient _server;

        public UdpRemote(IPEndPoint client, UdpClient server)
        {
            _client = client;
            _server = server;
        }

        public override IPAddress Address
            => _client.Address;

        public override ushort Port
            => (ushort)_client.Port;

        public override bool Send(byte[] data, int length)
        {
            _server.Send(data, length, _client);
            return true;
        }

        public override bool Equals(IRemote obj)
            => obj is UdpRemote remote && remote.Address.Equals(Address) && remote.Port == Port;
    }
}