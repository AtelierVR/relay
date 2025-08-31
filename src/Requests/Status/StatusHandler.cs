using Relay.Clients;
using Relay.Instances;
using Relay.Master;
using Relay.Packets;
using Relay.Priority;
using Relay.Utils;
using Buffer = Relay.Utils.Buffer;

namespace Relay.Requests.Status;

public class StatusHandler : Handler {
	protected override void OnSetup() {
		PacketPriorityManager.SetMinimumPriority(RequestType.Status, EPriority.Normal);
		PacketDispatcher.RegisterHandler(RequestType.Status, OnStatus);
	}

	private static void OnStatus(PacketData data) {
		// Reject if not handshake
		if (!data.Client.IsHandshake) return;

		var pagination = data.Payload.ReadByte();
        Logger.Debug($"{data.Client} requested status page {pagination}");

		var response  = Buffer.New();
		var pages = GetPages(Constants.MaxPacketSize - 2);
		if (pagination >= pages.Length)
			response.Write((byte)0);
		else response.Write(pages[pagination]);

		response.Write(pagination);
		response.Write((byte)pages.Length);

        Logger.Debug($"{response}");

		Request.SendBuffer(data.Client, response, ResponseType.Status, data.Uid);
	}


	private static Buffer[] GetPages(ushort bytesPerPage) {
		var liBuffer = new List<Buffer>();
		
		var buffer   = Buffer.New();
		buffer.Skip(1);
		
		byte nb = 0;
		foreach (var instance in InstanceManager.Instances) {
			var instanceBuffer = Buffer.New();
			instanceBuffer.Write(instance.Flags);
			instanceBuffer.Write(instance.InternalId);
			instanceBuffer.Write(instance.MasterId);
			instanceBuffer.Write((ushort)instance.Players.Count);
			instanceBuffer.Write(instance.Capacity);

			if (buffer.Length + instanceBuffer.Length > bytesPerPage || nb >= byte.MaxValue) {
				buffer.Goto(0);
				buffer.Write(nb);
				liBuffer.Add(buffer);
				buffer = Buffer.New();
				buffer.Skip(1);
				nb = 0;
			}

			buffer.Write(instanceBuffer);
			nb++;
		}

		if (nb <= 0)
			return [.. liBuffer];

		buffer.Goto(0);
		buffer.Write(nb);
		liBuffer.Add(buffer);


		return [.. liBuffer];
	}
}