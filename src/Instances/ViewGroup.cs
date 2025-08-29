using Relay.Clients;

namespace Relay.Instances;

/// <summary>
/// Représente un groupe de vue pour la gestion de la visibilité entre utilisateurs
/// </summary>
public class ViewGroup
{
    /// <summary>
    /// ID unique du groupe
    /// - [0 - ushort.MaxValue] : groupes automatiques basés sur l'ID utilisateur
    /// - [ushort.MaxValue + 1 et plus] : groupes personnalisés
    /// </summary>
    public uint Id { get; }

    /// <summary>
    /// Nom du groupe (optionnel pour les groupes personnalisés)
    /// </summary>
    public string? Name { get; set; }

    /// <summary>
    /// Liste des IDs des groupes que ce groupe peut voir
    /// </summary>
    public HashSet<uint> VisibleGroups { get; } = new();

    /// <summary>
    /// Liste des utilisateurs appartenant à ce groupe
    /// </summary>
    public HashSet<ushort> Members { get; } = new();

    /// <summary>
    /// Indique si c'est un groupe automatique (basé sur l'ID utilisateur)
    /// </summary>
    public bool IsAutoGroup
        => Id <= ushort.MaxValue;

    /// <summary>
    /// Indique si c'est un groupe personnalisé
    /// </summary>
    public bool IsCustomGroup
        => Id > ushort.MaxValue;

    public ViewGroup(uint id, string? name = null)
    {
        Id = id;
        Name = name;
    }

    /// <summary>
    /// Ajoute un utilisateur au groupe
    /// </summary>
    public bool AddMember(ushort playerId)
        => Members.Add(playerId);

    /// <summary>
    /// Retire un utilisateur du groupe
    /// </summary>
    public bool RemoveMember(ushort playerId)
        => Members.Remove(playerId);
    

    /// <summary>
    /// Vérifie si un utilisateur est membre de ce groupe
    /// </summary>
    public bool IsMember(ushort playerId)
        => Members.Contains(playerId);

    /// <summary>
    /// Ajoute un groupe à la liste des groupes visibles
    /// </summary>
    public bool AddVisibleGroup(uint groupId)
        => VisibleGroups.Add(groupId);

    /// <summary>
    /// Retire un groupe de la liste des groupes visibles
    /// </summary>
    public bool RemoveVisibleGroup(uint groupId)
        => VisibleGroups.Remove(groupId);

    /// <summary>
    /// Vérifie si ce groupe peut voir un autre groupe
    /// </summary>
    public bool CanSeeGroup(uint groupId)
        => VisibleGroups.Contains(groupId);

    public override string ToString()
    {
        var type = IsAutoGroup ? "Auto" : "Custom";
        var name = !string.IsNullOrEmpty(Name) ? $" '{Name}'" : "";
        return $"{GetType().Name}<{type}>[Id={Id}{name}, Members={Members.Count}, VisibleGroups={VisibleGroups.Count}]";
    }
}
