using System;
using System.Collections.Generic;
using Relay.Clients;
using Relay.Instances;
using Relay.Packets;
using Relay.Players;
using Relay.Priority;
using Relay.Utils;
using Buffer = Relay.Utils.Buffer;

namespace Relay.Requests.Instances.ServerConfig;

public class ServerConfigHandler : Handler {
	protected override void OnSetup() {
		PacketPriorityManager.SetMinimumPriority(RequestType.ServerConfig, EPriority.Normal);
		PacketDispatcher.RegisterHandler(RequestType.ServerConfig, OnEnter);
	}

	private static void OnEnter(PacketData data) {
		var iid      = data.Payload.ReadByte();
		var instance = InstanceManager.Get(iid);
		if (instance == null) {
			SendResponse(data.Client, iid, data.Uid, ServerConfigResult.Failure, Messages.InstanceNotFound);
			return;
		}

		var player = instance.Players.Find(p => p.ClientId == data.Client.Id);
		if (player == null) {
			SendResponse(data.Client, iid, data.Uid, ServerConfigResult.Failure, Messages.YouAreNotInInstance);
			return;
		}

		if (!player.HasHighPrivilege) {
			SendResponse(data.Client, iid, data.Uid, ServerConfigResult.Failure, "Permission denied");
			return;
		}

		var flags = data.Payload.ReadEnum<ServerConfigFlags>();
		if (flags == ServerConfigFlags.None) {
			SendConfigUpdate(instance, player, ServerConfigFlags.All, data.Uid);
			return;
		}

		var result = ServerConfigFlags.None;

		if (flags.HasFlag(ServerConfigFlags.Tps)) {
			var newTps = data.Payload.ReadByte();
			instance.Tps =  newTps;
			result       |= ServerConfigFlags.Tps;
		}

		if (flags.HasFlag(ServerConfigFlags.Threshold)) {
			var newThreshold = data.Payload.ReadFloat();
			instance.Threshold =  newThreshold;
			result             |= ServerConfigFlags.Threshold;
		}

		if (flags.HasFlag(ServerConfigFlags.Capacity)) {
			var newCapacity = data.Payload.ReadUShort();
			instance.Capacity =  newCapacity;
			result            |= ServerConfigFlags.Capacity;
		}

		if (flags.HasFlag(ServerConfigFlags.Password)) {
			var newPassword = data.Payload.ReadString();
			instance.Password =  newPassword ?? string.Empty;
			result            |= ServerConfigFlags.Password;
		}

		if (flags.HasFlag(ServerConfigFlags.Flags)) {
			var newFlags = data.Payload.ReadEnum<InstanceFlags>();
			instance.Flags =  newFlags;
			result         |= ServerConfigFlags.Flags;
		}

		if (result == ServerConfigFlags.None) {
			SendResponse(data.Client, iid, data.Uid, ServerConfigResult.Failure, "No valid configuration flags provided");
			return;
		}

		Logger.Log($"{player} updated instance config: {result}");

		foreach (var other in instance.Players.Where(o => o.Client!.Id != data.Client.Id))
			SendConfigUpdate(instance, other, result);
		SendConfigUpdate(instance, player, result, data.Uid);
	}

	private static void SendResponse(Client client, byte iid, ushort uid, ServerConfigResult type, string? reason = null) {
		var buffer = Buffer.New();
		buffer.Write(iid);
		buffer.Write(type);
		if (!string.IsNullOrEmpty(reason))
			buffer.Write(reason);
		Request.SendBuffer(client, buffer, ResponseType.ServerConfig, uid);
	}

	public static void BroadcastConfigUpdate(Instance instance, ServerConfigFlags configFlags, ushort uid = 0) {
		foreach (var player in instance.Players) 
			SendConfigUpdate(instance, player, configFlags, uid);
	}

	private static void SendConfigUpdate(Instance instance, Player player, ServerConfigFlags configFlags, ushort uid = 0) {
		var response = Buffer.New();
		response.Write(instance.InternalId);
		response.Write(ServerConfigResult.Change);
		response.Write(configFlags);

		// Écrire les valeurs pour chaque flag activé
		if (configFlags.HasFlag(ServerConfigFlags.Tps))
			response.Write(player.CustomTps == 0 ? instance.Tps : player.CustomTps);

		if (configFlags.HasFlag(ServerConfigFlags.Threshold))
			response.Write(player.CustomThreshold == 0 ? instance.Threshold : player.CustomThreshold);

		if (configFlags.HasFlag(ServerConfigFlags.Capacity))
			response.Write(instance.Capacity);

		if (configFlags.HasFlag(ServerConfigFlags.Flags))
			response.Write(instance.Flags);

		if (configFlags.HasFlag(ServerConfigFlags.Password))
			response.Write((byte)1);

		Request.SendBuffer(player.Client!, response, ResponseType.ServerConfig, uid);
	}
}