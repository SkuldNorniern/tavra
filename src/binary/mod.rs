//! Canonical binary (`.tavb`) encoding and decoding.

mod decode;
mod encode;
mod error;
mod hash;
#[cfg(test)]
mod tests;
mod varint;

pub use error::Error;

use crate::limits::Limits;
use crate::value::Map;

/// Encodes `root` as a complete `.tavb` document (magic + version + value).
pub fn encode(root: &Map) -> Vec<u8> {
    encode::encode_document(root)
}

/// A document's canonical identity: `BLAKE3(value_bytes)`.
pub fn hash(root: &Map) -> [u8; 32] {
    hash::hash_document(root)
}

/// Decodes a complete `.tavb` document, rejecting any non-canonical or
/// malformed input. Never panics on arbitrary input bytes.
pub fn decode(bytes: &[u8]) -> Result<Map, Error> {
    decode_with_limits(bytes, &Limits::default())
}

/// [`decode`] with caller's limits. Only `max_depth` applies to binary.
pub fn decode_with_limits(bytes: &[u8], limits: &Limits) -> Result<Map, Error> {
    decode::decode_document(bytes, limits.depth())
}

/// Encodes `root`'s value bytes only, without the magic/version prefix —
/// this is what hashes and signatures are computed over.
pub fn encode_value_bytes(root: &Map) -> Vec<u8> {
    encode::encode_value_bytes(root)
}

/// Decodes value bytes with no magic/version prefix (the counterpart to
/// `encode_value_bytes`).
pub fn decode_value_bytes(bytes: &[u8]) -> Result<Map, Error> {
    decode::decode_value_bytes(bytes, Limits::default().depth())
}

pub(crate) fn decode_value_bytes_with_limits(bytes: &[u8], limits: &Limits) -> Result<Map, Error> {
    decode::decode_value_bytes(bytes, limits.depth())
}
