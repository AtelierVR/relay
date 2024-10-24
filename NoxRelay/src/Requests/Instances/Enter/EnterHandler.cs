using System;
using System.Net.Http;
using System.Threading;
using Relay.Clients;
using Relay.Instances;
using Relay.Master;
using Relay.Players;
using Relay.Utils;
using Buffer = Relay.Utils.Buffer;

namespace Relay.Requests.Instances.Enter;

public class EnterHandler : Handler
{
    public override void OnReceive(Buffer buffer, Client client)
    {
        if (client.Status != ClientStatus.Authentificated) return;
        buffer.Goto(0);
        var length = buffer.ReadUShort();
        var uid = buffer.ReadUShort();
        var type = buffer.ReadEnum<RequestType>();
        if (type != RequestType.Enter) return;
        Logger.Debug($"{client} sent enter");
        var instanceId = buffer.ReadUShort();
        var player = PlayerManager.GetFromClientInstance(client.Id, instanceId);
        var instance = player?.Instance ?? InstanceManager.Get(instanceId);

        var response = new Buffer();
        response.Write(instanceId);
        if (instance == null)
        {
            response.Write(EnterResult.NotFound);
            Request.SendBuffer(client, response, ResponseType.Enter, uid);
            return;
        }

        if (player != null && player.Status != PlayerStatus.None)
        {
            response.Write(EnterResult.Unknown);
            Request.SendBuffer(client, response, ResponseType.Enter, uid);
            return;
        }

        if (instance.Capacity <= instance.Players.Count && !instance.Flags.HasFlag(InstanceFlags.AllowOverload) || instance.Players.Count >= ushort.MaxValue)
        {
            response.Write(EnterResult.Full);
            Request.SendBuffer(client, response, ResponseType.Enter, uid);
            return;
        }

        var modered = instance.GetModered(client.User);
        if (modered is { IsBlacklisted: true })
        {
            response.Write(EnterResult.Blacklisted);
            response.Write(modered.BlacklistReason);
            response.Write((uint)modered.BlacklistExpiresAt.ToUnixTimeSeconds());
            Request.SendBuffer(client, response, ResponseType.Enter, uid);
            return;
        }

        if (instance.Flags.HasFlag(InstanceFlags.UseWhitelist) && modered is not { IsWhitelisted: true })
        {
            response.Write(EnterResult.NotWhitelisted);
            Request.SendBuffer(client, response, ResponseType.Enter, uid);
            return;
        }

        var pFlags = PlayerFlags.None;
        string? display = null;

        var flags = buffer.ReadEnum<EnterFlags>();
        if (flags.HasFlag(EnterFlags.AsBot))
        {
            if (!instance.Flags.HasFlag(InstanceFlags.AuthorizeBot))
            {
                response.Write(EnterResult.Refused);
                response.Write(Messages.ConnectionInstanceAsBotRefused);
                Request.SendBuffer(client, response, ResponseType.Enter, uid);
                return;
            }

            pFlags |= PlayerFlags.IsBot;
        }

        if (flags.HasFlag(EnterFlags.HideInList))
            pFlags |= PlayerFlags.HideInList;

        if (flags.HasFlag(EnterFlags.UsePseudonyme))
        {
            display = buffer.ReadString();
            if (string.IsNullOrWhiteSpace(display) || display.Length < 3 || display.Length > 32)
            {
                response.Write(EnterResult.InvalidPseudonyme);
                Request.SendBuffer(client, response, ResponseType.Enter, uid);
                return;
            }
        }

        if (instance.Flags.HasFlag(InstanceFlags.UsePassword)
            && (!flags.HasFlag(EnterFlags.UsePassword) || !instance.VerifyPassword(buffer.ReadString() ?? "")))
        {
            response.Write(EnterResult.IncorrectPassword);
            Request.SendBuffer(client, response, ResponseType.Enter, uid);
            return;
        }

        player = new Player
        {
            ClientId = client.Id,
            InstanceId = instanceId,
            Flags = pFlags
        };
        player.Display = display ?? client.User.DisplayName;
        player.Status = PlayerStatus.Configuration;

        response.Write(EnterResult.Success);
        response.Write(player.Flags);
        response.Write(player.Id);
        response.Write(player.Display);
        response.Write(player.CreatedAt);
        response.Write(player.Instance.MaxTps);
        Logger.Log($"{player} entered {instance}");
        Request.SendBuffer(client, response, ResponseType.Enter, uid);
    }

    public static void SendJoin(Client client, Player player, ushort uid = ushort.MinValue)
    {
        var response = new Buffer();
        response.Write(player.InstanceId);
        response.Write(player.Flags);
        response.Write(player.Id);
        response.Write(player.Client.User?.Id ?? uint.MinValue);
        response.Write(player.Display);
        response.Write(player.CreatedAt);
        response.Write(player.Client.Engine);
        response.Write(player.Client.Platform);
        Request.SendBuffer(client, response, ResponseType.Join, uid);
    }
}