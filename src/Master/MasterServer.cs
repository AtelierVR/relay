using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using Relay.Clients;
using Relay.Instances;
using Relay.Master.Update;
using Relay.Utils;
using System.Diagnostics;

namespace Relay.Master
{
    public class MasterServer
    {
        public static string ServerAddress;
        public static string ServerGateway;
        private static string _token = "";
        private static byte _maxInstances;
        public static bool IsConnected { get; private set; } = false;
        public static string MasterAddress = "";
        public static DateTime LastUpdate = DateTime.Now;
        public static ushort NextUpdateTime = Constants.DefaultUpdateTime;

        public static void UpdateImediately() => LastUpdate = DateTime.MinValue;


        public async void Start()
        {
            var config = Config.Load();
            ServerGateway = config.GetMasterGateway();
            ServerAddress = ServerGateway
                .Replace("http://", "").Replace("https://", "")
                .Split('/')[0];
            _token = config.GetToken();
            _maxInstances = config.GetMaxInstances();



            while (true)
            {
                if (IsConnected && DateTime.Now - LastUpdate < TimeSpan.FromMilliseconds(NextUpdateTime))
                {
                    await Task.Delay(Constants.MinUpdateTime);
                    continue;
                }
                else if (!IsConnected)
                    await Task.Delay(NextUpdateTime);

                LastUpdate = DateTime.Now;

                try
                {
                    await SendUpdate();
                }
                catch (Exception e)
                {
                    Logger.Warning("Failed to update master server");
                    Logger.Exception(e);

                    IsConnected = false;
                    MasterAddress = "";
                }
            }
        }


        public async Task SendUpdate()
        {
            var clients = ClientManager.Clients.ToArray().Select(client => new RequestClient
            {
                id = client.Id,
                remote = client.Remote.Address + ":" + client.Remote.Port,
                status = client.Status.ToString().ToLower(),
                platform = client.Platform.ToString().ToLower(),
                engine = client.Engine.ToString().ToLower(),
                last_seen = (ulong)client.LastSeen.ToUnixTimeMilliseconds(),
                user = client.User == null ? null : new RequestUser
                {
                    id = client.User.Id,
                    address = client.User.Address
                }
            }).ToArray();

            var instances = InstanceManager.Instances.ToArray().Select(instance => new RequestInstance
            {
                internal_id = instance.InternalId,
                master_id = instance.MasterId,
                flags = (uint)instance.Flags,
                max_players = instance.Capacity,
                players = instance.Players.ToArray().Select(player => new RequestPlayer
                {
                    id = player.Id,
                    client_id = player.Client.Id,
                    display = player.Display,
                    flags = (uint)player.Flags,
                    status = (byte)player.Status,
                    created_at = (ulong)player.CreatedAt.ToUnixTimeMilliseconds()
                }).ToArray()
            }).ToArray();

            var request = new RequestUpdate()
            {
                port = Config.Load().GetPort(),
                use_address = Config.Load().GetUseAddress(),
                max_instances = _maxInstances,
                clients = clients,
                instances = instances
            };

            var response = await Request<ResponseUpdate, RequestUpdate>("/api/relays/update", HttpMethod.Post, request);

            if (response.HasError())
            {
                if (IsConnected)
                    Logger.Warning("Disconnected from the master server");
                Logger.Debug($"Update Master ({ServerGateway}) error: {response.error.message}");

                IsConnected = false;
                MasterAddress = "";
            }
            else
            {
                if (!IsConnected)
                {
                    Logger.Log("Connected to the master server");
                    Logger.Debug($"Server address: {ServerGateway}");
                }

                NextUpdateTime = response.data.next_update;
                if (NextUpdateTime < Constants.MinUpdateTime)
                    NextUpdateTime = Constants.MinUpdateTime;

                IsConnected = true;
                MasterAddress = response.data.server;

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
                    instance.World = new World
                    {
                        MasterId = instanceRaw.WorldId(),
                        Address = instanceRaw.WorldServer() ?? response.data.server,
                        Version = instanceRaw.Version()
                    };
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

            return;
        }

        public static async Task<MasterResponse<T>> Request<T, TR>(string path, HttpMethod method, TR data = default)
        {
            try
            {
                var client = new HttpClient();
                client.DefaultRequestHeaders.Authorization =
                    new System.Net.Http.Headers.AuthenticationHeaderValue("Badger", _token);
                client.Timeout = TimeSpan.FromSeconds(2);
                var request = new HttpRequestMessage(method, $"{ServerGateway.TrimEnd('/')}{path}");
                if (data != null)
                {
                    var str = JsonConvert.SerializeObject(data);
                    var content = new StringContent(str);
                    content.Headers.ContentType = new System.Net.Http.Headers.MediaTypeHeaderValue("application/json");
                    request.Content = content;
                }
                var response = await client.SendAsync(request);
                var responseString = await response.Content.ReadAsStringAsync();
                return JObject
                    .Parse(responseString)
                    .ToObject<MasterResponse<T>>();
            }
            catch (Exception e)
            {
                Logger.Exception(e);
                return new MasterResponse<T>()
                {
                    error = new MasterResponseError() { message = e.Message, code = 1, status = 500 }
                };
            }
        }
    }
}