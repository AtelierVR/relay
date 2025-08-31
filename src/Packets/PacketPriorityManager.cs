using Relay.Priority;
using Relay.Utils;

namespace Relay.Packets;

public static class PacketPriorityManager {
	private static readonly Dictionary<RequestType, EPriority> PriorityMap = [];

	public static EPriority GetPriority(RequestType requestType)
		=> PriorityMap.GetValueOrDefault(requestType, EPriority.Normal);

	public static void SetPriority(RequestType requestType, EPriority priority)
		=> PriorityMap[requestType] = priority;

	public static void SetMinimumPriority(RequestType requestType, EPriority priority) {
		var crt = GetPriority(requestType);
		if (crt > priority) SetPriority(requestType, priority);
	}
}