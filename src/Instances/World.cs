namespace Relay.Instances;

public class World(uint masterId, string address, ushort version) {
	public readonly uint   MasterId = masterId;
	public readonly ushort Version  = version;
	public readonly string Address  = address;
}