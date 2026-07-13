use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

use crate::envelope::error::Error;

pub const PUBLIC_KEY_LEN: usize = 32;
pub const SIGNATURE_LEN: usize = 64;

pub fn sign(signing_key_bytes: &[u8; 32], message: &[u8]) -> [u8; SIGNATURE_LEN] {
    let key = SigningKey::from_bytes(signing_key_bytes);
    key.sign(message).to_bytes()
}

pub fn verify(public_key_bytes: &[u8; PUBLIC_KEY_LEN], message: &[u8], signature_bytes: &[u8; SIGNATURE_LEN]) -> Result<(), Error> {
    let key = VerifyingKey::from_bytes(public_key_bytes).map_err(|e| Error::new(format!("invalid Ed25519 public key: {e}")))?;
    let signature = Signature::from_bytes(signature_bytes);
    key.verify(message, &signature).map_err(|_| Error::new("signature verification failed"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SECRET_KEY_LENGTH;

    fn test_keypair(seed: u8) -> ([u8; SECRET_KEY_LENGTH], [u8; PUBLIC_KEY_LEN]) {
        let signing_key = SigningKey::from_bytes(&[seed; SECRET_KEY_LENGTH]);
        (signing_key.to_bytes(), signing_key.verifying_key().to_bytes())
    }

    #[test]
    fn roundtrip() {
        let (sk, pk) = test_keypair(1);
        let sig = sign(&sk, b"canonical value bytes");
        assert!(verify(&pk, b"canonical value bytes", &sig).is_ok());
    }

    #[test]
    fn tampered_message_fails() {
        let (sk, pk) = test_keypair(2);
        let sig = sign(&sk, b"original message");
        assert!(verify(&pk, b"different message", &sig).is_err());
    }

    #[test]
    fn wrong_key_fails() {
        let (sk, _) = test_keypair(3);
        let (_, other_pk) = test_keypair(4);
        let sig = sign(&sk, b"message");
        assert!(verify(&other_pk, b"message", &sig).is_err());
    }

    #[test]
    fn tampered_signature_fails() {
        let (sk, pk) = test_keypair(5);
        let mut sig = sign(&sk, b"message");
        sig[0] ^= 0x01;
        assert!(verify(&pk, b"message", &sig).is_err());
    }
}
