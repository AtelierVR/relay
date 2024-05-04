using System.Collections.Generic;

namespace Relay.Instances;

public class InstanceManager
{
    public static List<Instance> Instances = new();

    public static Instance Get(ushort internalId) => Instances.Find(instance => instance.InternalId == internalId);
    public static Instance Get(uint masterId) => Instances.Find(instance => instance.MasterId == masterId);
    public static void Add(Instance instance) => Instances.Add(instance);
    public static bool Has(ushort internalId) => Instances.Exists(instance => instance.InternalId == internalId);

    public static void Remove(Instance instance)
    {
        if (Instances.Contains(instance))
            Instances.Remove(instance);
    }

    public static ushort GetNextInternalId()
    {
        ushort internalId = 0;
        while (Has(internalId))
            internalId++;
        return internalId;
    }
}