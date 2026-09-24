//! `.tave` envelope: compression, authenticated encryption, and signatures
//! wrapped around a canonical `.tavb` document.

mod aead;
mod compress;
mod error;
mod header;
mod kdf;
mod random;
mod sign;

pub use error::Error;
pub use header::{Flags, KdfParams};

use crate::binary;
use crate::limits::Limits;
use crate::value::Map;

/// How to protect the document when sealing.
pub enum SealMode<'a> {
    /// No encryption; payload is plaintext (optionally compressed).
    None,
    /// XChaCha20-Poly1305 with a caller-supplied 32-byte key.
    Key(&'a [u8; aead::KEY_LEN]),
    /// XChaCha20-Poly1305 with an Argon2id-derived key.
    Password(&'a [u8]),
}

pub struct SealOptions<'a> {
    pub mode: SealMode<'a>,
    pub compress: bool,
    /// Ed25519 signing key (32-byte seed), if the document should be signed.
    pub sign_with: Option<&'a [u8; 32]>,
}

/// Encodes, optionally compresses, optionally encrypts, and optionally
/// signs `root` into a complete `.tave` document.
pub fn seal(root: &Map, opts: &SealOptions<'_>) -> Vec<u8> {
    let value_bytes = binary::encode_value_bytes(root);

    // store raw if compression fails or payload is over open's limit
    let compressed = (opts.compress && value_bytes.len() <= Limits::default().max_decompressed_len)
        .then(|| compress::compress(&value_bytes).ok())
        .flatten();
    let is_compressed = compressed.is_some();
    let plaintext = compressed.unwrap_or_else(|| value_bytes.clone());

    let flags = Flags {
        encrypted: !matches!(opts.mode, SealMode::None),
        signed: opts.sign_with.is_some(),
        compressed: is_compressed,
        password: matches!(opts.mode, SealMode::Password(_)),
    };

    // Built once per branch, so the AAD (which must include kdf_params) and
    // the encryption step always agree on the exact same params.
    let (aad, nonce, payload) = match &opts.mode {
        SealMode::None => (header::build_aad(flags, None), None, plaintext),
        SealMode::Key(key) => {
            let aad = header::build_aad(flags, None);
            let (nonce, ct) = aead::encrypt(key, &aad, &plaintext);
            (aad, Some(nonce), ct)
        }
        SealMode::Password(password) => {
            let params = kdf::random_params();
            let aad = header::build_aad(flags, Some(&params));
            // `random_params` always uses the fixed DEFAULT_* cost
            // constants, which are valid Argon2id parameters by
            // construction, so derivation cannot fail here.
            let key = kdf::derive_key(password, &params)
                .unwrap_or_else(|_| unreachable!("random_params always produces valid Argon2id parameters"));
            let (nonce, ct) = aead::encrypt(&key, &aad, &plaintext);
            (aad, Some(nonce), ct)
        }
    };

    let signature = opts.sign_with.map(|sk| sign::sign(sk, &value_bytes));

    let mut out = aad; // starts as magic || version || flags || kdf_params
    if let Some(n) = nonce {
        out.extend_from_slice(&n);
    }
    if let Some(sig) = signature {
        out.extend_from_slice(&sig);
    }
    out.extend_from_slice(&payload);
    out
}

