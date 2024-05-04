namespace Relay.Requests.Auth.Bearer;

public class AuthBearerRequest
{
    public string access_token { get; set; }
    public uint user_id { get; set; }
    public string ip { get; set; }
}