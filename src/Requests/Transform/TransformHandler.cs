using Relay.Clients;
using Relay.Instances;
using Relay.Packets;
using Relay.Players;
using Relay.Priority;
using Relay.Utils;
using Buffer = Relay.Utils.Buffer;

namespace Relay.Requests.Instances.Transform;

internal class TransformHandler : Handler
{
	protected override void OnSetup()
	{
		PacketPriorityManager.SetMinimumPriority(RequestType.Transform, EPriority.Low);
		PacketDispatcher.RegisterHandler(RequestType.Transform, OnTransform);
	}

	private static void OnTransform(PacketData data)
	{
		var iid = data.Payload.ReadByte();
		var instance = InstanceManager.Get(iid);

		if (instance == null)
		{
			Logger.Debug($"{data.Client} sent transform for invalid instance {iid}");
			return;
		}

		var trType = data.Payload.ReadEnum<TransformType>();

		switch (trType)
		{
			case TransformType.OnPlayer:
				OnPlayer(data, instance);
				break;
			case TransformType.ByPath:
				// var bpPath    = data.Payload.ReadString();
				// var bpTrFlags = data.Payload.ReadEnum<TransformFlags>();
				Logger.Debug("TransformType.ByPath is not implemented yet");
				break;
			case TransformType.OnObject:
				// var ooObjId   = data.Payload.ReadUShort();
				// var ooTrFlags = data.Payload.ReadEnum<TransformFlags>();
				Logger.Debug("TransformType.OnObject is not implemented yet");
				break;
			default:
				Logger.Debug($"{data.Client} sent unknown transform type {trType}");
				break;
		}
	}

	private static void OnPlayer(PacketData data, Instance instance)
	{
		var players = instance.GetPlayers();

		var player = players.FirstOrDefault(p => p.ClientId == data.Client.Id);
		if (player == null || !player.IsReady())
		{
			Logger.Debug($"{data.Client} sent transform for not ready player {player?.Id}");
			return;
		}

		var pid = data.Payload.ReadUShort();
		Player? opPlayer;

		if (pid != ushort.MaxValue && pid != player.Id)
		{
			opPlayer = players.FirstOrDefault(p => p.Id == pid);
			if (opPlayer == null || !opPlayer.IsReady())
			{
				Logger.Debug($"{data.Client} sent transform for not ready operation player {opPlayer?.Id} {opPlayer?.Status}");
				return;
			}
			if (!player.IsAllowed(opPlayer))
			{
				Logger.Debug($"{data.Client} sent transform for not allowed player {opPlayer?.Id}");
				return;
			}
		}
		else opPlayer = player;

		var opPlayerRig = data.Payload.ReadUShort();
		var opTransform = opPlayer.Transforms.Get(opPlayerRig) ?? new Utils.Transform();

		var opTrFlags = data.Payload.ReadEnum<TransformFlags>();
		if (opTrFlags.HasFlag(TransformFlags.Reset))
			opTransform = new Utils.Transform();
		opTransform.flags |= opTrFlags;

		if (opTrFlags.HasFlag(TransformFlags.Position))
			opTransform.position = data.Payload.ReadVector3();
		if (opTrFlags.HasFlag(TransformFlags.Rotation))
			opTransform.rotation = data.Payload.ReadQuaternion();
		if (opTrFlags.HasFlag(TransformFlags.Scale))
			opTransform.scale = data.Payload.ReadVector3();
		if (opTrFlags.HasFlag(TransformFlags.Velocity))
			opTransform.velocity = data.Payload.ReadVector3();
		if (opTrFlags.HasFlag(TransformFlags.AngularVelocity))
			opTransform.angularVelocity = data.Payload.ReadVector3();

		foreach (var other in players)
		{
			if (other.ClientId == data.Client.Id) continue;
			SendTransform(other.Client, player.InstanceId, opPlayer.Id, opPlayerRig, opTransform);
		}

		opTransform.flags &= ~TransformFlags.Reset;
		opPlayer.Transforms.Set(opPlayerRig, opTransform);
	}

	public static void SendTransform(Client client, byte instanceId, ushort playerId, ushort playerRig, Utils.Transform transform, ushort uid = ushort.MinValue)
	{
		var response = Buffer.New();
		response.Write(instanceId);
		response.Write(TransformType.OnPlayer);
		response.Write(playerId);
		response.Write(playerRig);
		response.Write(transform.flags);
		if (transform.flags.HasFlag(TransformFlags.Position))
			response.Write(transform.position);
		if (transform.flags.HasFlag(TransformFlags.Rotation))
			response.Write(transform.rotation);
		if (transform.flags.HasFlag(TransformFlags.Scale))
			response.Write(transform.scale);
		if (transform.flags.HasFlag(TransformFlags.Velocity))
			response.Write(transform.velocity);
		if (transform.flags.HasFlag(TransformFlags.AngularVelocity))
			response.Write(transform.angularVelocity);
		Request.SendBuffer(client, response, ResponseType.Transform, uid, EPriority.Low);
	}
}