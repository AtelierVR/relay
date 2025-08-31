using System.Net;

namespace Relay.Utils;

public abstract class Remote {
	public virtual IPAddress Address
		=> IPAddress.Any;

	public virtual ushort Port
		=> 0;

	public abstract bool Send(byte[]   data, int length);
	public abstract bool Equals(Remote obj);

	public override int GetHashCode()
		=> HashCode.Combine(Address, Port);

	public override string ToString()
		=> $"{GetType().Name}[Address={Address}, Port={Port}]";
}