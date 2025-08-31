namespace Relay.Instances;

public class InstanceManager
{
    public static readonly List<Instance> Instances = [];

    public static Instance? Get(byte internalId)
        => Instances.Find(instance => instance.InternalId == internalId);

    public static Instance? Get(uint masterId)
        => Instances.Find(instance => instance.MasterId == masterId);

    public static void Add(Instance instance)
        => Instances.Add(instance);

    public static bool Has(ushort internalId)
        => Instances.Exists(instance => instance.InternalId == internalId);

    public static void Remove(Instance instance)
        => Instances.Remove(instance);

    public static byte GetNextInternalId()
    {
        byte internalId = 0;
        while (Has(internalId))
            internalId++;
        return internalId;
    }

    public static List<Instance> GetAllInstances()
        => Instances;
}