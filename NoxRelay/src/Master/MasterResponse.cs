using System;

namespace Relay.Master;

public class MasterResponse<T>
{
    public T data { get; set; }
    public ulong time { get; set; }
    public string request { get; set; }
    public MasterResponseError error { get; set; }
    public DateTime GetTime() => new DateTime(1970, 1, 1, 0, 0, 0, DateTimeKind.Utc).AddMilliseconds(time);
    public bool HasError() => error is { status: > 0 };
}

public class MasterResponseError
{
    public string message { get; set; }
    public ushort code { get; set; }
    public ushort status { get; set; }
    
    public override string ToString() => $"{GetType().Name}[message={message}, code={code}, status={status}]";
}