namespace Relay.Requests.Segmentation;

public class SegmentedPacket(ushort total) {
	private readonly Dictionary<ushort, byte[]> _segments = new();
	private DateTime _lastUpdated = DateTime.UtcNow;

	public void AddSegment(ushort segmentIndex, byte[] data) {
		_segments[segmentIndex] = data;
		_lastUpdated = DateTime.UtcNow;
	}

	public bool IsExpired(int timeout)
		=> DateTime.UtcNow - _lastUpdated > TimeSpan.FromSeconds(timeout);

	public bool IsComplete()
		=> _segments.Count == total;

	public byte[] Merge() {
		var totalSize = 0;
		for (ushort i = 0; i < total; i++)
			if (_segments.TryGetValue(i, out var segment))
				totalSize += segment.Length;
		var buffer = Utils.Buffer.New(Math.Min(totalSize, Utils.Buffer.DefaultLength));
		for (ushort i = 0; i < total; i++)
			if (_segments.TryGetValue(i, out var segment))
				buffer.Write(segment);
		return buffer.ToBuffer();
	}
}