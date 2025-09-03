using Relay.Clients;
using Relay.Instances;
using Relay.Master;
using Relay.Packets;
using Relay.Players;
using Relay.Priority;
using Relay.Utils;
using Buffer = Relay.Utils.Buffer;

namespace Relay.Requests.Instances.Traveling;

public class TravelingHandler : Handler
{
	protected override void OnSetup()
	{
		PacketPriorityManager.SetMinimumPriority(RequestType.Traveling, EPriority.High);
		PacketDispatcher.RegisterHandler(RequestType.Traveling, OnTraveling);
	}

	private static void OnTraveling(PacketData data)
	{
		var iid = data.Payload.ReadByte();
		var instance = InstanceManager.Get(iid);
		if (instance == null)
		{
			SendResponse(data.Client, iid, data.Uid, TravelingResults.Unknown, Messages.InstanceNotFound);
			return;
		}

		var player = instance.GetPlayers().FirstOrDefault(p => p.ClientId == data.Client.Id);
		if (player == null)
		{
			SendResponse(data.Client, iid, data.Uid, TravelingResults.Unknown, Messages.YouAreNotInInstance);
			return;
		}

		var action = data.Payload.ReadEnum<TravelingAction>();

		Logger.Debug($"{player} requests traveling action {action}");

		switch (action)
		{
			case TravelingAction.Travel:
				Travel(player, data.Uid);
				return;
			case TravelingAction.Ready:
				Ready(player, data.Uid);
				return;
			case TravelingAction.Failed:
				return;
			default:
				SendResponse(data.Client, iid, data.Uid, TravelingResults.Unknown, $"Action {action} is not supported yet");
				return;
		}
	}

	private static void SendResponse(Client client, byte internalId, ushort uid, TravelingResults result, string? reason = null)
	{
		var response = Buffer.New();
		response.Write(internalId);
		response.Write(result);
		if (reason != null)
			response.Write(reason);
		Request.SendBuffer(client, response, ResponseType.Traveling, uid, EPriority.High);
	}

	private static void Travel(Player player, ushort uid = 0)
	{
		var response = Buffer.New();
		response.Write(player.InstanceId);

		if (player.Status == PlayerStatus.None)
		{
			response.Write(TravelingResults.Unknown);
			response.Write("Player is not ready to travel");
			Request.SendBuffer(player.Client!, response, ResponseType.Traveling, uid, EPriority.High);
			return;
		}

		var world = player.Instance!.World;

		if (world == null)
		{
			response.Write(TravelingResults.Unknown);
			response.Write("World is not set for the instance");
			Request.SendBuffer(player.Client, response, ResponseType.Traveling, uid, EPriority.High);
			return;
		}

		response.Write(TravelingResults.UseMaster);
		response.Write(world.MasterId);
		response.Write(world.Address ?? string.Empty);
		response.Write(world.Version);

		Request.SendBuffer(player.Client!, response, ResponseType.Traveling, uid, EPriority.High);

		player.Status = PlayerStatus.Traveling;

		MasterServer.UpdateImmediately();
	}

	private static void Ready(Player player, ushort uid = 0)
	{
		var response = Buffer.New();
		response.Write(player.InstanceId);
		if (player.Status != PlayerStatus.Traveling)
		{
			response.Write(TravelingResults.Unknown);
			response.Write("Player is not traveling");
			Request.SendBuffer(player.Client, response, ResponseType.Traveling, uid, EPriority.High);
			return;
		}

		player.Status = PlayerStatus.Ready;

		response.Write(TravelingResults.Ready);
		Request.SendBuffer(player.Client, response, ResponseType.Traveling, uid, EPriority.High);
		MasterServer.UpdateImmediately();
	}
}