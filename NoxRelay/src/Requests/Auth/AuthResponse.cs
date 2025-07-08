using System;

namespace Relay.Requests.Auth;

public class AuthResponse
{
    public bool valid { get; set; }
    public bool is_blacklisted { get; set; }
    public AuthBlacklistedResponse blacklisted { get; set; }

    public bool is_invalid_token { get; set; }
    public AuthUserResponse user { get; set; }
    public string token_type { get; set; }
    public string access_token { get; set; }

    public override string ToString() =>
        $"{GetType().Name}[valid={valid}, is_blacklisted={is_blacklisted}, is_invalid_token={is_invalid_token}, user={user}]";
}

public class AuthUserResponse
{
    public uint id { get; set; }
    public string server { get; set; }
    public string display { get; set; }

    public override string ToString() => $"{GetType().Name}[id={id}, server={server}, display={display}]";
}

public class AuthBlacklistedResponse
{
    public ulong expires { get; set; }
    public string reason { get; set; }

    public DateTimeOffset ExprireAt 
        => DateTimeOffset.FromUnixTimeMilliseconds((long)expires);

    public override string ToString() 
        => $"{GetType().Name}[exprire={ExprireAt}, reason={reason}]";
}