using System;
using Relay.Clients;
using Relay.Utils;
using Relay.Requests;
using Buffer = Relay.Utils.Buffer;

namespace Relay.Tests
{
    public static class MultiPacketTest
    {
        public static void TestLargePacket(Client client)
        {
            Logger.Log("Testing large packet transmission...");
            
            // Create a large data buffer (2KB)
            var largeData = new byte[2048];
            for (int i = 0; i < largeData.Length; i++)
            {
                largeData[i] = (byte)(i % 256);
            }
            
            var buffer = new Buffer();
            buffer.Write(largeData);
            
            Logger.Log($"Created test packet of {largeData.Length} bytes");
            
            // This should trigger multipacket transmission
            var result = Request.SendBuffer(client, buffer, ResponseType.CustomDataPacket, 0x1234);
            
            if (result != ushort.MaxValue)
            {
                Logger.Log($"Large packet sent successfully with UID {result:X4}");
            }
            else
            {
                Logger.Error("Failed to send large packet");
            }
        }
        
        public static void TestMultipleFragments(Client client)
        {
            Logger.Log("Testing multiple fragment transmission...");
            
            // Create an extra large data buffer (5KB)
            var extraLargeData = new byte[5120];
            for (int i = 0; i < extraLargeData.Length; i++)
            {
                extraLargeData[i] = (byte)((i * 7 + 42) % 256); // Some pattern
            }
            
            var buffer = new Buffer();
            buffer.Write(extraLargeData);
            
            Logger.Log($"Created extra large test packet of {extraLargeData.Length} bytes");
            
            // This should trigger multipacket transmission with multiple fragments
            var result = Request.SendBuffer(client, buffer, ResponseType.Transform, 0x5678);
            
            if (result != ushort.MaxValue)
            {
                Logger.Log($"Extra large packet sent successfully with UID {result:X4}");
            }
            else
            {
                Logger.Error("Failed to send extra large packet");
            }
        }
    }
}
