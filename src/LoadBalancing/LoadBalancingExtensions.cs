using Relay.LoadBalancing;
using Relay.Utils;

namespace Relay.LoadBalancing
{
    /// <summary>
    /// Extension methods for integrating load balancing with existing relay components
    /// </summary>
    public static class LoadBalancingExtensions
    {
        /// <summary>
        /// Executes a request with performance monitoring
        /// </summary>
        public static T ExecuteWithMonitoring<T>(this Func<T> action, string serverId = "local")
        {
            return PerformanceMonitor.Instance.MeasureExecutionTime(() =>
            {
                var result = action();
                
                // Update server metrics with the performance data
                var metrics = PerformanceMonitor.Instance.GetSystemMetrics();
                LoadBalancerManager.Instance.UpdateServerPerformance(
                    serverId, 
                    metrics.AverageResponseTime, 
                    metrics.CpuUsage, 
                    metrics.MemoryUsage
                );
                
                return result;
            });
        }

        /// <summary>
        /// Executes a request with performance monitoring
        /// </summary>
        public static void ExecuteWithMonitoring(this Action action, string serverId = "local")
        {
            PerformanceMonitor.Instance.MeasureExecutionTime(() =>
            {
                action();
                
                // Update server metrics with the performance data
                var metrics = PerformanceMonitor.Instance.GetSystemMetrics();
                LoadBalancerManager.Instance.UpdateServerPerformance(
                    serverId, 
                    metrics.AverageResponseTime, 
                    metrics.CpuUsage, 
                    metrics.MemoryUsage
                );
            });
        }

        /// <summary>
        /// Gets or creates a client ID for load balancer tracking
        /// </summary>
        public static string GetClientId(this IRemote remote)
        {
            return $"{remote.Address}:{remote.Port}";
        }
    }
}
