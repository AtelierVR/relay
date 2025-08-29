using System.Collections.Generic;
using Relay.Clients;

namespace Relay.Utils
{
    public class Handler
    {
        public static List<Handler> Handlers = new List<Handler>();
        public static T Get<T>() where T : Handler => Handlers.Find(handler => handler is T) as T;

        public Handler()
        {
            Handlers.Add(this);
            Logger.Debug($"Handler {GetType().Name} registered");
        }

        public static void Listing()
        {
            var subclasses = typeof(Handler).Assembly.GetTypes();
            foreach (var subclass in subclasses)
                if (subclass.IsSubclassOf(typeof(Handler)))
                    subclass.GetConstructor(new System.Type[] { })?.Invoke(new object[] { });
        }

        public virtual void OnReceive(Buffer buffer, Client client)
        {
        }
    }
}