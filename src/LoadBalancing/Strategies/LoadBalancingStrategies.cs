namespace Relay.LoadBalancing.Strategies
{
    /// <summary>
    /// Round Robin load balancing strategy
    /// </summary>
    public class RoundRobinStrategy : ILoadBalancingStrategy
    {
        private int _currentIndex = 0;
        private readonly object _lock = new object();

        public ServerNode? SelectServer(IList<ServerNode> servers)
        {
            if (servers == null || servers.Count == 0)
                return null;

            var healthyServers = servers.Where(s => s.IsHealthy && s.LoadFactor < 1.0).ToList();
            if (healthyServers.Count == 0)
                return null;

            lock (_lock)
            {
                _currentIndex = (_currentIndex + 1) % healthyServers.Count;
                return healthyServers[_currentIndex];
            }
        }
    }

    /// <summary>
    /// Weighted Round Robin load balancing strategy based on server priority
    /// </summary>
    public class WeightedRoundRobinStrategy : ILoadBalancingStrategy
    {
        private readonly Dictionary<string, int> _serverWeights = new();
        private readonly object _lock = new object();

        public ServerNode? SelectServer(IList<ServerNode> servers)
        {
            if (servers == null || servers.Count == 0)
                return null;

            var healthyServers = servers.Where(s => s.IsHealthy && s.LoadFactor < 1.0).ToList();
            if (healthyServers.Count == 0)
                return null;

            lock (_lock)
            {
                // Initialize weights if needed
                foreach (var server in healthyServers)
                {
                    if (!_serverWeights.ContainsKey(server.Id))
                        _serverWeights[server.Id] = server.Priority;
                }

                // Find server with highest current weight
                ServerNode? selectedServer = null;
                int maxWeight = -1;

                foreach (var server in healthyServers)
                {
                    if (_serverWeights[server.Id] > maxWeight)
                    {
                        maxWeight = _serverWeights[server.Id];
                        selectedServer = server;
                    }
                }

                if (selectedServer != null)
                {
                    // Decrease selected server's weight
                    _serverWeights[selectedServer.Id] -= healthyServers.Sum(s => s.Priority);
                    
                    // Increase all servers' weights by their priority
                    foreach (var server in healthyServers)
                    {
                        _serverWeights[server.Id] += server.Priority;
                    }
                }

                return selectedServer;
            }
        }
    }

    /// <summary>
    /// Least Connections load balancing strategy
    /// </summary>
    public class LeastConnectionsStrategy : ILoadBalancingStrategy
    {
        public ServerNode? SelectServer(IList<ServerNode> servers)
        {
            if (servers == null || servers.Count == 0)
                return null;

            var healthyServers = servers.Where(s => s.IsHealthy && s.LoadFactor < 1.0).ToList();
            if (healthyServers.Count == 0)
                return null;

            return healthyServers.OrderBy(s => s.CurrentConnections).First();
        }
    }

    /// <summary>
    /// Weighted Least Connections strategy considering server priority
    /// </summary>
    public class WeightedLeastConnectionsStrategy : ILoadBalancingStrategy
    {
        public ServerNode? SelectServer(IList<ServerNode> servers)
        {
            if (servers == null || servers.Count == 0)
                return null;

            var healthyServers = servers.Where(s => s.IsHealthy && s.LoadFactor < 1.0).ToList();
            if (healthyServers.Count == 0)
                return null;

            return healthyServers.OrderBy(s => (double)s.CurrentConnections / s.Priority).First();
        }
    }

    /// <summary>
    /// Resource-based load balancing strategy considering CPU and memory usage
    /// </summary>
    public class ResourceBasedStrategy : ILoadBalancingStrategy
    {
        public ServerNode? SelectServer(IList<ServerNode> servers)
        {
            if (servers == null || servers.Count == 0)
                return null;

            var healthyServers = servers.Where(s => s.IsHealthy && s.LoadFactor < 1.0).ToList();
            if (healthyServers.Count == 0)
                return null;

            return healthyServers.OrderBy(s => s.GetScore()).First();
        }
    }

    /// <summary>
    /// Response Time based load balancing strategy
    /// </summary>
    public class ResponseTimeStrategy : ILoadBalancingStrategy
    {
        public ServerNode? SelectServer(IList<ServerNode> servers)
        {
            if (servers == null || servers.Count == 0)
                return null;

            var healthyServers = servers.Where(s => s.IsHealthy && s.LoadFactor < 1.0).ToList();
            if (healthyServers.Count == 0)
                return null;

            return healthyServers.OrderBy(s => s.AverageResponseTime).First();
        }
    }
}
