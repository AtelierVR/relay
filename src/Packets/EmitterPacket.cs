using Relay.Priority;
using Relay.Utils;

namespace Relay.Packets;

public class EmitterPacket(Remote remote, byte[] buffer, EPriority priority) : IComparable<EmitterPacket> {
	public readonly Remote    Remote    = remote;
	public readonly byte[]    Buffer    = buffer;
	public readonly EPriority Priority  = priority;
	public readonly DateTime  CreatedAt = DateTime.UtcNow;

	public int CompareTo(EmitterPacket? other) {
		if (other == null) return (int)EPriority.Normal;
		var priorityComparison = other.Priority.CompareTo(Priority);
		return priorityComparison == 0
			? CreatedAt.CompareTo(other.CreatedAt)
			: priorityComparison;
	}
}