using System;

namespace Relay;

public class Logger
{
    public const string Format = "{0} [{1}] {2}";
    // get environment variable
    public static bool PrintDebug => Environment.GetEnvironmentVariable("DEBUG") == "true";

    public static string LastDate => DateTime.Now.ToString("dd/MM/yyyy HH:mm:ss");

    public static void Log(string message, params object[] args)
    {
        Console.WriteLine(Format, LastDate, "INFO", string.Format(message, args));
    }

    public static void Error(string message, params object[] args)
    {
        Console.WriteLine(Format, LastDate, "ERROR", string.Format(message, args));
    }

    public static void Warning(string message, params object[] args)
    {
        Console.WriteLine(Format, LastDate, "WARNING", string.Format(message, args));
    }

    public static void Debug(string message, params object[] args)
    {
        if (!PrintDebug) return;
        Console.WriteLine(Format, LastDate, "DEBUG", string.Format(message, args));
    }
}