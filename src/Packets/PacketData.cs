using Relay.Clients;
using Relay.Utils;

namespace Relay.Packets;

public struct PacketData(ushort length, ushort uid, RequestType type, Utils.Buffer payload, Client client) {
	public readonly ushort       Length  = length;
	public readonly ushort       Uid     = uid;
	public readonly RequestType  Type    = type;
	public readonly Utils.Buffer Payload = payload;
	public readonly Client       Client  = client;
}