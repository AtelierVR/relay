using Relay.Clients;
using Relay.Utils;

namespace Relay.Packets;

public static class PacketDispatcher {
	private static readonly Dictionary<RequestType, Action<PacketData>[]> Handlers = [];

	public static void RegisterHandler(RequestType type, Action<PacketData> handler) {
		if (!Handlers.ContainsKey(type))
			Handlers[type] = [];
		Handlers[type] = [.. Handlers[type], handler];
	}

	public static Action<PacketData>[] GetHandlers(RequestType type)
		=> Handlers.GetValueOrDefault(type, []);
}