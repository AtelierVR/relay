using System.Net;
using Relay.Clients;
using Relay.Packets;
using Relay.Priority;
using Relay.Utils;
using Buffer = Relay.Utils.Buffer;

namespace Relay.Requests;

public static class Request
{
    #region OnBuffer Interfaces

    private static readonly PriorityQueue<ReceivePacket> ReceiveQueue = new(Comparer<ReceivePacket>.Default, maxSize: 50000);

    public static void OnBuffer(Client client, Buffer buffer)
        => OnBuffer(client.Remote, buffer.ToBuffer());

    public static void OnBuffer(Remote remote, Buffer buffer)
        => OnBuffer(remote, buffer.ToBuffer());

    public static void OnBuffer(Client client, byte[] buffer)
        => OnBuffer(client.Remote, buffer);

    public static void OnBuffer(Remote remote, byte[] buf)
    {
        if (buf.Length < 5) return;
        var typ = (RequestType)buf[4];
        var pry = PacketPriorityManager.GetPriority(typ);
        var tsk = new ReceivePacket(remote, typ, buf, pry);
        if (Handling)
        {
            if (!ReceiveQueue.TryEnqueue(tsk))
                Logger.Warning($"[Request.OnBuffer] Failed to enqueue packet for {remote}");
        }
        else DirectReceive(tsk);
    }

    #endregion

    #region SendBuffer Interfaces

    private static readonly PriorityQueue<EmitterPacket> EmitterQueue = new(Comparer<EmitterPacket>.Default, maxSize: 50000);

    public static void SendBuffer(Client client, Buffer buffer, ResponseType type, ushort uid = 0, EPriority priority = EPriority.Normal)
        => SendBuffer(client.Remote, buffer.ToBuffer(), type, uid, priority);

    public static void SendBuffer(Remote remote, Buffer buffer, ResponseType type, ushort uid = 0, EPriority priority = EPriority.Normal)
        => SendBuffer(remote, buffer.ToBuffer(), type, uid, priority);

    public static void SendBuffer(Client client, byte[] buffer, ResponseType type, ushort uid = 0, EPriority priority = EPriority.Normal)
        => SendBuffer(client.Remote, buffer, type, uid, priority);

    public static void SendBuffer(Remote remote, byte[] buffer, ResponseType type, ushort uid = 0, EPriority priority = EPriority.Normal)
    {
        var buf = Buffer.New(Math.Max(buffer.Length, Constants.MaxPacketSize));
        buf.Write((ushort)(buffer.Length + 5));
        buf.Write(uid);
        buf.Write(type);
        buf.Write(buffer);
        var tsk = new EmitterPacket(remote, buf.ToBuffer(), priority);
        if (Handling)
        {
            if (!EmitterQueue.TryEnqueue(tsk))
                Logger.Warning($"[Request.SendBuffer] Failed to enqueue packet for {remote}");
        }
        else DirectSend(tsk);
    }

    #endregion

    #region Handler Loops

    private static CancellationTokenSource? _cts;

    public static bool Handling = false;

    public static void Start()
    {
        Logger.Debug("[Request] Starting request handler...");

        _cts = new CancellationTokenSource();
        var token = _cts.Token;

        var cpus = Environment.ProcessorCount / 4;
        cpus = Math.Max(cpus, 1);

        if (Handling)
        {

            Logger.Debug($"[Request] Starting {cpus} request handler(s)...");

            for (var i = 0; i < cpus; i++)
                Task.Run(
                    async () =>
                    {
                        while (!token.IsCancellationRequested)
                        {
                            try
                            {
                                Emit();
                            }
                            catch (Exception ex)
                            {
                                Logger.Error($"[Request.Emit] {ex}");
                            }

                            await Task.Delay(1, token);
                        }

                        Logger.Debug($"[Request] Stopped emitting.");
                    }, token);

            for (var i = 0; i < cpus; i++)
                Task.Run(
                    async () =>
                    {
                        while (!token.IsCancellationRequested)
                        {
                            try
                            {
                                Receive();
                            }
                            catch (Exception ex)
                            {
                                Logger.Error($"[Request.Receive] {ex}");
                            }

                            await Task.Delay(1, token);
                        }

                        Logger.Debug("[Request] Stopped receiving.");
                    }, token
                );
        }
        else Logger.Debug("[Request] Skipping request handlers...");

        Task.Run(
            async () =>
            {
                while (!token.IsCancellationRequested)
                {
                    Timeout();
                    await Task.Delay(1000, token);
                }

                Logger.Debug("[Request] Stopped timing out.");
            }, token
        );
    }

    public static void Stop()
    {
        _cts?.Cancel();
        _cts?.Dispose();
        _cts = null;
    }


    private static void Emit()
    {
        var t0 = DateTime.UtcNow;
        while (true)
        {
            if ((DateTime.UtcNow - t0).TotalMilliseconds > Constants.MaxEmitTimeMs)
            {
                Logger.Warning("Emitter queue is too long, dropping packets...");
                ReceiveQueue.Clear();
                break;
            }
            if (!EmitterQueue.TryDequeue(out var tsk)) break;

            DirectSend(tsk);
        }
    }

    private static void DirectSend(EmitterPacket tsk)
    {
        tsk.Remote.Send(tsk.Buffer, tsk.Buffer.Length);
    }

    private static void DirectReceive(ReceivePacket tsk)
    {
        var client = ClientManager.Get(tsk.Remote);
        if (client == null)
        {
            client = new Client(tsk.Remote);
            client.OnConnect();
        }

        client.LastSeen = DateTime.UtcNow;

        var handlers = PacketDispatcher.GetHandlers(tsk.Type);
        if (handlers.Length == 0)
        {
            Logger.Debug($"[Request.Receive] No handlers found for {tsk.Type}");
            return;
        }

        var buf = new Buffer(tsk.Payload);
        var len = buf.ReadUShort();
        var uid = buf.ReadUShort();
        var typ = buf.ReadEnum<RequestType>();

        var data = new PacketData(len, uid, typ, buf, client);

        foreach (var handler in handlers)
        {
            data.Payload.Goto(5);
            handler(data);
        }
    }

    private static void Receive()
    {
        var t0 = DateTime.UtcNow;
        while (true)
        {
            if ((DateTime.UtcNow - t0).TotalMilliseconds > Constants.MaxReceiveTimeMs)
            {
                Logger.Warning("Receive queue is too long, dropping packets...");
                ReceiveQueue.Clear();
                break;
            }

            if (!ReceiveQueue.TryDequeue(out var tsk)) break;

            DirectReceive(tsk);
        }
    }

    private static void Timeout()
    {
        var now = DateTime.UtcNow;
        var timeout = Config.Load().GetConnectionTimeout();
        foreach (var client in ClientManager.Clients.ToArray())
            if ((now - client.LastSeen).TotalSeconds >= timeout)
                client.Timeout();
    }

    #endregion
}