using Newtonsoft.Json.Linq;

namespace Relay.Utils
{
	public class Config
	{
		public static string GetPath()
			=> Path.Combine(Environment.CurrentDirectory, "config.json");

		public static Config? Current;

		private JObject _jsonObject = new JObject();

		public static Config Load(bool force = false)
		{
			if (Current != null && !force) return Current;
			if (!File.Exists(GetPath()))
			{
				Logger.Log("Config file not found, creating a new one...");
				var initialConfig = HasEnv("initial_config") ? GetFromEnv<string>("initial_config") : null;
				if (initialConfig != null && string.IsNullOrEmpty(initialConfig))
				{
					Logger.Log("Using initial config from environment variable...");
					var obj = JObject.Parse(initialConfig);
					Current = new Config { _jsonObject = obj }.Save();
				}
				else
				{
					Logger.Log("Using default config...");
					Current = new Config().Save();
				}
			}
			else
			{
				Logger.Log("Loading config file...");
				var jsonString = File.ReadAllText(GetPath());
				var config = new Config() { _jsonObject = JObject.Parse(jsonString) };
				Current = config;
			}

			return Current;
		}

		public bool Has(string propertyName)
			=> _jsonObject[propertyName] != null;

		public T? Get<T>(string propertyName, T? defaultValue = default)
		{
			var token = _jsonObject[propertyName];
			return token == null
				? defaultValue
				: token.ToObject<T>();
		}

		public static T? GetFromEnv<T>(string key)
		{
			var value = Environment.GetEnvironmentVariable($"nox_{key}".ToUpper());
			if (value == null) return default;
			return (T)Convert.ChangeType(value, typeof(T));
		}

		public static bool HasEnv(string key)
			=> Environment.GetEnvironmentVariable($"nox_{key}".ToUpper()) != null;

		public void Set<T>(string propertyName, T? value)
			=> _jsonObject[propertyName] = value == null ? null : JToken.FromObject(value);

		public Config Save()
		{
			File.WriteAllText(GetPath(), _jsonObject.ToString());
			return this;
		}

		public ushort GetPort()
			=> Has("port")
				? Get<ushort>("port")
				: (
				HasEnv("port")
					? GetFromEnv<ushort>("port")
					: Constants.Port
				);

		public ushort GetConnectionTimeout()
			=> Has("connection_timeout")
				? Get<ushort>("connection_timeout")
				: (
				HasEnv("connection_timeout")
					? GetFromEnv<ushort>("connection_timeout")
					: Constants.ConnectionTimeout
				);

		public ushort GetKeepAliveInterval()
			=> Has("keep_alive_interval")
				? Get<ushort>("keep_alive_interval")
				: (
				HasEnv("keep_alive_interval")
					? GetFromEnv<ushort>("keep_alive_interval")
					: Constants.KeepAliveInterval
				);

		public string GetMasterGateway()
			=> Get<string>("master_gateway") ?? GetFromEnv<string>("master_gateway") ?? Constants.MasterGateway;

		public string GetToken()
			=> Get<string>("token") ?? GetFromEnv<string>("token") ?? "";

		public byte GetMaxInstances()
			=> Has("max_instances")
				? Get<byte>("max_instances")
				: (
				HasEnv("max_instances")
					? GetFromEnv<byte>("max_instances")
					: Constants.MaxInstances
				);

		public string GetUsedAddress()
			=> Get<string>("use_address") ?? GetFromEnv<string>("use_address") ?? Constants.UseAddress;
	}
}