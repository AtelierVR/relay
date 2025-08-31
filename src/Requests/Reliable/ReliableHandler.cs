using Relay.Packets;
using Relay.Priority;
using Relay.Utils;

namespace Relay.Requests.Reliable;

public class ReliableHandler : Handler {
	protected override void OnSetup() {
		PacketPriorityManager.SetMinimumPriority(RequestType.Reliable, EPriority.Critical);
		PacketDispatcher.RegisterHandler(RequestType.Reliable, OnReliable);
	}

	private static void OnReliable(PacketData data) {
		var cnt    = data.Payload.ReadByte();
		var remote = data.Client.Remote;
		for (var i = 0; i < cnt; i++) {
			var len = data.Payload.ReadUShort();
			Request.OnBuffer(remote, data.Payload.ReadBytes(len));
		}
	}
}