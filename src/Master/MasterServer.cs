using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using Relay.Clients;
using Relay.Instances;
using Relay.Master.Update;
using Relay.Utils;

namespace Relay.Master
{
	public static class MasterServer
	{
		private static bool _isConnected;
		private static DateTime _lastUpdate = DateTime.Now;
		private static CancellationTokenSource? _ct;

		public static void UpdateImmediately()
			=> _lastUpdate = DateTime.MinValue;

		public static string GetMasterAddress()
			=> Config.Load().GetMasterGateway();

		public static string GetUsedAddress()
		=> Config.Load().GetUsedAddress();

		public static string GetToken()
			=> Config.Load().GetToken();

		public static byte GetMaxInstances()
			=> Config.Load().GetMaxInstances();

		private static CancellationTokenSource? _cts;

		public static void Start()
		{
			Logger.Debug("[MasterServer] Starting handler...");

			_cts = new CancellationTokenSource();
			var token = _cts.Token;

			Task.Run(
				async () =>
				{
					while (!token.IsCancellationRequested)
					{
						if (DateTime.UtcNow - _lastUpdate < TimeSpan.FromMilliseconds(Constants.DefaultUpdateTime))
						{
							await Task.Delay(Constants.MinUpdateTime, _cts.Token);
							continue;
						}

						_lastUpdate = DateTime.UtcNow;

						await SendUpdate();
						await Task.Delay(1, _cts.Token);
					}

					Logger.Debug("[Request] Stopped emitting.");
				}, token
			);

			UpdateImmediately();
		}

		public static void Stop()
		{
			_cts?.Cancel();
			_cts?.Dispose();
			_cts = null;
		}


		private static async Task SendUpdate()
		{
			var clients = ClientManager.Clients.ToArray()
				.Select(
					client => new RequestClient
					{
						id = client.Id,
						remote = client.Remote.Address + ":" + client.Remote.Port,
						platform = client.Platform.ToString().ToLower(),
						engine = client.Engine.ToString().ToLower(),
						last_seen = (ulong)client.LastSeen.ToUnixTimeMilliseconds(),
						user = client.IsAuthenticated
							? new RequestUser
							{
								id = client.User!.Value.Id,
								address = client.User!.Value.Address
							}
							: null,
					}
				)
				.ToArray();

			var instances = InstanceManager.Instances.ToArray()
				.Select(
					instance => new RequestInstance
					{
						internal_id = instance.InternalId,
						master_id = instance.MasterId,
						flags = (uint)instance.Flags,
						max_players = instance.Capacity,
						players = [.. instance.GetPlayers()
							.Select(
								player => new RequestPlayer
								{
									id = player.Id,
									client_id = player.Client.Id,
									display = player.Display,
									flags = (uint)player.Flags,
									status = (byte)player.Status,
									created_at = (ulong)player.CreatedAt.ToUnixTimeMilliseconds()
								}
							)]
					}
				)
				.ToArray();

			var request = new RequestUpdate
			{
				port = Config.Load().GetPort(),
				use_address = Config.Load().GetUsedAddress(),
				max_instances = GetMaxInstances(),
				clients = clients,
				instances = instances
			};


			var response = await Request<ResponseUpdate, RequestUpdate>("/api/relays/update", HttpMethod.Post, request);

			if (response.HasError())
			{
				if (_isConnected)
					Logger.Warning("Disconnected from the master server");
				Logger.Debug($"Update Master error: {response.error.message}");
				_isConnected = false;
				return;
			}

			if (!_isConnected)
			{
				Logger.Log("Connected to the master server");
				Logger.Debug($"Server address: {GetMasterAddress()}");
			}

			_isConnected = true;

			foreach (var instanceRaw in response.data.instances)
			{
				var instance = InstanceManager.Get(instanceRaw.master_id);
				if (instance == null)
				{
					instance = new Instance
					{
						Capacity = instanceRaw.capacity,
						MasterId = instanceRaw.master_id,
						Flags = (InstanceFlags)instanceRaw.flags
					};
					Logger.Log($"{instance} added");
				}

				instance.Capacity = instanceRaw.capacity;
				instance.MasterId = instanceRaw.master_id;
				instance.Flags = (InstanceFlags)instanceRaw.flags;
				instance.Password = instanceRaw.password;
				instance.World = new World(instanceRaw.WorldId(), instanceRaw.WorldServer(), instanceRaw.Version());
			}

			var instancesToRemove = InstanceManager.Instances
				.Where(instance => response.data.instances.All(i => i.master_id != instance.MasterId))
				.ToArray();

			foreach (var instance in instancesToRemove)
			{
				Logger.Log($"{instance} removed");
				InstanceManager.Remove(instance);
			}
		}

		public static async Task<MasterResponse<T>> Request<T, TR>(string path, HttpMethod method, TR? data = default)
		{
			try
			{
				var client = new HttpClient();
				client.DefaultRequestHeaders.Authorization =
					new System.Net.Http.Headers.AuthenticationHeaderValue("Badger", GetToken());
				client.Timeout = TimeSpan.FromSeconds(2);
				var request = new HttpRequestMessage(method, $"{GetMasterAddress().TrimEnd('/')}{path}");
				if (data != null)
				{
					var str = JsonConvert.SerializeObject(data);
					var content = new StringContent(str);
					content.Headers.ContentType = new System.Net.Http.Headers.MediaTypeHeaderValue("application/json");
					request.Content = content;
				}

				var response = await client.SendAsync(request);
				var responseString = await response.Content.ReadAsStringAsync();
				return JObject.Parse(responseString).ToObject<MasterResponse<T>>()
					?? new MasterResponse<T>
					{
						error = new MasterResponseError { message = "Invalid response from master server", code = 0, status = 500 }
					};
			}
			catch (Exception e)
			{
				Logger.Exception(e);
				return new MasterResponse<T>
				{
					error = new MasterResponseError { message = e.Message, code = 1, status = 500 }
				};
			}
		}
	}
}