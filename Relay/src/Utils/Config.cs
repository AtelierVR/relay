using System;
using System.IO;
using System.Security.Policy;
using Newtonsoft.Json.Linq;

namespace Relay.Utils
{
    public class Config
    {
        public static string GetPath() => Path.Combine(Environment.CurrentDirectory, "config.json");
        public static Config Current;

        private JObject _jsonObject = new JObject();

        public static Config Load(bool force = false)
        {
            if (Current != null && !force) return Current;
            if (!File.Exists(GetPath()))
                return new Config().Save();
            var jsonString = File.ReadAllText(GetPath());
            var config = new Config() { _jsonObject = JObject.Parse(jsonString) };
            Current = config;
            return config;
        }

        public bool Has(string propertyName) => _jsonObject[propertyName] != null;

        public T Get<T>(string propertyName)
        {
            var token = _jsonObject[propertyName];
            return token != null ? token.ToObject<T>() : default(T);
        }

        public void Set<T>(string propertyName, T value)
        {
            _jsonObject[propertyName] = JToken.FromObject(value);
        }

        public Config Save()
        {
            File.WriteAllText(GetPath(), _jsonObject.ToString());
            return this;
        }

        public ushort GetPort() => Has("port") ? Get<ushort>("port") : Constants.Port;

        public ushort GetConnectionTimeout() => Has("connection_timeout")
            ? Get<ushort>("connection_timeout")
            : Constants.ConnectionTimeout;

        public string GetMasterGateway() =>
            Has("master_gateway") ? Get<string>("master_gateway") : Constants.MasterGateway;

        public string GetToken() => Has("token") ? Get<string>("token") : "";
        public byte GetMaxInstances() => Has("max_instances") ? Get<byte>("max_instances") : Constants.MaxInstances;
        public string GetUseAddress() => Has("use_address") ? Get<string>("use_address") : Constants.UseAddress;
    }
}