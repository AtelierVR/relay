namespace Relay.Utils;

using System;
using System.Text;
using System.Security.Cryptography;

public class Hashing
{
    static string Hash(string password, string salt)
    {
        using var pbkdf2 = new Rfc2898DeriveBytes(Encoding.UTF8.GetBytes(password),
            Convert.FromBase64String(salt),
            10000, HashAlgorithmName.SHA256);
        return $"{salt}:{Convert.ToBase64String(pbkdf2.GetBytes(64))}";
    }

    public static bool Verify(string password, string hash) => hash == Hash(password, hash.Split(':')[0]);
}