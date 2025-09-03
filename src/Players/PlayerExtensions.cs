namespace Relay.Players;

public static class PlayerExtensions
{
    public static bool IsReady(this Player player)
        => player is { Status: > PlayerStatus.Preparing };

    public static bool IsAllowed(this Player player, Player target)
        => player != null
            && target != null
            && player.InstanceId == target.InstanceId
            && player.HasPrivilege;
}