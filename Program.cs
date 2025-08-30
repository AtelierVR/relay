using Relay.Master;
using Relay.Requests;
using Relay.Utils;
using Relay.LoadBalancing;

namespace Relay
{
    internal class MainProcess
    {
        public static void Main(string[] args)
        {
            Logger.Log("Starting NoxRelay...");
            if (Logger.PrintDebug)
                Logger.Warning("Debug mode enabled");

            // Initialize BufferPool
            BufferPool.Instance.Preload(100);
            Logger.Log("BufferPool initialized and preloaded");

            // Initialize LoadBalancer
            var loadBalancerManager = LoadBalancerManager.Instance;
            Logger.Log("LoadBalancer initialized");

            // Starting the server
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

            Logger.Log("All worker threads started");

            // Waiting for the threads to finish
            thread1.Join();
            thread2.Join();
            thread3.Join();
            thread4.Join();
        }

        static void WorkerUdpRecv()
            => UdpRecv.Start();

        static void WorkerNetCheck()
            => Request.Check();

        static void WorkerMaster()
            => MasterServer.Start();

        static void WorkerTcpRecv()
            => TCPRecv.Start();
    }
}