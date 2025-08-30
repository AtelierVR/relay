namespace Relay.LoadBalancing
{
    /// <summary>
    /// Configuration settings for the load balancer
    /// </summary>
    public class LoadBalancerConfig
    {
        /// <summary>
        /// Load balancing algorithm to use
        /// </summary>
        public LoadBalancingAlgorithm Algorithm { get; set; } = LoadBalancingAlgorithm.ResourceBased;

        /// <summary>
        /// Health check interval in milliseconds
        /// </summary>
        public int HealthCheckInterval { get; set; } = 30000;

        /// <summary>
        /// Health check timeout in milliseconds
        /// </summary>
        public int HealthCheckTimeout { get; set; } = 5000;

        /// <summary>
        /// Maximum number of failed health checks before marking server as unhealthy
        /// </summary>
        public int MaxFailedHealthChecks { get; set; } = 3;

        /// <summary>
        /// Maximum number of buffers to keep in the buffer pool
        /// </summary>
        public int BufferPoolSize { get; set; } = 1000;

        /// <summary>
        /// Number of buffers to preload in the pool
        /// </summary>
        public int BufferPoolPreload { get; set; } = 100;

        /// <summary>
        /// Enable detailed load balancer logging
        /// </summary>
        public bool EnableDetailedLogging { get; set; } = true;

        /// <summary>
        /// Metrics collection interval in milliseconds
        /// </summary>
        public int MetricsInterval { get; set; } = 30000;

        /// <summary>
        /// Connection cleanup interval in milliseconds
        /// </summary>
        public int CleanupInterval { get; set; } = 300000;

        /// <summary>
        /// List of server configurations
        /// </summary>
        public List<ServerConfig> Servers { get; set; } = new List<ServerConfig>();

        public static LoadBalancerConfig Default()
        {
            return new LoadBalancerConfig
            {
                Servers = new List<ServerConfig>
                {
                    new ServerConfig
                    {
                        Id = "local-1",
                        Host = "127.0.0.1",
                        Port = 23032,
                        MaxConnections = 500,
                        Priority = 1
                    }
                }
            };
        }
    }

    /// <summary>
    /// Configuration for a single server
    /// </summary>
    public class ServerConfig
    {
        /// <summary>
        /// Unique server identifier
        /// </summary>
        public string Id { get; set; } = string.Empty;

        /// <summary>
        /// Server hostname or IP address
        /// </summary>
        public string Host { get; set; } = "127.0.0.1";

        /// <summary>
        /// Server port
        /// </summary>
        public int Port { get; set; } = 23032;

        /// <summary>
        /// Maximum number of connections this server can handle
        /// </summary>
        public int MaxConnections { get; set; } = 1000;

        /// <summary>
        /// Server priority (higher values indicate higher priority)
        /// </summary>
        public int Priority { get; set; } = 1;

        /// <summary>
        /// Whether this server is enabled
        /// </summary>
        public bool Enabled { get; set; } = true;

        /// <summary>
        /// Server region or zone
        /// </summary>
        public string Region { get; set; } = "default";

        /// <summary>
        /// Additional server tags for filtering
        /// </summary>
        public Dictionary<string, string> Tags { get; set; } = new Dictionary<string, string>();
    }
}
