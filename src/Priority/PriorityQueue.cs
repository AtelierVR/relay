namespace Relay.Priority;

public class PriorityQueue<T>(IComparer<T> comparer, int maxSize = 10000) where T : class {
	private readonly SortedSet<T> _queue   = new(comparer);
	private readonly object       _lock    = new();
	private readonly int          _maxSize = maxSize;

	public int Count {
		get {
			lock (_lock) {
				return _queue.Count;
			}
		}
	}

	public bool TryEnqueue(T item) {
		lock (_lock) {
			if (_queue.Count < _maxSize)
				return _queue.Add(item);

			var last = _queue.Max;
			if (last == null)
				return _queue.Add(item);

			_queue.Remove(last);
			Logger.Warning("Priority queue full, dropped low priority packet");

			return _queue.Add(item);
		}
	}

	public bool TryDequeue(out T item) {
		lock (_lock) {
			item = null!;
			if (_queue.Count == 0)
				return false;

			var m = _queue.Min;
			item = m!;
			if (m == null)
				return false;

			_queue.Remove(m);
			return true;
		}
	}

	public void Clear() {
		lock (_lock) {
			_queue.Clear();
		}
	}
}