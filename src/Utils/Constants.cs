namespace Relay.Utils
{
	public class Constants
	{
		public const ushort ProtocolVersion = 1;
		public const ushort MaxPacketSize = 1024;
		public const ushort Port = 23032;
		public const string UseAddress = "127.0.0.1:23032";
		public const string MasterGateway = "http://127.0.0.1:53032";
		public const ushort ConnectionTimeout = 15;
		public const ushort KeepAliveInterval = 5;
		public const ushort SegmentationTimeout = 30; // seconds
		public const byte MaxInstances = 3;
		public const ushort MinUpdateTime = 100;
		public const ushort DefaultUpdateTime = 100;
		public const double MaxEmitTimeMs = 1000d;
		public const double MaxReceiveTimeMs = 50d;
		public const bool Debug = true;
	}

	public enum RequestType : byte
	{
		None = 0xFF,

		// System Messages
		Disconnect = 0x00,
		Handshake = 0x01,
		Segmentation = 0x02,
		Reliable = 0x03,
		Latency = 0x04,

		Authentification = 0x05,
		Enter = 0x06,
		Quit = 0x07,
		Custom = 0x08,
		PasswordRequirement = 0x09,
		Traveling = 0x0A,
		Transform = 0x0B,
		Teleport = 0x0C,
		AvatarChanged = 0x0D,
		ServerConfig = 0x0E,
		AvatarParams = 0x0F,
		Status = 0x11,
	}

	public enum ResponseType : byte
	{
		None = 0xFF,

		// System Messages
		Disconnect = 0x00,
		Handshake = 0x01,
		Segmentation = 0x02,
		Reliable = 0x03,
		Latency = 0x04,

		Authentification = 0x05,
		Enter = 0x06,
		Quit = 0x07,
		Custom = 0x08,
		PasswordRequirement = 0x09,
		Traveling = 0x0A,
		Transform = 0x0B,
		Teleport = 0x0C,
		AvatarChanged = 0x0D,
		ServerConfig = 0x0E,
		AvatarParams = 0x0F,
		Join = 0x10,
		Leave = 0x11,
		Status = 0x12,
	}

	public class Messages
	{
		public const string IncompatibleProtocol = "Incompatible protocol version ({0} != {1}).";
		public const string ConnectionTimeout = "Connection timed out.";
		public const string ConnectionInstanceAsBotRefused = "Bots are not allowed in this instance.";
		public const string InstanceNotFound = "Instance not found.";
		public const string AlreadyInInstance = "Already in an instance.";
		public const string InstanceIsFull = "Instance is full.";
		public const string YouAreNotWhitelisted = "You are not whitelisted.";
		public const string IncorrectPassword = "Incorrect password.";
		public const string YouAreNotInInstance = "You are not in this instance.";
	}
}