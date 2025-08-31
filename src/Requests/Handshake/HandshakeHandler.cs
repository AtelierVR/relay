using Relay.Clients;
using Relay.Master;
using Relay.Packets;
using Relay.Priority;
using Relay.Utils;
using Buffer = Relay.Utils.Buffer;

namespace Relay.Requests.Handshake;

public class HandshakeHandler : Handler {
	protected override void OnSetup() {
		PacketPriorityManager.SetMinimumPriority(RequestType.Handshake, EPriority.Critical);
		PacketDispatcher.RegisterHandler(RequestType.Handshake, OnHandshake);
	}

	private static void OnHandshake(PacketData data) {
		var protocol = data.Payload.ReadUShort();
		if (protocol != Constants.ProtocolVersion) {
			Logger.Debug($"{data.Client} sent incompatible handshake");
			Logger.Debug($"{data.Client} sent {protocol} but we are on {Constants.ProtocolVersion}");
			Logger.Debug($"{data.Client} sent {data.Payload}");
			return;
		}

		data.Client.Engine      = data.Payload.ReadString() ?? string.Empty;
		data.Client.Platform    = data.Payload.ReadString() ?? string.Empty;
		data.Client.IsHandshake = true;

		var response = Buffer.New();
		var config   = Config.Load();

		response.Write(Constants.ProtocolVersion);
		response.Write(data.Client.Id);

		response.Write(data.Client.Remote.Address.GetAddressBytes());
		response.Write(data.Client.Remote.Port);

		var flags     = HandshakeFlags.None;
		var master    = MasterServer.GetMasterAddress();
		var hasMaster = !string.IsNullOrEmpty(master);

		if (!hasMaster) flags |= HandshakeFlags.IsOffline;
		response.Write(flags);
		if (hasMaster) response.Write(master);
		
		response.Write(Constants.MaxPacketSize);
		response.Write(config.GetConnectionTimeout());
		response.Write(config.GetKeepAliveInterval());

		Request.SendBuffer(data.Client, response, ResponseType.Handshake, data.Uid);

		Logger.Debug($"{data.Client} handshake");
	}
}