using System.Collections.Concurrent;
using System.Net;
using System.Net.NetworkInformation;
using Relay.LoadBalancing.Strategies;
using Relay.Utils;

namespace Relay.LoadBalancing
{
    /// <summary>
    /// Main load balancer class that manages server nodes and distributes traffic
    /// </summary>
    public class LoadBalancer
    {
        private readonly ConcurrentDictionary<string, ServerNode> _servers;
        private readonly ILoadBalancingStrategy _strategy;
        private readonly Timer _healthCheckTimer;
        private readonly Timer _cleanupTimer;
        private readonly object _strategyLock = new object();

        // Health check settings
        private readonly int _healthCheckInterval = 30000; // 30 seconds
        private readonly int _healthCheckTimeout = 5000;   // 5 seconds

        // Connection tracking
        private readonly ConcurrentDictionary<string, List<string>> _clientConnections;

        private static readonly Lazy<LoadBalancer> _instance = new Lazy<LoadBalancer>(() => new LoadBalancer());
        public static LoadBalancer Instance => _instance.Value;

        public LoadBalancer(LoadBalancingAlgorithm algorithm = LoadBalancingAlgorithm.ResourceBased)
        {
            _servers = new ConcurrentDictionary<string, ServerNode>();
            _clientConnections = new ConcurrentDictionary<string, List<string>>();
            _strategy = CreateStrategy(algorithm);

            // Start health check timer
            _healthCheckTimer = new Timer(PerformHealthChecks, null, _healthCheckInterval, _healthCheckInterval);
            
            // Start cleanup timer (runs every 5 minutes)
            _cleanupTimer = new Timer(CleanupStaleConnections, null, 300000, 300000);
        }

        /// <summary>
        /// Adds a server to the load balancer
        /// </summary>
        public void AddServer(string id, IPEndPoint endPoint, int maxConnections = 1000, int priority = 1)
        {
            var server = new ServerNode(id, endPoint, maxConnections, priority);
            _servers.TryAdd(id, server);
            Logger.Log($"Added server to load balancer: {server}");
        }

        /// <summary>
        /// Removes a server from the load balancer
        /// </summary>
        public void RemoveServer(string id)
        {
            if (_servers.TryRemove(id, out var server))
            {
                Logger.Log($"Removed server from load balancer: {server}");
            }
        }

        /// <summary>
        /// Gets the best available server for a new connection
        /// </summary>
        public ServerNode? GetBestServer()
        {
            var servers = _servers.Values.ToList();
            
            lock (_strategyLock)
            {
                return _strategy.SelectServer(servers);
            }
        }

        /// <summary>
        /// Gets a server by its ID
        /// </summary>
        public ServerNode? GetServer(string id)
        {
            _servers.TryGetValue(id, out var server);
            return server;
        }

        /// <summary>
        /// Records a new client connection to a server
        /// </summary>
        public void RecordConnection(string serverId, string clientId)
        {
            if (_servers.TryGetValue(serverId, out var server))
            {
                server.IncrementConnections();
                
                // Track client connection
                _clientConnections.AddOrUpdate(clientId, 
                    new List<string> { serverId },
                    (key, list) => 
                    {
                        list.Add(serverId);
                        return list;
                    });
            }
        }

        /// <summary>
        /// Records a client disconnection from a server
        /// </summary>
        public void RecordDisconnection(string serverId, string clientId)
        {
            if (_servers.TryGetValue(serverId, out var server))
            {
                var newCount = server.DecrementConnections();
                if (newCount < 0)
                    server.CurrentConnections = 0;
            }

            // Remove client connection tracking
            if (_clientConnections.TryGetValue(clientId, out var connections))
            {
                connections.Remove(serverId);
                if (connections.Count == 0)
                {
                    _clientConnections.TryRemove(clientId, out _);
                }
            }
        }

        /// <summary>
        /// Updates server performance metrics
        /// </summary>
        public void UpdateServerMetrics(string serverId, double responseTime, double cpuUsage, double memoryUsage)
        {
            if (_servers.TryGetValue(serverId, out var server))
            {
                // Update average response time using exponential moving average
                server.AverageResponseTime = server.AverageResponseTime == 0 
                    ? responseTime 
                    : (server.AverageResponseTime * 0.9) + (responseTime * 0.1);
                
                server.CpuUsage = cpuUsage;
                server.MemoryUsage = memoryUsage;
                server.LastHealthCheck = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
            }
        }

        /// <summary>
        /// Gets all registered servers
        /// </summary>
        public IReadOnlyList<ServerNode> GetAllServers()
        {
            return _servers.Values.ToList().AsReadOnly();
        }

