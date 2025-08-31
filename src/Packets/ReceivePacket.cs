using Relay.Priority;
using Relay.Utils;

namespace Relay.Packets;

public class ReceivePacket(Remote remote, RequestType type, byte[] payload, EPriority priority) : IComparable<ReceivePacket> {
	public readonly Remote      Remote    = remote;
	public readonly RequestType Type      = type;
	public readonly byte[]      Payload   = payload;
	public readonly EPriority   Priority  = priority;
	public readonly DateTime    CreatedAt = DateTime.UtcNow;

	public int CompareTo(ReceivePacket? other) {
		if (other == null) return (int)EPriority.Normal;
		var priorityComparison = other.Priority.CompareTo(Priority);
		return priorityComparison == 0
			? CreatedAt.CompareTo(other.CreatedAt)
			: priorityComparison;
	}
}