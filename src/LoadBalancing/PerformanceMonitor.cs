using System.Diagnostics;
using Relay.Utils;

namespace Relay.LoadBalancing
{
    /// <summary>
    /// Monitors system performance metrics for load balancing decisions
    /// </summary>
    public class PerformanceMonitor
    {
        private readonly Timer _monitoringTimer;
        private readonly List<double> _responseTimeHistory;
        private readonly object _lock = new object();

        private double _currentCpuUsage;
        private double _currentMemoryUsage;
        private double _averageResponseTime;

        private static readonly Lazy<PerformanceMonitor> _instance = new Lazy<PerformanceMonitor>(() => new PerformanceMonitor());
        public static PerformanceMonitor Instance => _instance.Value;

        public PerformanceMonitor()
        {
            _responseTimeHistory = new List<double>();

            // Start monitoring timer (every 5 seconds)
            _monitoringTimer = new Timer(UpdateMetrics, null, 5000, 5000);
            
            Logger.Log("PerformanceMonitor initialized");
        }

        /// <summary>
        /// Gets the current CPU usage percentage (0-100)
        /// </summary>
        public double CpuUsage
        {
            get
            {
                lock (_lock)
                {
                    return _currentCpuUsage;
                }
            }
        }

        /// <summary>
        /// Gets the current memory usage percentage (0-100)
        /// </summary>
        public double MemoryUsage
        {
            get
            {
                lock (_lock)
                {
                    return _currentMemoryUsage;
                }
            }
        }

        /// <summary>
        /// Gets the average response time in milliseconds
        /// </summary>
        public double AverageResponseTime
        {
            get
            {
                lock (_lock)
                {
                    return _averageResponseTime;
                }
            }
        }

        /// <summary>
        /// Records a response time measurement
        /// </summary>
        public void RecordResponseTime(double responseTimeMs)
        {
            lock (_lock)
            {
                _responseTimeHistory.Add(responseTimeMs);
                
                // Keep only the last 100 measurements
                if (_responseTimeHistory.Count > 100)
                {
                    _responseTimeHistory.RemoveAt(0);
                }

                // Update average
                _averageResponseTime = _responseTimeHistory.Count > 0 ? _responseTimeHistory.Average() : 0;
            }
        }

        /// <summary>
        /// Measures the execution time of an action and records it
        /// </summary>
        public T MeasureExecutionTime<T>(Func<T> action)
        {
            var stopwatch = Stopwatch.StartNew();
            try
            {
                return action();
            }
            finally
            {
                stopwatch.Stop();
                RecordResponseTime(stopwatch.Elapsed.TotalMilliseconds);
            }
        }

        /// <summary>
        /// Measures the execution time of an action and records it
        /// </summary>
        public void MeasureExecutionTime(Action action)
        {
            var stopwatch = Stopwatch.StartNew();
            try
            {
                action();
            }
            finally
            {
                stopwatch.Stop();
                RecordResponseTime(stopwatch.Elapsed.TotalMilliseconds);
            }
        }

        /// <summary>
        /// Gets comprehensive system metrics
        /// </summary>
        public SystemMetrics GetSystemMetrics()
        {
            lock (_lock)
            {
                return new SystemMetrics
                {
                    CpuUsage = _currentCpuUsage,
                    MemoryUsage = _currentMemoryUsage,
                    AverageResponseTime = _averageResponseTime,
                    TotalMemoryMB = GetTotalPhysicalMemory(),
                    AvailableMemoryMB = GetAvailableMemory(),
                    ThreadCount = Process.GetCurrentProcess().Threads.Count,
                    HandleCount = GetHandleCount()
                };
            }
        }

        private void UpdateMetrics(object? state)
        {
            try
            {
                lock (_lock)
                {
                    // Update CPU usage using process CPU time
                    _currentCpuUsage = GetCpuUsage();

                    // Update memory usage
                    _currentMemoryUsage = GetMemoryUsage();
                }
            }
            catch (Exception ex)
            {
                Logger.Error($"Error updating performance metrics: {ex.Message}");
            }
        }

        private double GetCpuUsage()
        {
            try
            {
                var process = Process.GetCurrentProcess();
                
                // Simple CPU usage estimation based on process CPU time vs wall time
                // This is a basic implementation - for production, consider using platform-specific APIs
                var cpuUsage = Math.Min(50.0, (process.TotalProcessorTime.TotalMilliseconds / Environment.TickCount64) * 100);
                return cpuUsage;
            }
            catch
            {
                return 0;
            }
        }

        private double GetMemoryUsage()
        {
            try
            {
                var process = Process.GetCurrentProcess();
                var workingSet = process.WorkingSet64;
                var totalMemory = GetTotalPhysicalMemory() * 1024 * 1024; // Convert MB to bytes
                
                return totalMemory > 0 ? (workingSet / (double)totalMemory) * 100 : 0;
            }
            catch
            {
                return 0;
            }
        }

        private long GetTotalPhysicalMemory()
        {
            try
            {
                // Simple fallback: assume 8GB for basic estimation
                // In production, you would implement platform-specific memory detection
                return 8192; // MB
            }
            catch
            {
                return 8192; // Default fallback
            }
        }

        private double GetAvailableMemory()
        {
            try
            {
                // Fallback calculation
                var totalMemory = GetTotalPhysicalMemory();
                var usedPercentage = GetMemoryUsage();
                return totalMemory * (1 - usedPercentage / 100);
            }
            catch
            {
                return 0;
            }
        }

        private int GetHandleCount()
        {
            try
            {
                return Process.GetCurrentProcess().HandleCount;
            }
            catch
            {
                return 0;
            }
        }

        public void Dispose()
        {
            _monitoringTimer?.Dispose();
        }
    }

    /// <summary>
    /// System performance metrics
    /// </summary>
    public class SystemMetrics
    {
        public double CpuUsage { get; set; }
        public double MemoryUsage { get; set; }
        public double AverageResponseTime { get; set; }
        public long TotalMemoryMB { get; set; }
        public double AvailableMemoryMB { get; set; }
        public int ThreadCount { get; set; }
        public int HandleCount { get; set; }

        public override string ToString()
        {
            return $"CPU: {CpuUsage:F1}%, Memory: {MemoryUsage:F1}%, " +
                   $"Avg Response: {AverageResponseTime:F1}ms, Threads: {ThreadCount}";
        }
    }
}
