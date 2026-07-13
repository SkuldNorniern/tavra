use crate::value::Map;

use crate::binary::encode::encode_value_bytes;

/// A document's canonical identity: `BLAKE3(value_bytes)`, where
/// `value_bytes` excludes the magic/version prefix. Envelope signatures
/// sign the same bytes this hash is computed over.
pub fn hash_document(root: &Map) -> [u8; 32] {
    blake3::hash(&encode_value_bytes(root)).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text;
    use crate::value::Value;

    fn parse_map(src: &str) -> Map {
        match text::parse(src).unwrap() {
            Value::Map(m) => m,
            _ => unreachable!("document root is always a map"),
        }
    }

    #[test]
    fn same_value_same_hash_regardless_of_source_order() {
        let a = hash_document(&parse_map("b = 2\na = 1\n"));
        let b = hash_document(&parse_map("a = 1\nb = 2\n"));
        assert_eq!(a, b);
    }

    #[test]
    fn different_values_different_hash() {
        let a = hash_document(&parse_map("a = 1\n"));
        let b = hash_document(&parse_map("a = 2\n"));
        assert_ne!(a, b);
    }

    #[test]
    fn hash_excludes_magic_and_version_prefix() {
        let root = parse_map("a = 1\n");
        let expected = blake3::hash(&encode_value_bytes(&root));
        assert_eq!(hash_document(&root), *expected.as_bytes());
    }
}
