namespace Relay.LoadBalancing
{
    /// <summary>
    /// Load balancing algorithms
    /// </summary>
    public enum LoadBalancingAlgorithm
    {
        RoundRobin,
        WeightedRoundRobin,
        LeastConnections,
        WeightedLeastConnections,
        ResourceBased,
        ResponseTime
    }

    /// <summary>
    /// Interface for load balancing strategies
    /// </summary>
    public interface ILoadBalancingStrategy
    {
        /// <summary>
        /// Selects the best server from the available servers
        /// </summary>
        /// <param name="servers">List of available servers</param>
        /// <returns>The selected server, or null if no server is available</returns>
        ServerNode? SelectServer(IList<ServerNode> servers);
    }
}
