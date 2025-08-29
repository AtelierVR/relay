using Relay.Clients;
using Relay.Players;
using Relay.Utils;
using Buffer = Relay.Utils.Buffer;

namespace Relay.Requests.Instances.Transform;

internal class TransformHandler : Handler
{
    public override void OnReceive(Buffer buffer, Client client)
    {
        if (client.Status != ClientStatus.Authentificated) return;
        buffer.Goto(0);
        var length = buffer.ReadUShort();
        var uid = buffer.ReadUShort();
        var type = buffer.ReadEnum<RequestType>();
        if (type != RequestType.Transform) return;
        var internalId = buffer.ReadByte();
        var player = client.GetInstancePlayer(internalId);
        if (player is not { Status: PlayerStatus.Ready }) return;

        var tr_type = buffer.ReadEnum<TransformType>();

        Logger.Debug($"{client} sent transform {tr_type}");
        switch (tr_type)
        {
            case TransformType.OnPlayer:
                var op_player_id = buffer.ReadUShort();
                var op_player = client.GetInstancePlayer(internalId);

                if (op_player is not { Status: PlayerStatus.Ready }) return;
                var isSelf = op_player.Client == client;

                if (!isSelf && !player.HasPrivilege)
                {
                    // only moderators can transform other players
                    Logger.Warning($"{player} tried to transform {op_player}");
                    return;
                }
                var op_player_rig = buffer.ReadUShort();
                var op_transform = op_player.Transforms.Get(op_player_rig) ?? new Utils.Transform();

                var op_tr_flags = buffer.ReadEnum<TransformFlags>();
                if (op_tr_flags.HasFlag(TransformFlags.Reset))
                    op_transform = new Utils.Transform();
                op_transform.flags |= op_tr_flags;

                if (op_tr_flags.HasFlag(TransformFlags.Position))
                    op_transform.position = buffer.ReadVector3();
                if (op_tr_flags.HasFlag(TransformFlags.Rotation))
                    op_transform.rotation = buffer.ReadQuaternion();
                if (op_tr_flags.HasFlag(TransformFlags.Scale))
                    op_transform.scale = buffer.ReadVector3();
                if (op_tr_flags.HasFlag(TransformFlags.Scale))
                    op_transform.scale = buffer.ReadVector3();
                if (op_tr_flags.HasFlag(TransformFlags.Velocity))
                    op_transform.velocity = buffer.ReadVector3();
                if (op_tr_flags.HasFlag(TransformFlags.AngularVelocity))
                    op_transform.angularVelocity = buffer.ReadVector3();

                // send to all players in the instance except the sender
                foreach (var other in op_player.Instance.Players)
                    if (other.Client != client)
                        SendTransform(other.Client, internalId, op_player_id, op_player_rig, op_transform);

                op_transform.flags &= ~TransformFlags.Reset;
                op_player.Transforms.Set(op_player_rig, op_transform);

                Logger.Debug($"{player} transformed {op_player} to {(PlayerRig)op_player_rig}");
                break;
            case TransformType.ByPath:
                var bp_path = buffer.ReadString();
                var bp_tr_flags = buffer.ReadEnum<TransformFlags>();
                break;
            case TransformType.OnObject:
                var oo_obj_id = buffer.ReadUShort();
                var oo_tr_flags = buffer.ReadEnum<TransformFlags>();
                break;
        }
    }

    public static void SendTransform(Client client, byte instanceId, ushort playerId, ushort playerRig, Utils.Transform transform, ushort uid = ushort.MinValue)
    {
        var response = new Buffer();
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
        Request.SendBuffer(client, response, ResponseType.Transform, uid);
    }
}