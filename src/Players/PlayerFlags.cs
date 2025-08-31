using System;

namespace Relay.Players;

[Flags]
public enum PlayerFlags : uint {
	None = 0,

	// Indiquate if the player is a bot
	IsBot = 1 << 0,

	// Player is the reference player for the instance
	InstanceMaster = 1 << 1,

	// Is the manual added moderator of the instance
	InstanceModerator = 1 << 2,

	// Is the creator of the instance
	InstanceOwner = 1 << 3,

	// Player is a moderator in the guild who owns the instance
	GuildModerator = 1 << 4,

	// Player is ta moderator in the master node who owns the instance
	MasterModerator = 1 << 5,

	// Player is the owner of the world
	WorldOwner = 1 << 6,

	// Player is a moderator in the world
	WorldModerator = 1 << 7,

	// Player is authenticated and verified
	AuthUnverified = 1 << 8,

	// Player is hidden in motded lists
	HideInList = 1 << 9,

	// Player has a custom display name
	HasCustomDisplay = 1 << 10,

	HasPrivilege = InstanceMaster | InstanceModerator | InstanceOwner | GuildModerator | MasterModerator | WorldOwner | WorldModerator,

	HasHigthPrivilege = InstanceOwner | MasterModerator
}