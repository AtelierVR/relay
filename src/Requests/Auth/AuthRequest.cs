namespace Relay.Requests.Auth;

public class AuthRequest
{
    public string access_token { get; set; }
    public string token_type { get; set; }
    public string ip { get; set; }
}