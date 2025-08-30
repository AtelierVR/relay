using System.Net;
using Relay.LoadBalancing;
using Relay.Utils;

namespace Relay.LoadBalancing
{
    /// <summary>
    /// Manages load balancer integration with the relay system
    /// </summary>
    public class LoadBalancerManager
    {
        private readonly LoadBalancer _loadBalancer;
        private readonly Timer _metricsTimer;

        private static readonly Lazy<LoadBalancerManager> _instance = new Lazy<LoadBalancerManager>(() => new LoadBalancerManager());
        public static LoadBalancerManager Instance => _instance.Value;

        public LoadBalancerManager()
        {
            _loadBalancer = LoadBalancer.Instance;
            
            // Initialize with default servers from config
            InitializeDefaultServers();
            
            // Start metrics collection timer (every 30 seconds)
            _metricsTimer = new Timer(CollectMetrics, null, 30000, 30000);
            
            Logger.Log("LoadBalancerManager initialized");
        }

        /// <summary>
        /// Handles a new client connection by selecting the best server
        /// </summary>
        public ServerNode? HandleNewConnection(string clientId)
        {
            var server = _loadBalancer.GetBestServer();
            if (server != null)
            {
                _loadBalancer.RecordConnection(server.Id, clientId);
                Logger.Log($"Assigned client {clientId} to server {server.Id}");
            }
            else
            {
                Logger.Warning($"No available server for client {clientId}");
            }
            return server;
        }

        /// <summary>
        /// Handles client disconnection
        /// </summary>
        public void HandleDisconnection(string serverId, string clientId)
        {
            _loadBalancer.RecordDisconnection(serverId, clientId);
            Logger.Log($"Client {clientId} disconnected from server {serverId}");
        }

        /// <summary>
        /// Updates server performance metrics
        /// </summary>
        public void UpdateServerPerformance(string serverId, double responseTime, double cpuUsage = 0, double memoryUsage = 0)
        {
            _loadBalancer.UpdateServerMetrics(serverId, responseTime, cpuUsage, memoryUsage);
        }

        /// <summary>
        /// Adds a new server to the load balancer
        /// </summary>
        public void AddServer(string id, string host, int port, int maxConnections = 1000, int priority = 1)
        {
            try
            {
                var endPoint = new IPEndPoint(IPAddress.Parse(host), port);
                _loadBalancer.AddServer(id, endPoint, maxConnections, priority);
            }
            catch (Exception ex)
            {
                Logger.Error($"Failed to add server {id}: {ex.Message}");
            }
        }

        /// <summary>
        /// Removes a server from the load balancer
        /// </summary>
        public void RemoveServer(string id)
        {
            _loadBalancer.RemoveServer(id);
        }

        /// <summary>
        /// Gets current load balancer statistics
        /// </summary>
        public LoadBalancerStats GetStats()
        {
            return _loadBalancer.GetStats();
        }

        /// <summary>
        /// Gets all healthy servers
        /// </summary>
        public IReadOnlyList<ServerNode> GetHealthyServers()
        {
            return _loadBalancer.GetHealthyServers();
        }

        /// <summary>
        /// Changes the load balancing algorithm
        /// </summary>
        public void ChangeAlgorithm(LoadBalancingAlgorithm algorithm)
        {
            _loadBalancer.ChangeStrategy(algorithm);
        }

        private void InitializeDefaultServers()
        {
            try
            {
                var config = Config.Load();
                
                // Add local server as default
                AddServer("local-1", "127.0.0.1", config.GetPort(), 500, 1);
                
                // Add additional servers from config if available
                // This would be extended to read from a configuration file
                AddServer("local-2", "127.0.0.1", config.GetPort() + 1, 500, 1);
                
                Logger.Log("Default servers initialized");
            }
            catch (Exception ex)
            {
                Logger.Error($"Failed to initialize default servers: {ex.Message}");
            }
        }

        private void CollectMetrics(object? state)
        {
            try
            {
                var stats = GetStats();
                Logger.Log($"LoadBalancer Metrics: {stats}");
                
                // Log individual server stats
                foreach (var server in _loadBalancer.GetAllServers())
                {
                    if (Logger.PrintDebug)
                    {
                        Logger.Log($"Server {server.Id}: Load={server.LoadFactor:P1}, " +
                                 $"Connections={server.CurrentConnections}/{server.MaxConnections}, " +
                                 $"Healthy={server.IsHealthy}, ResponseTime={server.AverageResponseTime:F1}ms");
                    }
                }
            }
            catch (Exception ex)
            {
                Logger.Error($"Error collecting load balancer metrics: {ex.Message}");
            }
        }

        public void Dispose()
        {
            _metricsTimer?.Dispose();
            _loadBalancer?.Dispose();
        }
    }
}
