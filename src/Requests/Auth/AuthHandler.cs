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

        if (flags.HasFlag(AuthFlags.UseGuest))
        {
            var response = new Buffer();
            response.Write(AuthResult.Unknown);
            response.Write("This server does not support unauthenticated access.");
            Request.SendBuffer(client, response, ResponseType.Authentification, uid);
            return;
        }

        var accessToken = buffer.ReadString();
        Logger.Debug($"aa {flags} {flags.HasFlag(AuthFlags.UseIntegrity)}");

        var thread = new Thread(() => WorkerAuth(
            client, 
            uid,
            accessToken, 
            flags.HasFlag(AuthFlags.UseIntegrity) ? "integrity" : "bearer")
        );
        thread.Start();
    }

    public static async void WorkerAuth(Client client, ushort uid, string accessToken, string type)
    {
        var lastStatus = client.Status;
        var bearer = new AuthRequest
        {
            access_token = accessToken,
            token_type = type,
            ip = client.Remote.Address.ToString()
        };
        client.Status = ClientStatus.Authentificating;
        Logger.Debug($"{client} authentificating with {bearer.access_token} with {type}");
        var response = await MasterServer.Request<AuthResponse, AuthRequest>(
            "/api/relays/checkbearer",
            HttpMethod.Post, bearer
        );
        if (response.HasError())
        {
            var buffer = new Buffer();
            Logger.Debug($"{client} authentification error {response.error}");
            buffer.Write(AuthResult.MasterError);
            buffer.Write(response.error.message);
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
                buffer.Write(client.User.Address);
                buffer.Write(client.User.DisplayName);
                client.Status = ClientStatus.Authentificated;
                Logger.Log($"{client} authentificated as {client.User.Id}@{client.User.Address}");
            }
            else if (response.data.is_blacklisted)
            {
                buffer.Write(AuthResult.Blacklisted);
                buffer.Write(response.data.blacklisted.ExprireAt);
                buffer.Write(response.data.blacklisted.reason);
                client.Status = lastStatus;
                Logger.Debug($"{client} authentification error blacklisted {response.data.blacklisted.reason} for {response.data.user.id}@{response.data.user.server}");
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
                buffer.Write("Unknown authentification error.");
                client.Status = lastStatus;
                Logger.Debug($"{client} authentification error unknown");
            }

            Request.SendBuffer(client, buffer, ResponseType.Authentification, uid);
        }
    }
}