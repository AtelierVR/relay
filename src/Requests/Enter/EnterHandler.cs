using Relay.Clients;
using Relay.Instances;
using Relay.Master;
using Relay.Packets;
using Relay.Players;
using Relay.Priority;
using Relay.Requests.Instances.Avatar;
using Relay.Requests.Instances.Transform;
using Relay.Utils;
using Buffer = Relay.Utils.Buffer;

namespace Relay.Requests.Instances.Enter;

public class EnterHandler : Handler
{
    protected override void OnSetup()
    {
        PacketPriorityManager.SetMinimumPriority(RequestType.Enter, EPriority.High);
        PacketDispatcher.RegisterHandler(RequestType.Enter, OnEnter);
    }

    public static void OnEnter(PacketData data)
    {
        var iid = data.Payload.ReadByte();

        Logger.Debug($"{data.Client} requests to enter instance {iid}");

        var instance = InstanceManager.Get(iid);
        if (instance == null)
        {
            SendResponse(data.Client, iid, data.Uid, EnterResult.NotFound, Messages.InstanceNotFound);
            return;
        }

        var player = instance.Players.Find(p => p.ClientId == data.Client.Id);
        if (player != null)
        {
            SendResponse(data.Client, iid, data.Uid, EnterResult.Unknown, Messages.AlreadyInInstance);
            return;
        }

        if (instance.IsFull())
        {
            SendResponse(data.Client, iid, data.Uid, EnterResult.Full, Messages.InstanceIsFull);
            return;
        }

        var moderated = data.Client.TryGetUser(out var user) ? instance.GetModerated(user) : null;
        if (moderated != null && moderated.IsBlacklisted(out var reason, out var expiresAt))
        {
            var buffer = Buffer.New();
            buffer.Write(iid);
            buffer.Write(EnterResult.Blacklisted);
            buffer.Write(reason ?? string.Empty);
            buffer.Write(expiresAt ?? DateTimeOffset.MinValue);
            Request.SendBuffer(data.Client, buffer, ResponseType.Enter, data.Uid);
            return;
        }

        if (instance.Flags.HasFlag(InstanceFlags.UseWhitelist) && (moderated == null || !moderated.IsWhitelisted()))
        {
            SendResponse(data.Client, iid, data.Uid, EnterResult.NotWhitelisted, Messages.YouAreNotWhitelisted);
            return;
        }

        var pFlags = PlayerFlags.None;
        string? display = null;
        var flags = data.Payload.ReadEnum<EnterFlags>();

        if (flags.HasFlag(EnterFlags.AsBot))
        {
            if (!instance.Flags.HasFlag(InstanceFlags.AuthorizeBot))
            {
                SendResponse(data.Client, iid, data.Uid, EnterResult.Refused, Messages.ConnectionInstanceAsBotRefused);
                return;
            }

            pFlags |= PlayerFlags.IsBot;
        }

        if (flags.HasFlag(EnterFlags.HideInList))
            pFlags |= PlayerFlags.HideInList;

        if (flags.HasFlag(EnterFlags.UsePseudonyme))
            display = data.Payload.ReadString();

        if (instance.Flags.HasFlag(InstanceFlags.UsePassword) && (!flags.HasFlag(EnterFlags.UsePassword) || !instance.VerifyPassword(data.Payload.ReadString() ?? string.Empty)))
        {
            SendResponse(data.Client, iid, data.Uid, EnterResult.IncorrectPassword, Messages.IncorrectPassword);
            return;
        }

        var nPlayer = new Player
        {
            Id = instance.GetNextPlayerId(),
            ClientId = data.Client.Id,
            InstanceId = instance.InternalId,
            Flags = pFlags,
            Status = PlayerStatus.Preparing,
            Display = string.IsNullOrWhiteSpace(display) ? string.Empty : display
        };

        instance.Players.Add(nPlayer);
        MasterServer.UpdateImmediately();

        var response = Buffer.New();
        response.Write(iid);
        response.Write(EnterResult.Success);
        response.Write(nPlayer.Flags);
        response.Write(nPlayer.Id);
        response.Write(data.Client.User?.Id ?? 0);
        response.Write(data.Client.User?.Address ?? string.Empty);
        response.Write(nPlayer.Display);
        response.Write(nPlayer.CreatedAt);
        response.Write(nPlayer.CustomTps == 0 ? instance.Tps : nPlayer.CustomTps);
        response.Write(nPlayer.CustomThreshold == 0 ? instance.Threshold : nPlayer.CustomThreshold);
        Request.SendBuffer(data.Client, response, ResponseType.Enter, data.Uid);

        foreach (var otherPlayer in instance.Players)
        {
            if (otherPlayer.ClientId == data.Client.Id) continue;
            SendJoin(data.Client, otherPlayer);
            SendJoin(otherPlayer.Client, nPlayer);
        }

        Logger.Log($"{nPlayer} entered {instance}");
    }

    public static void SendResponse(Client client, byte internalId, ushort uid, EnterResult result, string? reason = null)
    {
        var response = Buffer.New();
        response.Write(internalId);
        response.Write(result);
        if (reason != null)
            response.Write(reason);
        Request.SendBuffer(client, response, ResponseType.Enter, uid);
    }

    public static void SendJoin(Client client, Player player, ushort uid = ushort.MinValue)
    {
        var c = player.Client;

        var response = Buffer.New();
        response.Write(player.InstanceId);
        response.Write(player.Flags);
        response.Write(player.Id);
        response.Write(c.User?.Id ?? 0);
        response.Write(c.User?.Address ?? string.Empty);
        response.Write(player.Display);
        response.Write(player.CreatedAt);
        response.Write(c.Engine ?? string.Empty);
        response.Write(c.Platform ?? string.Empty);
        Request.SendBuffer(client, response, ResponseType.Join, uid);

        if (player.Avatar != null)
            AvatarChangedHandler.SendAvatarChanged(client, player.InstanceId, player.Id, player.Avatar);

        foreach (var (part, tr) in player.Transforms.GetPairs())
        {
            if (tr.flags == TransformFlags.None) continue;
            TransformHandler.SendTransform(client, player.InstanceId, player.Id, part, tr);
        }
    }
}