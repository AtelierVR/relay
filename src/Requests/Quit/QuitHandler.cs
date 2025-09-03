using Relay.Clients;
using Relay.Instances;
using Relay.Master;
using Relay.Packets;
using Relay.Players;
using Relay.Priority;
using Relay.Utils;
using Buffer = Relay.Utils.Buffer;

namespace Relay.Requests.Instances.Quit
{
	public class QuitHandler : Handler
	{
		protected override void OnSetup()
		{
			PacketPriorityManager.SetMinimumPriority(RequestType.Quit, EPriority.Critical);
			PacketDispatcher.RegisterHandler(RequestType.Quit, OnQuit);
		}

		private static void OnQuit(PacketData data)
		{
			var iid = data.Payload.ReadByte();
			var instance = InstanceManager.Get(iid);
			if (instance == null)
			{
				SendResponse(data.Client, iid, data.Uid, QuitType.UnknownError, Messages.InstanceNotFound);
				return;
			}

			var player = instance.GetPlayers().FirstOrDefault(p => p.ClientId == data.Client.Id);
			if (player == null)
			{
				SendResponse(data.Client, iid, data.Uid, QuitType.UnknownError, Messages.YouAreNotInInstance);
				return;
			}

			var action = data.Payload.ReadEnum<QuitType>();
			var reason = data.Payload.Remaining() > 2 ? data.Payload.ReadString() : null;

			player.Leave(action, reason, data.Uid);
		}

		private static void SendResponse(Client client, byte iid, ushort uid, QuitType type, string? reason = null)
		{
			var buffer = Buffer.New();
			buffer.Write(iid);
			buffer.Write(type);
			if (!string.IsNullOrEmpty(reason))
				buffer.Write(reason);
			Request.SendBuffer(client, buffer, ResponseType.Quit, uid, EPriority.Critical);
		}


		public static void LeavePlayer(Player player, QuitType type, string? reason = null, Player? by = null, ushort uid = 0)
		{
			if (player.Status == PlayerStatus.None) return;
			switch (type)
			{
				case QuitType.ModerationKick when by?.GetModerated() is null || !by.GetModerated()!.IsModerator():
				case QuitType.VoteKick when by != null:
				case QuitType.Timeout when by != player:
				case QuitType.Normal when by != player:
				case QuitType.ConfigurationError:
				case QuitType.UnknownError:
					return;
			}

			var ins = player.Instance;

			Logger.Log($"{player} left the instance");

			Buffer response;

			if (player.Status == PlayerStatus.Ready)
				foreach (var other in ins.GetPlayers().Where(other => other.Id != player.Id))
				{
					if (other is not { Status: PlayerStatus.Ready }) continue;
					var allMessage = other.HasPrivilege
						|| by is not null && by.Id == other.Id;

					// broadcast leave event to other players of the player
					response = Buffer.New();
					response.Write(player.InstanceId);
					response.Write(allMessage ? QuitType.Normal : type);
					response.Write(player.Id);
					if (allMessage && (type & QuitType.ModerationAction) != 0)
					{
						response.Write(by?.Id ?? ushort.MaxValue);
						if (!string.IsNullOrEmpty(reason))
							response.Write(reason);
					}

					Request.SendBuffer(other.Client, response, ResponseType.Leave, priority: EPriority.High);

					// send leave event to player of all other players
					response = Buffer.New();
					response.Write(player.InstanceId);
					response.Write(QuitType.Normal);
					response.Write(other.Id);
					Request.SendBuffer(player.Client, response, ResponseType.Leave, priority: EPriority.High);
				}

			response = Buffer.New();
			response.Write(player.InstanceId);
			response.Write(type);
			if (!string.IsNullOrEmpty(reason))
				response.Write(reason);
			Request.SendBuffer(player.Client, response, ResponseType.Quit, uid, EPriority.Critical);
		}
	}
}