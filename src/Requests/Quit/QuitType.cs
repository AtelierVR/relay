namespace Relay.Requests.Instances.Quit;

[Flags]
public enum QuitType : byte
{
    Normal = 0,
    Timeout = 1,
    ModerationKick = 2,
    VoteKick = 3,
    ConfigurationError = 4,
    UnknownError = 5,

    ModerationAction = ModerationKick | VoteKick
}