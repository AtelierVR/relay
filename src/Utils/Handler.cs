using System.Collections.Generic;
using Relay.Clients;

namespace Relay.Utils {
	public class Handler {
		private static readonly List<Handler> Handlers = [];

		protected virtual void OnSetup() { }

		public static void Listing() {
			var subclasses = typeof(Handler).Assembly.GetTypes();
			foreach (var subclass in subclasses)
				if (subclass.IsSubclassOf(typeof(Handler))) {
					Logger.Debug($"[Handler] Loading {subclass.FullName}");
					var instance = (Handler)Activator.CreateInstance(subclass)!;
					instance.OnSetup();
					Handlers.Add(instance);
				}
		}
	}
}