        /// <summary>
        /// Gets only healthy servers
        /// </summary>
        public IReadOnlyList<ServerNode> GetHealthyServers()
        {
            return _servers.Values.Where(s => s.IsHealthy).ToList().AsReadOnly();
        }

        /// <summary>
        /// Changes the load balancing strategy
        /// </summary>
        public void ChangeStrategy(LoadBalancingAlgorithm algorithm)
        {
            lock (_strategyLock)
            {
                var newStrategy = CreateStrategy(algorithm);
                Logger.Log($"Changed load balancing strategy to: {algorithm}");
            }
        }

        private ILoadBalancingStrategy CreateStrategy(LoadBalancingAlgorithm algorithm)
        {
            return algorithm switch
            {
                LoadBalancingAlgorithm.RoundRobin => new RoundRobinStrategy(),
                LoadBalancingAlgorithm.WeightedRoundRobin => new WeightedRoundRobinStrategy(),
                LoadBalancingAlgorithm.LeastConnections => new LeastConnectionsStrategy(),
                LoadBalancingAlgorithm.WeightedLeastConnections => new WeightedLeastConnectionsStrategy(),
                LoadBalancingAlgorithm.ResourceBased => new ResourceBasedStrategy(),
                LoadBalancingAlgorithm.ResponseTime => new ResponseTimeStrategy(),
                _ => new ResourceBasedStrategy()
            };
        }

        private async void PerformHealthChecks(object? state)
        {
            var tasks = _servers.Values.Select(CheckServerHealth).ToArray();
            await Task.WhenAll(tasks);
        }

        private async Task CheckServerHealth(ServerNode server)
        {
            try
            {
                var ping = new Ping();
                var reply = await ping.SendPingAsync(server.EndPoint.Address, _healthCheckTimeout);
                
                var wasHealthy = server.IsHealthy;
                server.IsHealthy = reply.Status == IPStatus.Success;
                server.LastHealthCheck = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();

                if (reply.Status == IPStatus.Success)
                {
                    server.AverageResponseTime = server.AverageResponseTime == 0 
                        ? reply.RoundtripTime 
                        : (server.AverageResponseTime * 0.9) + (reply.RoundtripTime * 0.1);
                }

                if (wasHealthy != server.IsHealthy)
                {
                    Logger.Log($"Server {server.Id} health changed: {(server.IsHealthy ? "Healthy" : "Unhealthy")}");
                }
            }
            catch (Exception ex)
            {
                server.IsHealthy = false;
                Logger.Error($"Health check failed for server {server.Id}: {ex.Message}");
            }
        }

        private void CleanupStaleConnections(object? state)
        {
            var cutoffTime = DateTimeOffset.UtcNow.AddMinutes(-10).ToUnixTimeMilliseconds();
            var staleClients = _clientConnections.Keys.ToList();

            foreach (var clientId in staleClients)
            {
                // This is a simple cleanup - in a real implementation, you'd want to track
                // connection timestamps and remove truly stale connections
                if (_clientConnections.TryGetValue(clientId, out var connections))
                {
                    // Remove connections that haven't been active
                    // This is placeholder logic - implement based on your connection tracking needs
                }
            }
        }

        /// <summary>
        /// Gets load balancer statistics
        /// </summary>
        public LoadBalancerStats GetStats()
        {
            var servers = _servers.Values.ToList();
            return new LoadBalancerStats
            {
                TotalServers = servers.Count,
                HealthyServers = servers.Count(s => s.IsHealthy),
                TotalConnections = servers.Sum(s => s.CurrentConnections),
                AverageLoad = servers.Where(s => s.IsHealthy).Average(s => s.LoadFactor),
                TotalClients = _clientConnections.Count
            };
        }

        public void Dispose()
        {
            _healthCheckTimer?.Dispose();
            _cleanupTimer?.Dispose();
        }
    }

    /// <summary>
    /// Load balancer statistics
    /// </summary>
    public class LoadBalancerStats
    {
        public int TotalServers { get; set; }
        public int HealthyServers { get; set; }
        public int TotalConnections { get; set; }
        public double AverageLoad { get; set; }
        public int TotalClients { get; set; }

        public override string ToString()
        {
            return $"LoadBalancer Stats - Servers: {HealthyServers}/{TotalServers}, " +
                   $"Connections: {TotalConnections}, Clients: {TotalClients}, " +
                   $"Avg Load: {AverageLoad:P1}";
        }
    }
}
