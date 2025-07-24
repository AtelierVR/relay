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
        public const byte MaxInstances = 3;
        public const bool PrintDebug = true;
        public const ushort MinUpdateTime = 100;
        public const ushort DefaultUpdateTime = 100;
    }

    public enum RequestType : byte
    {
        Disconnect = 0x00,
        Handshake = 0x01,
        Status = 0x02,
        Latency = 0x03,
        Authentification = 0x04,
        Enter = 0x05,
        Quit = 0x06,
        CustomDataPacket = 0x07,
        PasswordRequirement = 0x08,
        Traveling = 0x09,
        Transform = 0x0C,
        Teleport = 0x0D,
        None = 0xFF
    }

    public enum ResponseType : byte
    {
        Disconnect = 0x00,
        Handshake = 0x01,
        Status = 0x02,
        Latency = 0x03,
        Authentification = 0x04,
        Enter = 0x05,
        Quit = 0x06,
        CustomDataPacket = 0x07,
        Traveling = 0x09,
        Join = 0x0A,
        Leave = 0x0B,
        Transform = 0x0C,
        Teleport = 0x0D
    }

    public enum Platfrom : byte
    {
        None = 0,
        Windows = 1,
        Linux = 2,
        MacOS = 3,
        Android = 4,
        IOS = 5
    }

    public enum Engine : byte
    {
        None = 0,
        Unity = 1,
        Unreal = 2,
        Godot = 3,
        Source = 4
    }

    public class Messages
    {
        public const string IncompatibleProtocol = "Incompatible protocol version ({0} != {1}).";
        public const string ConnectionTimeout = "Connection timed out.";
        public const string ConnectionInstanceAsBotRefused = "Bots are not allowed in this instance.";
    }
}