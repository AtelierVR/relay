using System;
using System.Security.Cryptography;
using System.Text;
using System.Threading;
using Relay.Master;
using Relay.Requests;
using Relay.Utils;

namespace Relay
{
    internal class MainProcess
    {
        static UdpRecv _udpRecv;
        static MasterServer _master;
        static TCPRecv _tcpRecv;
        static Request _netRecv;

        public static void Main(string[] args)
        {
            _udpRecv = new UdpRecv();
            _tcpRecv = new TCPRecv();
            _netRecv = new Request();
            _master = new MasterServer();

            Handler.Listing();

            var thread1 = new Thread(WorkerUdpRecv);
            var thread2 = new Thread(WorkerNetCheck);
            var thread3 = new Thread(WorkerMaster);
            var thread4 = new Thread(WorkerTcpRecv);

            // Starting the threads
            thread1.Start();
            thread2.Start();
            thread3.Start();
            thread4.Start();

            // Waiting for the threads to finish
            thread1.Join();
            thread2.Join();
            thread3.Join();
            thread4.Join();
        }

        static void WorkerUdpRecv() => UdpRecv.Start();
        static void WorkerNetCheck() => Request.Check();
        static void WorkerMaster() => _master.Start();
        static void WorkerTcpRecv() => _tcpRecv.Start();
    }
}