/// How to open a sealed document.
pub enum OpenMode<'a> {
    /// Document is expected to be unencrypted.
    None,
    Key(&'a [u8; aead::KEY_LEN]),
    Password(&'a [u8]),
}

/// Decrypts (if needed), decompresses (if needed), and decodes a `.tave`
/// document. With `verify_with`, document must be signed by that key;
/// unsigned is rejected. Without it, signature is not checked.
pub fn open(bytes: &[u8], mode: &OpenMode<'_>, verify_with: Option<&[u8; sign::PUBLIC_KEY_LEN]>) -> Result<Map, Error> {
    open_with_limits(bytes, mode, verify_with, &Limits::default())
}

/// [`open`] with caller's limits, e.g. lower ones for a network service.
pub fn open_with_limits(
    bytes: &[u8],
    mode: &OpenMode<'_>,
    verify_with: Option<&[u8; sign::PUBLIC_KEY_LEN]>,
    limits: &Limits,
) -> Result<Map, Error> {
    let mut pos = 0;
    let magic = header::read_bytes(bytes, &mut pos, 4)?;
    if magic != header::MAGIC {
        return Err(Error::new("bad magic: not a .tave document"));
    }
    let version = header::read_u8(bytes, &mut pos)?;
    if version != header::VERSION {
        return Err(Error::new(format!("unsupported version {version}")));
    }
    let flags = Flags::from_byte(header::read_u8(bytes, &mut pos)?)?;

    let kdf_params =
        if flags.password { Some(KdfParams::decode(bytes, &mut pos)?) } else { None };

    if verify_with.is_some() && !flags.signed {
        return Err(Error::new("signature required but document is unsigned"));
    }

    match (&mode, flags.encrypted, flags.password) {
        (OpenMode::None, true, _) => return Err(Error::new("document is encrypted; a key or password is required")),
        (OpenMode::Key(_), false, _) => return Err(Error::new("document is not encrypted; no key expected")),
        (OpenMode::Password(_), _, false) => return Err(Error::new("document does not use password mode")),
        _ => {}
    }

    let aad_end = pos;
    let aad = &bytes[..aad_end];

    let nonce: Option<[u8; aead::NONCE_LEN]> = if flags.encrypted {
        let raw = header::read_bytes(bytes, &mut pos, aead::NONCE_LEN)?;
        let mut n = [0u8; aead::NONCE_LEN];
        n.copy_from_slice(raw);
        Some(n)
    } else {
        None
    };

    let signature: Option<[u8; sign::SIGNATURE_LEN]> = if flags.signed {
        let raw = header::read_bytes(bytes, &mut pos, sign::SIGNATURE_LEN)?;
        let mut s = [0u8; sign::SIGNATURE_LEN];
        s.copy_from_slice(raw);
        Some(s)
    } else {
        None
    };

    let payload = &bytes[pos..];

    let plaintext = match (flags.encrypted, mode) {
        (false, _) => payload.to_vec(),
        (true, OpenMode::Key(key)) => {
            let nonce = nonce.ok_or_else(|| Error::new("missing nonce"))?;
            aead::decrypt(key, &nonce, aad, payload)?
        }
        (true, OpenMode::Password(password)) => {
            let params = kdf_params.as_ref().ok_or_else(|| Error::new("missing KDF parameters"))?;
            kdf::check_limits(params, limits)?;
            let key = kdf::derive_key(password, params)?;
            let nonce = nonce.ok_or_else(|| Error::new("missing nonce"))?;
            aead::decrypt(&key, &nonce, aad, payload)?
        }
        (true, OpenMode::None) => return Err(Error::new("document is encrypted; a key or password is required")),
    };

    let value_bytes = if flags.compressed { compress::decompress_limited(&plaintext, limits.max_decompressed_len)? } else { plaintext };

    if let Some(pk) = verify_with {
        let sig = signature.ok_or_else(|| Error::new("signature required but document is unsigned"))?;
        sign::verify(pk, &value_bytes, &sig)?;
    }

    binary::decode_value_bytes_with_limits(&value_bytes, limits).map_err(|e| Error::new(format!("invalid document: {e}")))
}

/// Generates a fresh 32-byte XChaCha20-Poly1305 key.
pub fn generate_key() -> [u8; aead::KEY_LEN] {
    random::random_bytes::<{ aead::KEY_LEN }>()
}

/// Generates a fresh Ed25519 signing key seed and its matching public key.
pub fn generate_signing_key() -> ([u8; 32], [u8; sign::PUBLIC_KEY_LEN]) {
    let seed = random::random_bytes::<32>();
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&seed);
    (seed, signing_key.verifying_key().to_bytes())
}

#[cfg(test)]
mod tests;
