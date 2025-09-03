using Relay.Clients;
using Relay.Master;
using Relay.Packets;
using Relay.Priority;
using Relay.Utils;
using Buffer = Relay.Utils.Buffer;

namespace Relay.Requests.Auth;

public class AuthentificationHandler : Handler
{
	protected override void OnSetup()
	{
		PacketPriorityManager.SetMinimumPriority(RequestType.Authentification, EPriority.High);
		PacketDispatcher.RegisterHandler(RequestType.Authentification, OnAuthentification);
	}

	public static void OnAuthentification(PacketData data)
	{
		// Reject if not handshake, authenticating or authenticated
		if (!data.Client.IsHandshake || data.Client.IsAuthenticating || data.Client.IsAuthenticated)
			return;

		Logger.Debug($"{data.Client} starting authentification...");

		var flags = data.Payload.ReadEnum<AuthFlags>();
		if (flags.HasFlag(AuthFlags.UseGuest))
		{
			var response = Buffer.New();
			response.Write(AuthResult.Unknown);
			response.Write("This server does not support unauthenticated access.");
			Request.SendBuffer(data.Client, response, ResponseType.Authentification, data.Uid, EPriority.High);
			return;
		}

		var accessToken = data.Payload.ReadString();
		if (string.IsNullOrEmpty(accessToken))
		{
			var response = Buffer.New();
			response.Write(AuthResult.InvalidToken);
			response.Write("Access token is missing.");
			Request.SendBuffer(data.Client, response, ResponseType.Authentification, data.Uid, EPriority.High);
			return;
		}

		Task.Run(
			async () => await WorkerAuth(
				data.Client,
				data.Uid,
				accessToken,
				flags.HasFlag(AuthFlags.UseIntegrity) ? "integrity" : "bearer"
			)
		);
	}


	public static async Task WorkerAuth(Client client, ushort uid, string accessToken, string type)
	{
		client.User = new User { Id = User.InvalidId }; // Mark as authenticating
		var bearer = new AuthRequest
		{
			access_token = accessToken,
			token_type = type,
			ip = client.Remote.Address.ToString()
		};

		Logger.Debug($"{client} authentificating with {bearer.access_token} with {type}");
		var response = await MasterServer.Request<AuthResponse, AuthRequest>(
			"/api/relays/checkbearer",
			HttpMethod.Post, bearer
		);

		var buffer = Buffer.New();
		if (response.HasError())
		{
			Logger.Debug($"{client} authentification error {response.error}");
			buffer.Write(AuthResult.MasterError);
			buffer.Write(response.error.message);
			Request.SendBuffer(client, buffer, ResponseType.Authentification, uid, EPriority.High);
			client.User = null;
			return;
		}

		if (response.data.valid)
		{
			var user = new User
			{
				Id = response.data.user.id,
				DisplayName = response.data.user.display,
				Address = response.data.user.server
			};
			client.User = user;
			buffer.Write(AuthResult.Success);
			buffer.Write(user.Id);
			buffer.Write(user.Address);
			buffer.Write(user.DisplayName);
			Logger.Log($"{client} authentificated as {user.Id}@{user.Address}");
		}
		else if (response.data.is_blacklisted)
		{
			buffer.Write(AuthResult.Blacklisted);
			buffer.Write(response.data.blacklisted.ExpireAt);
			buffer.Write(response.data.blacklisted.reason);
			client.User = null;
			Logger.Debug($"{client} authentification error blacklisted {response.data.blacklisted.reason} for {response.data.user.id}@{response.data.user.server}");
		}
		else if (response.data.is_invalid_token)
		{
			buffer.Write(AuthResult.InvalidToken);
			client.User = null;
			Logger.Debug($"{client} authentification error invalid token");
		}
		else
		{
			buffer.Write(AuthResult.Unknown);
			buffer.Write("Unknown authentification error.");
			client.User = null;
			Logger.Debug($"{client} authentification error unknown");
		}

		Request.SendBuffer(client, buffer, ResponseType.Authentification, uid, EPriority.High);
	}
}