using Relay.Master;
using Relay.Requests;
using Relay.Utils;

namespace Relay
{
	internal class MainProcess
	{
		private static readonly ManualResetEvent ShutdownEvent = new(false);

		public static void Main(string[] args)
		{
			Logger.Log("Starting NoxRelay...");
			if (Logger.PrintDebug)
				Logger.Warning("Debug mode enabled");

			Console.CancelKeyPress += OnCancelKeyPress;
			AppDomain.CurrentDomain.ProcessExit += OnProcessExit;

			Handler.Listing();

			MasterServer.Start();
			TcpReceiver.Start();
			UdpReceiver.Start();
			Request.Start();

			Logger.Log("NoxRelay started");
			Logger.Log("Press Ctrl+C to stop the server...");

			// Attendre le signal d'arrêt
			ShutdownEvent.WaitOne();
		}

		private static void OnCancelKeyPress(object? sender, ConsoleCancelEventArgs e)
		{
			Logger.Log("Ctrl+C pressed. Initiating graceful shutdown...");
			e.Cancel = true;
			Stop();
		}

		private static void OnProcessExit(object? sender, EventArgs e)
		{
			Logger.Log("Process exit signal received. Initiating graceful shutdown...");
			Stop();
		}

		private static void Stop()
		{
			Logger.Log("Stopping NoxRelay...");
			Request.Stop();
			TcpReceiver.Stop();
			UdpReceiver.Stop();
			MasterServer.Stop();

			// Signaler que l'arrêt est terminé
			ShutdownEvent.Set();

			Logger.Log("NoxRelay stopped");
		}
	}
}