using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Linq;
using Relay.Clients;

namespace Relay.Utils
{
    public class MultiPacketSession
    {
        public ushort SessionId { get; set; }
        public ushort TotalPackets { get; set; }
        public uint TotalSize { get; set; }
        public Dictionary<ushort, byte[]> Packets { get; set; } = new Dictionary<ushort, byte[]>();
        public DateTime CreatedAt { get; set; } = DateTime.UtcNow;
        public ResponseType OriginalType { get; set; }
        public ushort OriginalUid { get; set; }
        
        public bool IsComplete => Packets.Count == TotalPackets;
        
        public byte[] GetMergedData()
        {
            if (!IsComplete) return null;
            
            var result = new byte[TotalSize];
            uint offset = 0;
            
            for (ushort i = 0; i < TotalPackets; i++)
            {
                if (Packets.TryGetValue(i, out var packetData))
                {
                    Array.Copy(packetData, 0, result, offset, packetData.Length);
                    offset += (uint)packetData.Length;
                }
                else
                {
                    return null; // Missing packet
                }
            }
            
            return result;
        }
    }
    
    public static class MultiPacketManager
    {
        private static readonly ConcurrentDictionary<string, MultiPacketSession> Sessions = new();
        private static readonly object CleanupLock = new object();
        private static DateTime LastCleanup = DateTime.UtcNow;
        private const int SessionTimeoutSeconds = 30;
        private static int MaxPacketSize => Constants.MaxFragmentSize; // Use fragment size for multipackets
        
        public static string GetSessionKey(Client client, ushort sessionId)
        {
            return $"{client.Id}_{sessionId}";
        }
        
        public static void CleanupExpiredSessions()
        {
            lock (CleanupLock)
            {
                if (DateTime.UtcNow.Subtract(LastCleanup).TotalSeconds < 5) return;
                LastCleanup = DateTime.UtcNow;
                
                var expiredKeys = Sessions
                    .Where(kvp => DateTime.UtcNow.Subtract(kvp.Value.CreatedAt).TotalSeconds > SessionTimeoutSeconds)
                    .Select(kvp => kvp.Key)
                    .ToList();
                
                foreach (var key in expiredKeys)
                {
                    Sessions.TryRemove(key, out _);
                    Logger.Warning($"Multipacket session {key} expired and was removed");
                }
            }
        }
        
        public static void StartSession(Client client, ushort sessionId, ushort totalPackets, uint totalSize, ResponseType originalType, ushort originalUid)
        {
            CleanupExpiredSessions();
            
            var key = GetSessionKey(client, sessionId);
            var session = new MultiPacketSession
            {
                SessionId = sessionId,
                TotalPackets = totalPackets,
                TotalSize = totalSize,
                OriginalType = originalType,
                OriginalUid = originalUid
            };
            
            Sessions[key] = session;
            Logger.Debug($"Started multipacket session {key} with {totalPackets} packets, total size {totalSize}");
        }
        
        public static bool AddPacket(Client client, ushort sessionId, ushort packetIndex, byte[] data)
        {
            var key = GetSessionKey(client, sessionId);
            if (!Sessions.TryGetValue(key, out var session))
            {
                Logger.Warning($"Received packet for unknown session {key}");
                return false;
            }
            
            if (packetIndex >= session.TotalPackets)
            {
                Logger.Warning($"Invalid packet index {packetIndex} for session {key} (max: {session.TotalPackets - 1})");
                return false;
            }
            
            session.Packets[packetIndex] = data;
            Logger.Debug($"Added packet {packetIndex}/{session.TotalPackets - 1} to session {key}");
            
            return true;
        }
        
        public static MultiPacketSession CompleteSession(Client client, ushort sessionId)
        {
            var key = GetSessionKey(client, sessionId);
            if (!Sessions.TryRemove(key, out var session))
            {
                Logger.Warning($"Cannot complete unknown session {key}");
                return null;
            }
            
            if (!session.IsComplete)
            {
                Logger.Warning($"Attempted to complete incomplete session {key} ({session.Packets.Count}/{session.TotalPackets})");
                return null;
            }
            
            Logger.Debug($"Completed multipacket session {key}");
            return session;
        }
        
        public static List<Buffer> FragmentData(byte[] data, ResponseType type, ushort uid)
        {
            if (data.Length <= MaxPacketSize)
            {
                // No need to fragment
                return new List<Buffer>();
            }
            
            var fragments = new List<Buffer>();
            var totalPackets = (ushort)Math.Ceiling((double)data.Length / MaxPacketSize);
            var sessionId = GenerateSessionId();
            
            // Start packet
            var startBuffer = new Buffer();
            startBuffer.Write(sessionId);
            startBuffer.Write(totalPackets);
            startBuffer.Write((uint)data.Length);
            startBuffer.Write(type);
            startBuffer.Write(uid);
            fragments.Add(startBuffer);
            
            // Data packets
            for (ushort i = 0; i < totalPackets; i++)
            {
                var offset = i * MaxPacketSize;
                var length = Math.Min(MaxPacketSize, data.Length - offset);
                var packetData = new byte[length];
                Array.Copy(data, offset, packetData, 0, length);
                
                var dataBuffer = new Buffer();
                dataBuffer.Write(sessionId);
                dataBuffer.Write(i);
                dataBuffer.Write((ushort)length);
                dataBuffer.Write(packetData);
                fragments.Add(dataBuffer);
            }
            
            // End packet
            var endBuffer = new Buffer();
            endBuffer.Write(sessionId);
            fragments.Add(endBuffer);
            
            return fragments;
        }
        
        private static ushort _nextSessionId = 1;
        private static readonly object SessionIdLock = new object();
        
        private static ushort GenerateSessionId()
        {
            lock (SessionIdLock)
            {
                return _nextSessionId++;
            }
        }
    }
}
