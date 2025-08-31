using Relay.Clients;
using Relay.Instances;
using Relay.Packets;
using Relay.Players;
using Relay.Priority;
using Relay.Utils;
using Buffer = Relay.Utils.Buffer;

namespace Relay.Requests.Instances.Transform;

internal class TransformHandler : Handler {
	protected override void OnSetup() {
		PacketPriorityManager.SetMinimumPriority(RequestType.Transform, EPriority.Low);
		PacketDispatcher.RegisterHandler(RequestType.Transform, OnAuthentification);
	}

	private static void OnAuthentification(PacketData data) {
		var iid      = data.Payload.ReadByte();
		var instance = InstanceManager.Get(iid);
		if (instance == null) return;

		var player = instance.Players.Find(p => p.ClientId == data.Client.Id);
		if (player == null || !player.IsReady()) return;

		var     pid = data.Payload.ReadUShort();
		Player? opPlayer;

		if (pid != ushort.MaxValue) {
			opPlayer = instance.Players.FirstOrDefault(p => p.Id == pid);
			if (opPlayer == null || !opPlayer.IsReady()) return;
			if (!player.IsAllowed(opPlayer)) return;
		} else opPlayer = player;

		var trType = data.Payload.ReadEnum<TransformType>();

		switch (trType) {
			case TransformType.OnPlayer:
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

				foreach (var other in instance.Players.Where(other => other.Client != data.Client))
					SendTransform(other.Client!, player.InstanceId, opPlayer.Id, opPlayerRig, opTransform);

				opTransform.flags &= ~TransformFlags.Reset;
				opPlayer.Transforms.Set(opPlayerRig, opTransform);

				break;
			default:
			case TransformType.ByPath:
				// var bpPath    = data.Payload.ReadString();
				// var bpTrFlags = data.Payload.ReadEnum<TransformFlags>();
				break;
			case TransformType.OnObject:
				// var ooObjId   = data.Payload.ReadUShort();
				// var ooTrFlags = data.Payload.ReadEnum<TransformFlags>();
				break;
		}
	}

	public static void SendTransform(Client client, byte instanceId, ushort playerId, ushort playerRig, Utils.Transform transform, ushort uid = ushort.MinValue) {
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