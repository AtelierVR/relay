using System.Collections.Concurrent;

namespace Relay.Utils
{
    /// <summary>
    /// A thread-safe buffer pool to reduce memory allocations and garbage collection pressure
    /// </summary>
    public class BufferPool
    {
        private readonly ConcurrentQueue<Buffer> _buffers;
        private readonly int _maxPoolSize;
        private readonly object _lock = new object();
        private int _currentPoolSize;

        private static readonly Lazy<BufferPool> _instance = new Lazy<BufferPool>(() => new BufferPool());
        public static BufferPool Instance => _instance.Value;

        /// <summary>
        /// Creates a new BufferPool with the specified maximum pool size
        /// </summary>
        /// <param name="maxPoolSize">Maximum number of buffers to keep in the pool (default: 1000)</param>
        public BufferPool(int maxPoolSize = 1000)
        {
            _maxPoolSize = maxPoolSize;
            _buffers = new ConcurrentQueue<Buffer>();
            _currentPoolSize = 0;
        }

        /// <summary>
        /// Rents a buffer from the pool. If no buffer is available, creates a new one.
        /// </summary>
        /// <param name="initialOffset">Initial offset for the buffer (default: 0)</param>
        /// <returns>A Buffer instance ready for use</returns>
        public Buffer Rent(ushort initialOffset = 0)
        {
            if (_buffers.TryDequeue(out var buffer))
            {
                lock (_lock)
                {
                    _currentPoolSize--;
                }
                
                // Reset the buffer for reuse
                buffer.Clear();
                buffer.offset = initialOffset;
                buffer.length = initialOffset;
                return buffer;
            }

            // No buffer available in pool, create a new one
            return new Buffer(initialOffset);
        }

        /// <summary>
        /// Returns a buffer to the pool for reuse
        /// </summary>
        /// <param name="buffer">The buffer to return to the pool</param>
        public void Return(Buffer buffer)
        {
            if (buffer == null)
                return;

            lock (_lock)
            {
                if (_currentPoolSize >= _maxPoolSize)
                {
                    // Pool is full, let the buffer be garbage collected
                    return;
                }
                
                _currentPoolSize++;
            }

            // Clear sensitive data before returning to pool
            buffer.Clear();
            _buffers.Enqueue(buffer);
        }

        /// <summary>
        /// Gets the current number of buffers in the pool
        /// </summary>
        public int PoolSize
        {
            get
            {
                lock (_lock)
                {
                    return _currentPoolSize;
                }
            }
        }

        /// <summary>
        /// Gets the maximum pool size
        /// </summary>
        public int MaxPoolSize => _maxPoolSize;

        /// <summary>
        /// Clears all buffers from the pool
        /// </summary>
        public void Clear()
        {
            while (_buffers.TryDequeue(out _))
            {
                // Just dequeue all buffers
            }

            lock (_lock)
            {
                _currentPoolSize = 0;
            }
        }

        /// <summary>
        /// Preloads the pool with the specified number of buffers
        /// </summary>
        /// <param name="count">Number of buffers to preload</param>
        public void Preload(int count)
        {
            count = Math.Min(count, _maxPoolSize);
            
            for (int i = 0; i < count; i++)
            {
                var buffer = new Buffer();
                Return(buffer);
            }
        }
    }
}
