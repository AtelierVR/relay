using System;
using System.Net;
using System.Net.Sockets;
using System.Threading;
using Relay.Requests;
using Relay.Utils;
using Buffer = Relay.Utils.Buffer;

namespace Relay
{
    public static class TCPRecv
    {
        // tcp server
        private static TcpListener? _tcpServer;

        public static void Start()
        {
            var config = Config.Load();
            _tcpServer = new TcpListener(IPAddress.Any, config.GetPort());
            _tcpServer.Start();
            Logger.Log($"TCP Server started on port {config.GetPort()}");
            while (true)
            {
                var client = _tcpServer.AcceptTcpClient();
                var thread = new Thread(() => OnConnection(client));
                thread.Start();
            }
        }

        private static void OnConnection(TcpClient client)
        {
            var clientRemote = new TcpRemote(client);
            var data = new byte[Constants.MaxPacketSize];
            while (true)
                try
                {
                    var read = clientRemote.GetStream().Read(data, 0, data.Length);
                    if (read == 0) break;
                    var buffer = BufferPool.Instance.Rent();
                    buffer.Write(data);
                    buffer.Goto(0);
                    buffer.length = (ushort)read;
                    Request.OnBuffer(clientRemote, buffer);
                    BufferPool.Instance.Return(buffer);
                }
                catch (Exception e)
                {
                    Logger.Error($"Error on connection: {e}");
                    break;
                }
        }
    }

    public class TcpRemote : IRemote
    {
        private readonly TcpClient _remote;
        private readonly NetworkStream _stream;

        public TcpRemote(TcpClient client)
        {
            _remote = client;
            _stream = _remote.GetStream();
        }

        public NetworkStream GetStream() => _stream;

        public override IPAddress Address
            => ((IPEndPoint)_remote.Client.RemoteEndPoint).Address;
        public override ushort Port
            => (ushort)((IPEndPoint)_remote.Client.RemoteEndPoint).Port;

        public override bool Send(byte[] data, int length)
        {
            try
            {
                _stream.Write(data, 0, length);
                return true;
            }
            catch (Exception e)
            {
                Console.WriteLine(e);
                return false;
            }
        }

        public override bool Equals(IRemote obj) =>
            obj is TcpRemote remote && remote.Address.Equals(Address) && remote.Port == Port;
    }
}