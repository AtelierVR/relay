using Relay.Clients;
using Relay.Players;
using Relay.Utils;

namespace Relay.Instances;

public class Instance {
	public readonly byte              InternalId;
	public          uint              MasterId;
	public          InstanceFlags     Flags;
	public          ushort            Capacity  = 0;
	public          byte              Tps       = 24;
	public          float             Threshold = 0.001f;
	private         List<UserModerated> _modereds = [];
	public          World?            World;
	public          List<Player>      Players    = new();
	public          List<ViewGroup>   ViewGroups = new();

	public UserModerated? GetModerated(uint userId, string address)
		=> _modereds.FirstOrDefault(m => m.UserId == userId && m.Address == address);

	public UserModerated? GetModerated(User user)
		=> GetModerated(user.Id, user.Address);

	#region ViewGroup Management

	/// <summary>
	/// Récupère ou crée un groupe automatique pour un utilisateur dans cette instance
	/// </summary>
	public ViewGroup GetOrCreateViewGroup(ushort playerId) {
		var existingGroup = ViewGroups.FirstOrDefault(g => g.Id == playerId);
		if (existingGroup != null)
			return existingGroup;
		var group = new ViewGroup(playerId, $"User_{playerId}");
		ViewGroups.Add(group);
		group.AddMember(playerId);
		group.AddVisibleGroup(playerId);
		return group;
	}

	/// <summary>
	/// Crée un nouveau groupe personnalisé dans cette instance
	/// </summary>
	public ViewGroup CreateCustomGroup(string? name = null) {
		var group = new ViewGroup(GetNextViewGroupId(), name);
		ViewGroups.Add(group);
		return group;
	}

	/// <summary>
	/// Récupère un groupe par son ID dans cette instance
	/// </summary>
	public ViewGroup? GetViewGroup(uint groupId)
		=> ViewGroups.FirstOrDefault(g => g.Id == groupId);

	/// <summary>
	/// Supprime un groupe personnalisé de cette instance
	/// </summary>
	public bool RemoveViewGroup(uint groupId) {
		if (groupId <= ushort.MaxValue)
			return false; // Ne peut pas supprimer un groupe automatique

		var group = ViewGroups.FirstOrDefault(g => g.Id == groupId);
		if (group != null && ViewGroups.Remove(group)) {
			// Nettoyer les références à ce groupe dans les autres groupes
			foreach (var otherGroup in ViewGroups)
				otherGroup.RemoveVisibleGroup(groupId);
			return true;
		}

		return false;
	}

	public IEnumerable<uint> GetUserGroups(ushort playerId)
		=> ViewGroups.Where(g => g.IsMember(playerId))
			.Select(g => g.Id);


	/// <summary>
	/// Vérifie si un utilisateur peut voir un autre utilisateur dans cette instance
	/// </summary>
	public bool CanUserSeeUser(ushort viewerId, ushort targetId) {
		if (viewerId == targetId)
			return true;
		var viewerGroups = GetUserGroups(viewerId).ToList();
		var targetGroups = GetUserGroups(targetId).ToList();
		foreach (var viewerGroupId in viewerGroups) {
			var viewerGroup = GetViewGroup(viewerGroupId);
			if (viewerGroup != null && targetGroups.Any(gtid => viewerGroup.CanSeeGroup(gtid)))
				return true;
		}

		return false;
	}


	/// <summary>
	/// Nettoie les données d'un utilisateur dans cette instance
	/// </summary>
	public void CleanupUser(ushort playerId) {
		foreach (var group in ViewGroups)
			group.RemoveMember(playerId);
		var autoGroup = ViewGroups.FirstOrDefault(g => g.Id == playerId);
		if (autoGroup != null && autoGroup.Members.Count == 0)
			ViewGroups.Remove(autoGroup);
	}

	#endregion

	private string? _password;

	public string? Password {
		get => Flags.HasFlag(InstanceFlags.UsePassword) ? _password : null;
		set {
			if (string.IsNullOrEmpty(value)) {
				Flags     &= ~InstanceFlags.UsePassword;
				_password =  null;
			} else {
				Flags     |= InstanceFlags.UsePassword;
				_password =  value;
			}
		}
	}

	public bool VerifyPassword(string password)
		=> string.IsNullOrEmpty(Password) || (!string.IsNullOrEmpty(password) && Hashing.Verify(password, Password));

	public Instance() {
		Flags      = InstanceFlags.None;
		InternalId = InstanceManager.GetNextInternalId();
		InstanceManager.Add(this);
	}

	public override string ToString()
		=> $"{GetType().Name}[InternalId={InternalId}, MasterId={MasterId}, PlayerCount={Players.Count}/{Capacity}]";

	internal ushort GetNextPlayerId() {
		ushort id = 0;
		while (Players.Exists(p => p.Id == id))
			id++;
		return id;
	}

	internal uint GetNextViewGroupId() {
		uint id = ushort.MaxValue + 1;
		while (ViewGroups.Exists(g => g.Id == id))
			id++;
		return id;
	}

	public bool IsFull()
		=> Capacity != 0 && Players.Count >= Capacity && !Flags.HasFlag(InstanceFlags.AllowOverload);
}