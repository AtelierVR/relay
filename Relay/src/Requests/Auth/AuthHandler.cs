using System;
using System.Net.Http;
using System.Threading;
using Relay.Clients;
using Relay.Master;
using Relay.Requests.Auth.Bearer;
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
        var serverAddress = buffer.ReadString();
        if (flags.HasFlag(AuthFlags.UseUnAuthentified))
        {
            var response = new Buffer();
            response.Write(AuthResult.Unknown);
            Request.SendBuffer(client, response, ResponseType.Authentification, uid);
            return;
        }

        var accessToken = buffer.ReadString();
        if (flags.HasFlag(AuthFlags.UseIntegrity))
        {
            var response = new Buffer();
            response.Write(AuthResult.Unknown);
            Request.SendBuffer(client, response, ResponseType.Authentification, uid);
        }
        else
        {
            var thread = new Thread(() => WorkerBearerAuth(client, uid, userId, accessToken));
            thread.Start();
        }
    }

    public static async void WorkerBearerAuth(Client client, ushort uid, uint userId, string accessToken)
    {
        var lastStatus = client.Status;
        var bearer = new AuthBearerRequest { access_token = accessToken, user_id = userId };
        client.Status = ClientStatus.Authentificating;
        Logger.Debug($"{client} authentificating with {bearer.access_token} as {bearer.user_id}");
        var response = await MasterServer.Request<AuthBearerResponse, AuthBearerRequest>(
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