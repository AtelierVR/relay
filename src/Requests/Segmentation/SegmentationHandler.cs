using Relay.Packets;
using Relay.Utils;
using System.Collections.Concurrent;
using Relay.Priority;

namespace Relay.Requests.Segmentation;

public class SegmentationHandler : Handler {
	private static readonly ConcurrentDictionary<int, SegmentedPacket> Segmented = new();

	protected override void OnSetup() {
		PacketPriorityManager.SetMinimumPriority(RequestType.Segmentation, EPriority.Critical);
		PacketDispatcher.RegisterHandler(RequestType.Segmentation, OnSegmentation);
	}

	private static void OnSegmentation(PacketData data) {
		var sid = data.Payload.ReadUShort();
		var all = data.Payload.ReadUShort();
		var crt = data.Payload.ReadUShort();
		var spl = data.Payload.ToBuffer();

		var key = HashCode.Combine(sid, data.Client.Remote.GetHashCode());

		if (!Segmented.TryGetValue(key, out var packet)) {
			packet         = new SegmentedPacket(all);
			Segmented[key] = packet;
		}

		packet.AddSegment(crt, spl);

		if (!packet.IsComplete()) return;
		var merge = packet.Merge();
		Segmented.TryRemove(key, out _);
		Request.OnBuffer(data.Client.Remote, merge);
	}
}