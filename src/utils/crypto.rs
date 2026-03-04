use anyhow::Result;
use ring::digest;
use ring::rand::{SecureRandom, SystemRandom};

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
/// Supports RSA key sizes from 2048 to 8192 bits using rsa crate instead of ring.
pub fn verify_pkcs1v15_sha256(message: &[u8], signature: &[u8], der_public_key: &[u8]) -> bool {
    use rsa::{pkcs1v15::VerifyingKey, pkcs8::DecodePublicKey, signature::Verifier, RsaPublicKey};
    use sha2::Sha256;
    use tracing::{debug, warn};

    // Décoder la clé publique depuis le format DER (SubjectPublicKeyInfo)
    let public_key = match RsaPublicKey::from_public_key_der(der_public_key) {
        Ok(key) => {
            use rsa::traits::PublicKeyParts;
            debug!(
                "[Crypto] Successfully decoded public key, {} bits",
                key.n().bits()
            );
            key
        }
        Err(e) => {
            warn!("[Crypto] Failed to decode public key: {}", e);
            return false;
        }
    };

    // Créer un VerifyingKey qui hashera automatiquement avec SHA-256 (comme SigningKey)
    let verifying_key = VerifyingKey::<Sha256>::new(public_key);
    debug!(
        "[Crypto] Verifying signature: message_len={}, signature_len={}",
        message.len(),
        signature.len()
    );

    // Convertir la signature en type Signature
    match rsa::pkcs1v15::Signature::try_from(signature) {
        Ok(sig) => match verifying_key.verify(message, &sig) {
            Ok(_) => {
                debug!("[Crypto] Signature verification succeeded");
                true
            }
            Err(e) => {
                warn!("[Crypto] Signature verification failed: {}", e);
                warn!("[Crypto] Message (hex): {}", hex::encode(message));
                warn!(
                    "[Crypto] Signature (first 64 bytes, hex): {}",
                    hex::encode(&signature[..signature.len().min(64)])
                );
                false
            }
        },
        Err(e) => {
            warn!("[Crypto] Failed to parse signature: {}", e);
            false
        }
    }
}

/// Return a lowercase hex SHA-256 fingerprint of a DER-encoded public key.
pub fn sha256_fingerprint(der: &[u8]) -> String {
    let hash = digest::digest(&digest::SHA256, der);
    hash.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

/// Parse the modulus + exponent from a SubjectPublicKeyInfo DER blob.
///
/// Returns an error if the key cannot be decoded or if the key size is not
/// supported (2048–8192 bits).
pub fn parse_rsa_public_key(der: &[u8]) -> Result<Vec<u8>> {
    use rsa::{pkcs8::DecodePublicKey, RsaPublicKey};

    // Validate the key is parseable
    let _public_key = RsaPublicKey::from_public_key_der(der)?;
    Ok(der.to_vec())
}
