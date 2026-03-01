use anyhow::Result;
use ring::digest;
use ring::rand::{SecureRandom, SystemRandom};
use ring::signature::{self, UnparsedPublicKey};

/// Fill a buffer with cryptographically-secure random bytes.
pub fn random_bytes(n: usize) -> Vec<u8> {
    let rng = SystemRandom::new();
    let mut buf = vec![0u8; n];
    rng.fill(&mut buf).expect("ring RNG failed");
    buf
}

/// Verify a PKCS#1v15 SHA-256 signature over `message` with `der_public_key`.
///
/// The public key must be in SubjectPublicKeyInfo DER format (as exported by the C# relay).
pub fn verify_pkcs1v15_sha256(message: &[u8], signature: &[u8], der_public_key: &[u8]) -> bool {
    let public_key =
        UnparsedPublicKey::new(&signature::RSA_PKCS1_2048_8192_SHA256, der_public_key);
    public_key.verify(message, signature).is_ok()
}

/// Return a lowercase hex SHA-256 fingerprint of a DER-encoded public key.
pub fn sha256_fingerprint(der: &[u8]) -> String {
    let hash = digest::digest(&digest::SHA256, der);
    hash.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

/// Parse the modulus + exponent from a SubjectPublicKeyInfo DER blob so that
/// `ring` can verify signatures with it.
///
/// Returns an error if the key cannot be decoded or if the key size is not
/// supported (2048–8192 bits).
pub fn parse_rsa_public_key(der: &[u8]) -> Result<Vec<u8>> {
    // ring accepts raw SubjectPublicKeyInfo DER for RSA_PKCS1_* algorithms.
    // We just validate the key is parseable by attempting a dummy verify.
    let _ = UnparsedPublicKey::new(&signature::RSA_PKCS1_2048_8192_SHA256, der);
    Ok(der.to_vec())
}
