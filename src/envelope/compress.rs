use std::io::Read;

use zstd::stream::encode_all;
use zstd::stream::read::Decoder;

use crate::envelope::error::Error;
#[cfg(test)]
use crate::limits::Limits;

pub fn compress(data: &[u8]) -> Result<Vec<u8>, Error> {
    encode_all(data, 0).map_err(|e| Error::new(format!("zstd compression failed: {e}")))
}

#[cfg(test)]
fn decompress(data: &[u8]) -> Result<Vec<u8>, Error> {
    decompress_limited(data, Limits::default().max_decompressed_len)
}

pub fn decompress_limited(data: &[u8], limit: usize) -> Result<Vec<u8>, Error> {
    let err = |e| Error::new(format!("zstd decompression failed: {e}"));
    let decoder = Decoder::new(data).map_err(err)?;
    let mut out = Vec::new();
    // one byte past limit to tell "at limit" from "over"
    decoder.take(limit as u64 + 1).read_to_end(&mut out).map_err(err)?;
    if out.len() > limit {
        return Err(Error::new(format!("decompressed payload exceeds {limit} bytes")));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let data = b"hello world, hello world, hello world".repeat(10);
        let compressed = compress(&data).unwrap();
        assert!(compressed.len() < data.len());
        assert_eq!(decompress(&compressed).unwrap(), data);
    }

    #[test]
    fn empty_roundtrip() {
        let compressed = compress(&[]).unwrap();
        assert_eq!(decompress(&compressed).unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn decompress_enforces_limit() {
        let data = vec![0u8; 4096];
        let compressed = compress(&data).unwrap();
        assert_eq!(decompress_limited(&compressed, 4096).unwrap(), data);
        assert!(decompress_limited(&compressed, 4095).is_err());
    }

    #[test]
    fn decompress_rejects_trailing_garbage() {
        let mut compressed = compress(b"hello").unwrap();
        compressed.extend_from_slice(b"junk");
        assert!(decompress(&compressed).is_err());
    }

    #[test]
    fn decompress_rejects_garbage() {
        assert!(decompress(b"not zstd data").is_err());
    }
}
