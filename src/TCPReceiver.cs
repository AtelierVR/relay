using System.Buffers;
using System.Net;
using System.Net.Sockets;
using Relay.Requests;
using Relay.Utils;

namespace Relay;

public static class TcpReceiver
{
	private static Socket? _listener;
	private static readonly ArrayPool<byte> Buffer = ArrayPool<byte>.Shared;

	public static void Start()
	{
		var config = Config.Load();
		_listener = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
		_listener.Bind(new IPEndPoint(IPAddress.Any, config.GetPort()));
		_listener.Listen(100);

		Logger.Debug($"[TCP Receiver] Started on port {config.GetPort()}");
		Accept();
	}

	private static void Accept()
	{
		var args = new SocketAsyncEventArgs();
		args.Completed += (s, e) => ProcessAccept(e);
		if (!_listener!.AcceptAsync(args))
			ProcessAccept(args);
	}

	private static void ProcessAccept(SocketAsyncEventArgs e)
	{
		while (true)
		{
			var client = e.AcceptSocket;
			if (client is { Connected: true })
			{
				var remote = new TcpRemote(client);
				var recvArgs = new SocketAsyncEventArgs();
				recvArgs.SetBuffer(Buffer.Rent(Constants.MaxPacketSize), 0, Constants.MaxPacketSize);
				recvArgs.UserToken = client;
				recvArgs.Completed += (_, ev) => ProcessReceive(ev);
				if (!client.ReceiveAsync(recvArgs)) ProcessReceive(recvArgs);
			}

			e.AcceptSocket = null;
			if (!_listener!.AcceptAsync(e)) continue;
			break;
		}
	}

	private static void ProcessReceive(SocketAsyncEventArgs e)
	{
		while (true)
		{
			var client = new TcpRemote((Socket)e.UserToken!);
			if (e is { BytesTransferred: > 0, Buffer: not null, SocketError: SocketError.Success, UserToken: not null })
			{
				var copy = new byte[e.BytesTransferred];
				System.Buffer.BlockCopy(e.Buffer, 0, copy, 0, e.BytesTransferred);
				Request.OnBuffer(client, copy);
				if (!client.Socket.ReceiveAsync(e)) continue;
			}
			else
			{
				Buffer.Return(e.Buffer!);
				client.Socket.Close();
			}

			break;
		}
	}

	public static void Stop()
	{
		_listener?.Close();
		_listener?.Dispose();
		_listener = null;
		Logger.Debug("[TCP Receiver] Stopped.");
	}
}

public class TcpRemote(Socket socket) : Remote
{
	public Socket Socket
		=> socket;

	public override IPAddress Address
		=> socket.RemoteEndPoint is IPEndPoint ep ? ep.Address : IPAddress.None;

	public override ushort Port
		=> socket.RemoteEndPoint is IPEndPoint ep ? (ushort)ep.Port : (ushort)0;

	public override bool Send(byte[] data, int length)
	{
		try
		{
			socket.Send(data, 0, length, SocketFlags.None);
			return true;
		}
		catch (Exception e)
		{
			Console.WriteLine(e);
			return false;
		}
	}

	public override bool Equals(Remote obj)
		=> obj is TcpRemote remote && remote.Address.Equals(Address) && remote.Port == Port;
}