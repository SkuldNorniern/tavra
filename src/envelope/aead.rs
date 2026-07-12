use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};

use super::error::Error;
use super::random::random_bytes;

pub const KEY_LEN: usize = 32;
pub const NONCE_LEN: usize = 24;

/// Encrypts `plaintext` under `key`, authenticating `aad`. Generates a
/// fresh random nonce (XChaCha20's 24-byte nonce is large enough that
/// random generation is safe against reuse at any realistic volume).
pub fn encrypt(key: &[u8; KEY_LEN], aad: &[u8], plaintext: &[u8]) -> ([u8; NONCE_LEN], Vec<u8>) {
    let cipher = XChaCha20Poly1305::new(&Key::from(*key));
    let nonce_bytes = random_bytes::<NONCE_LEN>();
    let nonce = XNonce::from(nonce_bytes);
    // The only failure mode for this cipher's `encrypt` is an oversized
    // plaintext (exabytes) — never reachable for in-memory documents.
    let ciphertext = cipher
        .encrypt(&nonce, Payload { msg: plaintext, aad })
        .unwrap_or_else(|_| unreachable!("XChaCha20-Poly1305 encryption cannot fail for in-memory payloads"));
    (nonce_bytes, ciphertext)
}

/// Decrypts `ciphertext` under `key`/`nonce`, verifying `aad`. The error
/// deliberately doesn't distinguish wrong key vs. tampered ciphertext vs.
/// tampered AAD — all are the same generic failure.
pub fn decrypt(key: &[u8; KEY_LEN], nonce: &[u8; NONCE_LEN], aad: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, Error> {
    let cipher = XChaCha20Poly1305::new(&Key::from(*key));
    let nonce = XNonce::from(*nonce);
    cipher.decrypt(&nonce, Payload { msg: ciphertext, aad }).map_err(|_| Error::new("decryption failed"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let key = [7u8; KEY_LEN];
        let aad = b"header bytes";
        let plaintext = b"canonical value bytes";
        let (nonce, ciphertext) = encrypt(&key, aad, plaintext);
        assert_eq!(decrypt(&key, &nonce, aad, &ciphertext).unwrap(), plaintext);
    }

    #[test]
    fn wrong_key_fails() {
        let (nonce, ciphertext) = encrypt(&[1u8; KEY_LEN], b"aad", b"secret");
        assert!(decrypt(&[2u8; KEY_LEN], &nonce, b"aad", &ciphertext).is_err());
    }

    #[test]
    fn tampered_ciphertext_fails() {
        let key = [3u8; KEY_LEN];
        let (nonce, mut ciphertext) = encrypt(&key, b"aad", b"secret");
        let last = ciphertext.len() - 1;
        ciphertext[last] ^= 0x01;
        assert!(decrypt(&key, &nonce, b"aad", &ciphertext).is_err());
    }

    #[test]
    fn tampered_aad_fails() {
        let key = [4u8; KEY_LEN];
        let (nonce, ciphertext) = encrypt(&key, b"original aad", b"secret");
        assert!(decrypt(&key, &nonce, b"different aad", &ciphertext).is_err());
    }

    #[test]
    fn nonces_are_not_reused_across_calls() {
        let key = [5u8; KEY_LEN];
        let (n1, _) = encrypt(&key, b"aad", b"secret");
        let (n2, _) = encrypt(&key, b"aad", b"secret");
        assert_ne!(n1, n2);
    }
}
