using System;

namespace Relay.Master;

public class MasterResponse<T>
{
    public T data { get; set; }
    public ulong time { get; set; }
    public string request { get; set; }
    public MasterResponseError error { get; set; }

    public DateTimeOffset GetTime() 
        => DateTimeOffset.FromUnixTimeMilliseconds((long)time);

    public bool HasError() 
        => error is { code: > 0 };

    public override string ToString() 
        => $"{GetType().Name}[{(HasError() ? $"error={error}" : $"data={data}, time={GetTime()}, request={request}")}]";
}

public class MasterResponseError
{
    public string message { get; set; }
    public ushort code { get; set; }
    public ushort status { get; set; }
    
    public override string ToString() 
        => $"{GetType().Name}[message={message}, code={code}, status={status}]";
}