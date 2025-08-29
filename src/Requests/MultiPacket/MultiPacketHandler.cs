using System;
using Relay.Clients;
using Relay.Utils;
using Buffer = Relay.Utils.Buffer;

namespace Relay.Requests.MultiPacket
{
    public class MultiPacketHandler : Handler
    {
        public override void OnReceive(Buffer buffer, Client client)
        {
            buffer.Goto(2); // Skip length
            var uid = buffer.ReadUShort();
            var type = buffer.ReadEnum<RequestType>();

            switch (type)
            {
                case RequestType.MultiPacketStart:
                    HandleMultiPacketStart(buffer, client, uid);
                    break;
                case RequestType.MultiPacketData:
                    HandleMultiPacketData(buffer, client, uid);
                    break;
                case RequestType.MultiPacketEnd:
                    HandleMultiPacketEnd(buffer, client, uid);
                    break;
            }
        }

        private void HandleMultiPacketStart(Buffer buffer, Client client, ushort uid)
        {
            var sessionId = buffer.ReadUShort();
            var totalPackets = buffer.ReadUShort();
            var totalSize = buffer.ReadUInt();
            var originalType = buffer.ReadEnum<ResponseType>();
            var originalUid = buffer.ReadUShort();

            MultiPacketManager.StartSession(client, sessionId, totalPackets, totalSize, originalType, originalUid);

            Logger.Debug($"Started multipacket session {sessionId} for {client} with {totalPackets} packets, size {totalSize}");

            // Send acknowledgment
            var response = new Buffer();
            response.Write(sessionId);
            Request.SendBuffer(client, response, ResponseType.MultiPacketStart, uid);
        }

        private void HandleMultiPacketData(Buffer buffer, Client client, ushort uid)
        {
            var sessionId = buffer.ReadUShort();
            var packetIndex = buffer.ReadUShort();
            var dataLength = buffer.ReadUShort();
            var data = buffer.ReadBytes(dataLength);

            if (data == null)
            {
                Logger.Warning($"Failed to read packet data for session {sessionId}, packet {packetIndex}");
                return;
            }

            var success = MultiPacketManager.AddPacket(client, sessionId, packetIndex, data);

            // Send acknowledgment
            var response = new Buffer();
            response.Write(sessionId);
            response.Write(packetIndex);
            response.Write((byte)(success ? 1 : 0));
            Request.SendBuffer(client, response, ResponseType.MultiPacketData, uid);
        }

        private void HandleMultiPacketEnd(Buffer buffer, Client client, ushort uid)
        {
            var sessionId = buffer.ReadUShort();
            var session = MultiPacketManager.CompleteSession(client, sessionId);

            if (session == null)
            {
                Logger.Warning($"Failed to complete multipacket session {sessionId} for {client}");
                
                // Send failure response
                var failResponse = new Buffer();
                failResponse.Write(sessionId);
                failResponse.Write((byte)0);
                Request.SendBuffer(client, failResponse, ResponseType.MultiPacketEnd, uid);
                return;
            }

            var mergedData = session.GetMergedData();
            if (mergedData == null)
            {
                Logger.Error($"Failed to merge data for session {sessionId}");
                
                // Send failure response
                var failResponse = new Buffer();
                failResponse.Write(sessionId);
                failResponse.Write((byte)0);
                Request.SendBuffer(client, failResponse, ResponseType.MultiPacketEnd, uid);
                return;
            }

            Logger.Debug($"Successfully merged {mergedData.Length} bytes for session {sessionId}");

            // Process the merged packet as if it was a regular packet
            var mergedBuffer = new Buffer();
            mergedBuffer.Write((ushort)(mergedData.Length + 4)); // Length including header
            mergedBuffer.Write(session.OriginalUid);
            mergedBuffer.Write(session.OriginalType);
            mergedBuffer.Write(mergedData);

            // Process through normal handlers
            mergedBuffer.Goto(0);
            foreach (var handler in Handler.Handlers)
            {
                if (handler is MultiPacketHandler) continue; // Skip self to avoid recursion
                handler.OnReceive(mergedBuffer, client);
            }

            // Send success response
            var response = new Buffer();
            response.Write(sessionId);
            response.Write((byte)1);
            Request.SendBuffer(client, response, ResponseType.MultiPacketEnd, uid);
        }
    }
}
