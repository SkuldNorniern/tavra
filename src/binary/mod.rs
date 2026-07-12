//! Canonical binary (`.tavb`) encoding and decoding.

mod decode;
mod encode;
mod error;
#[cfg(test)]
mod tests;
mod varint;

pub use error::Error;

use crate::value::Map;

/// Encodes `root` as a complete `.tavb` document (magic + version + value).
pub fn encode(root: &Map) -> Vec<u8> {
    encode::encode_document(root)
}

/// Decodes a complete `.tavb` document, rejecting any non-canonical or
/// malformed input. Never panics on arbitrary input bytes.
pub fn decode(bytes: &[u8]) -> Result<Map, Error> {
    decode::decode_document(bytes)
}

/// Encodes `root`'s value bytes only, without the magic/version prefix —
/// this is what hashes and signatures are computed over.
pub fn encode_value_bytes(root: &Map) -> Vec<u8> {
    encode::encode_value_bytes(root)
}
