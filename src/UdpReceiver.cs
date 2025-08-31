using System.Buffers;
using System.Net;
using System.Net.Sockets;
using Relay.Requests;
using Relay.Utils;
using Buffer = Relay.Utils.Buffer;

namespace Relay;

public static class UdpReceiver {
	private static          Socket?                  _listener;
	private static readonly ArrayPool<byte>          Buffer = ArrayPool<byte>.Shared;

	public static void Start() {
		var config = Config.Load();
		_listener = new Socket(AddressFamily.InterNetwork, SocketType.Dgram, ProtocolType.Udp);
		_listener.Bind(new IPEndPoint(IPAddress.Any, config.GetPort()));

		Logger.Debug($"[UDP Receiver] Started on port {config.GetPort()}");
		Receive();
	}

	private static void Receive() {
		var args = new SocketAsyncEventArgs();
		args.SetBuffer(Buffer.Rent(Constants.MaxPacketSize), 0, Constants.MaxPacketSize);
		args.RemoteEndPoint =  new IPEndPoint(IPAddress.Any, 0);
		args.Completed      += (_, e) => ProcessReceive(e);
		if (!_listener!.ReceiveFromAsync(args))
			ProcessReceive(args);
	}

	private static void ProcessReceive(SocketAsyncEventArgs e) {
		while (true) {
			if (e is { BytesTransferred: > 0, Buffer: not null, SocketError: SocketError.Success, RemoteEndPoint: IPEndPoint endpoint }) {
				var remote = new UdpRemote(_listener!, endpoint);
				var copy   = new byte[e.BytesTransferred];
				System.Buffer.BlockCopy(e.Buffer, 0, copy, 0, e.BytesTransferred);
				Request.OnBuffer(remote, copy);
			}

			e.RemoteEndPoint = new IPEndPoint(IPAddress.Any, 0);
			if (!_listener!.ReceiveFromAsync(e)) continue;
			break;
		}
	}

	public static void Stop()
	{
		_listener?.Close();
		_listener?.Dispose();
		_listener = null;
		Logger.Debug("[UDP Receiver] Stopped.");
	}
}

public class UdpRemote(Socket socket, IPEndPoint endPoint) : Remote {
	public override IPAddress Address
		=> endPoint.Address;

	public override ushort Port
		=> (ushort)endPoint.Port;

	public override bool Send(byte[] data, int length) {
		try {
			socket.SendTo(data, 0, length, SocketFlags.None, endPoint);
			return true;
		} catch (Exception e) {
			Console.WriteLine(e);
			return false;
		}
	}

	public override bool Equals(Remote obj)
		=> obj is UdpRemote remote && remote.Address.Equals(Address) && remote.Port == Port;
}