using Relay.Instances;
using Relay.Utils;
using Buffer = Relay.Utils.Buffer;
namespace Relay.Requests.Instances.ServerConfig;

/// <summary>
/// Classe utilitaire pour envoyer des mises à jour de configuration du serveur
/// </summary>
public static class ServerConfigHelper
{
    /// <summary>
    /// Met à jour le TPS d'une instance et notifie tous les joueurs
    /// </summary>
    public static void UpdateTps(Instance instance, byte newTps)
    {
        if (instance == null) return;

        var oldTps = instance.Tps;
        instance.Tps = newTps;
        Logger.Log($"TPS updated from {oldTps} to {newTps} for {instance}");

        ServerConfigHandler.BroadcastConfigUpdate(instance, ServerConfigFlags.Tps);
    }

    /// <summary>
    /// Met à jour le Threshold d'une instance et notifie tous les joueurs
    /// </summary>
    public static void UpdateThreshold(Instance instance, float newThreshold)
    {
        if (instance == null) return;

        var oldThreshold = instance.Threshold;
        instance.Threshold = newThreshold;
        Logger.Log($"Threshold updated from {oldThreshold} to {newThreshold} for {instance}");

        ServerConfigHandler.BroadcastConfigUpdate(instance, ServerConfigFlags.Threshold);
    }

    /// <summary>
    /// Met à jour la capacité d'une instance et notifie tous les joueurs
    /// </summary>
    public static void UpdateCapacity(Instance instance, ushort newCapacity)
    {
        if (instance == null) return;

        var oldCapacity = instance.Capacity;
        instance.Capacity = newCapacity;
        Logger.Log($"Capacity updated from {oldCapacity} to {newCapacity} for {instance}");

        ServerConfigHandler.BroadcastConfigUpdate(instance, ServerConfigFlags.Capacity);
    }

    /// <summary>
    /// Met à jour le mot de passe d'une instance et notifie tous les joueurs
    /// </summary>
    public static void UpdatePassword(Instance instance, string? newPassword)
    {
        if (instance == null) return;

        instance.Password = newPassword ?? string.Empty;
        Logger.Log($"Password updated for {instance}");

        ServerConfigHandler.BroadcastConfigUpdate(instance, ServerConfigFlags.Password);
    }

    /// <summary>
    /// Met à jour les flags d'une instance et notifie tous les joueurs
    /// </summary>
    public static void UpdateFlags(Instance instance, InstanceFlags newFlags)
    {
        if (instance == null) return;

        var oldFlags = instance.Flags;
        instance.Flags = newFlags;
        Logger.Log($"Flags updated from {oldFlags} to {newFlags} for {instance}");

        ServerConfigHandler.BroadcastConfigUpdate(instance, ServerConfigFlags.Flags);
    }
}
