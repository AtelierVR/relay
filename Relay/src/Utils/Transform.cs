using System.Numerics;

namespace Relay.Utils
{
    public class Transform
    {
        public Vector3 position;
        public Quaternion rotation;
        public Vector3 scale;
        public Vector3 velocity;
        public Vector3 angularVelocity;
        
        public override string ToString() => $"Transform[position={position}; rotation={rotation}; scale={scale}; velocity={velocity}; angularVelocity={angularVelocity}]";
    }
}