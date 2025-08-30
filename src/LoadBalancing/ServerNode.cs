using System.Net;

namespace Relay.LoadBalancing
{
    /// <summary>
    /// Represents a server node in the load balancer
    /// </summary>
    public class ServerNode
    {
        private int _currentConnections;

        public string Id { get; set; }
        public IPEndPoint EndPoint { get; set; }
        public bool IsHealthy { get; set; }
        
        public int CurrentConnections 
        { 
            get => _currentConnections; 
            set => _currentConnections = value; 
        }
        
        public int MaxConnections { get; set; }
        public long LastHealthCheck { get; set; }
        public double AverageResponseTime { get; set; }
        public double CpuUsage { get; set; }
        public double MemoryUsage { get; set; }
        public int Priority { get; set; } // Higher priority = preferred server

        public ServerNode(string id, IPEndPoint endPoint, int maxConnections = 1000, int priority = 1)
        {
            Id = id;
            EndPoint = endPoint;
            MaxConnections = maxConnections;
            Priority = priority;
            IsHealthy = true;
            CurrentConnections = 0;
            LastHealthCheck = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
            AverageResponseTime = 0;
            CpuUsage = 0;
            MemoryUsage = 0;
        }

        /// <summary>
        /// Gets the load factor of this server (0.0 = no load, 1.0 = full load)
        /// </summary>
        public double LoadFactor
        {
            get
            {
                if (MaxConnections == 0) return 1.0;
                return (double)CurrentConnections / MaxConnections;
            }
        }

        /// <summary>
        /// Gets a composite score for load balancing decisions (lower is better)
        /// </summary>
        public double GetScore()
        {
            if (!IsHealthy) return double.MaxValue;

            // Combine load factor, response time, and resource usage
            var score = (LoadFactor * 0.4) + 
                       (AverageResponseTime / 1000.0 * 0.3) + 
                       (CpuUsage * 0.2) + 
                       (MemoryUsage * 0.1);

            // Apply priority boost (higher priority = lower score)
            score /= Priority;

            return score;
        }

        /// <summary>
        /// Thread-safe increment of current connections
        /// </summary>
        public int IncrementConnections()
        {
            return Interlocked.Increment(ref _currentConnections);
        }

        /// <summary>
        /// Thread-safe decrement of current connections
        /// </summary>
        public int DecrementConnections()
        {
            return Interlocked.Decrement(ref _currentConnections);
        }

        public override string ToString()
        {
            return $"Server {Id} ({EndPoint}) - Connections: {CurrentConnections}/{MaxConnections}, Healthy: {IsHealthy}, Load: {LoadFactor:P1}";
        }
    }
}
