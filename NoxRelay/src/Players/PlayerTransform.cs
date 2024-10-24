using Relay.Utils;
using System.Collections;

namespace Relay.Players;

public class PlayerTransform
{
    Dictionary<ushort, Transform> transforms = new();

    public Dictionary<ushort, Transform> GetPairs() => transforms;
    public void Set(ushort part, Transform transform) => transforms[part] = transform;
    public Transform Get(ushort part) => transforms.TryGetValue(part, out Transform? transform) ? transform : new();
}
