using System;
using System.Net.Http;
using System.Threading;
using Relay.Clients;
using Relay.Master;
using Relay.Utils;
using Buffer = Relay.Utils.Buffer;

namespace Relay.Requests.Auth;

public class AuthHandler : Handler
{
    public override void OnReceive(Buffer buffer, Client client)
    {
        if (client.Status != ClientStatus.Handshaked) return;
        buffer.Goto(0);
        var length = buffer.ReadUShort();
        var uid = buffer.ReadUShort();
        var type = buffer.ReadEnum<RequestType>();
        if (type != RequestType.Authentification) return;
        Logger.Debug($"{client} sent authentification");
        var flags = buffer.ReadEnum<AuthFlags>();
        var userId = buffer.ReadUInt();
        if (flags.HasFlag(AuthFlags.UseUnAuthenticated))
        {
            var response = new Buffer();
            response.Write(AuthResult.Unknown);
            Request.SendBuffer(client, response, ResponseType.Authentification, uid);
            return;
        }

        var accessToken = buffer.ReadString();
        var thread = new Thread(() => WorkerAuth(
            client, 
            uid, 
            userId, 
            accessToken, 
            flags.HasFlag(AuthFlags.UseIntegrity) ? "integrity" : "bearer")
        );
        thread.Start();
    }

    public static async void WorkerAuth(Client client, ushort uid, uint userId, string accessToken, string type)
    {
        var lastStatus = client.Status;
        var bearer = new AuthRequest
        {
            access_token = accessToken,
            user_id = userId,
            token_type = type,
            ip = client.Remote.Address.ToString()
        };
        client.Status = ClientStatus.Authentificating;
        Logger.Debug($"{client} authentificating with {bearer.access_token} as {bearer.user_id}");
        var response = await MasterServer.Request<AuthResponse, AuthRequest>(
            "/api/relays/checkbearer",
            HttpMethod.Post, bearer
        );
        if (response.HasError())
        {
            var buffer = new Buffer();
            Logger.Debug($"{client} authentification error {response.error}");
            buffer.Write(AuthResult.CannotContactMasterServer);
            Request.SendBuffer(client, buffer, ResponseType.Authentification, uid);
            client.Status = lastStatus;
        }
        else
        {
            var buffer = new Buffer();
            if (response.data.valid)
            {
                client.User = new User
                {
                    Id = response.data.user.id,
                    DisplayName = response.data.user.display,
                    Address = response.data.user.server
                };
                buffer.Write(AuthResult.Success);
                buffer.Write(client.User.Id);
                buffer.Write(client.User.DisplayName);
                buffer.Write(client.User.Address);
                client.Status = ClientStatus.Authentificated;
                Logger.Log($"{client} authentificated as {client.User.Id}@{client.User.Address}");
            }
            else if (response.data.is_blacklisted)
            {
                buffer.Write(AuthResult.Blacklisted);
                buffer.Write(response.data.blacklisted.id);
                buffer.Write((DateTimeOffset.Now - response.data.blacklisted.ExprireAt).TotalMinutes);
                client.Status = lastStatus;
                Logger.Debug($"{client} authentification error blacklisted {response.data.blacklisted.id}");
            }
            else if (response.data.is_invalid_token)
            {
                buffer.Write(AuthResult.InvalidToken);
                client.Status = lastStatus;
                Logger.Debug($"{client} authentification error invalid token");
            }
            else
            {
                buffer.Write(AuthResult.Unknown);
                client.Status = lastStatus;
                Logger.Debug($"{client} authentification error unknown");
            }

            Request.SendBuffer(client, buffer, ResponseType.Authentification, uid);
        }
    }
}