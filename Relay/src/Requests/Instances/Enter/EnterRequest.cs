namespace Relay.Requests.Instances.Enter;

public class EnterRequest
{
    public uint instance_id { get; set; }
    public string ip { get; set; }
    public uint user_id { get; set; }
    public string display { get; set; }
}