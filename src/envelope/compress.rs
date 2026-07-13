use zstd::stream::{decode_all, encode_all};

use crate::envelope::error::Error;

pub fn compress(data: &[u8]) -> Result<Vec<u8>, Error> {
    encode_all(data, 0).map_err(|e| Error::new(format!("zstd compression failed: {e}")))
}

pub fn decompress(data: &[u8]) -> Result<Vec<u8>, Error> {
    decode_all(data).map_err(|e| Error::new(format!("zstd decompression failed: {e}")))
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
    fn decompress_rejects_garbage() {
        assert!(decompress(b"not zstd data").is_err());
    }
}
