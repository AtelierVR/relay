using Relay.Packets;
using Relay.Utils;
using System.Collections.Concurrent;
using Relay.Priority;

namespace Relay.Requests.Segmentation;

public class SegmentationHandler : Handler
{
	private static readonly ConcurrentDictionary<int, SegmentedPacket> Segmented = new();
	private static readonly Timer CleanupTimer;

	static SegmentationHandler()
	{
		// Initialize cleanup timer to run every 10 seconds
		CleanupTimer = new Timer(CleanupExpiredPackets, null, TimeSpan.FromSeconds(10), TimeSpan.FromSeconds(10));
	}

	private static void CleanupExpiredPackets(object? state)
	{
		var config = Config.Load();
		var timeout = config.GetSegmentationTimeout();
		var expiredKeys = new List<int>();

		foreach (var kvp in Segmented)
			if (kvp.Value.IsExpired(timeout))
				expiredKeys.Add(kvp.Key);

		foreach (var key in expiredKeys)
			if (Segmented.TryRemove(key, out _))
				Logger.Warning($"Removed expired segmented packet with key {key}");
	}

	protected override void OnSetup()
	{
		PacketPriorityManager.SetMinimumPriority(RequestType.Segmentation, EPriority.Critical);
		PacketDispatcher.RegisterHandler(RequestType.Segmentation, OnSegmentation);
	}

	private static void OnSegmentation(PacketData data)
	{
		var sid = data.Payload.ReadUShort();
		var all = data.Payload.ReadUShort();
		var crt = data.Payload.ReadUShort();
		var spl = data.Payload.ToBuffer();

		var key = HashCode.Combine(sid, data.Client.Remote.GetHashCode());

		if (!Segmented.TryGetValue(key, out var packet))
		{
			packet = new SegmentedPacket(all);
			Segmented[key] = packet;
		}
		else
		{
			var config = Config.Load();
			var timeout = config.GetSegmentationTimeout();
			if (packet.IsExpired(timeout))
				return;
		}

		packet.AddSegment(crt, spl);

		if (!packet.IsComplete()) return;
		var merge = packet.Merge();
		Segmented.TryRemove(key, out _);
		Request.OnBuffer(data.Client.Remote, merge);
	}
}