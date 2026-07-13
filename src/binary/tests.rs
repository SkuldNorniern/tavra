use crate::text;
use crate::value::{Int, Map, Value};

use super::{decode, encode};

fn parse_map(src: &str) -> Map {
    match text::parse(src).unwrap() {
        Value::Map(m) => m,
        _ => unreachable!("document root is always a map"),
    }
}

fn roundtrip(src: &str) {
    let root = parse_map(src);
    let bytes = encode(&root);
    let decoded = decode(&bytes).unwrap();
    assert_eq!(root, decoded, "round-trip mismatch for: {src}");
}

#[test]
fn scalars_roundtrip() {
    roundtrip("a = null\nb = true\nc = false\nd = 42\ne = -17\nf = 3.5\ng = \"hi\"\nh = b\"deadbeef\"\n");
}

#[test]
fn full_int_range_roundtrips() {
    for v in [0i64, 1, -1, i64::MAX, i64::MIN, 100, -100] {
        let mut m = Map::new();
        m.insert("v".into(), Value::Int(Int::from_i64(v)));
        let bytes = encode(&m);
        assert_eq!(decode(&bytes).unwrap(), m);
    }
    let mut m = Map::new();
    m.insert("v".into(), Value::Int(Int::from_u64(u64::MAX)));
    let bytes = encode(&m);
    assert_eq!(decode(&bytes).unwrap(), m);
}

#[test]
fn nested_structures_roundtrip() {
    roundtrip("server = {\n    host = \"0.0.0.0\"\n    tls = { cert = b64\"aGVsbG8gd29ybGQ=\" }\n}\n");
    roundtrip("servers = [\n    { name = \"a\", port = 1 },\n    { name = \"b\", port = 2 },\n]\n");
}

#[test]
fn datetimes_roundtrip() {
    roundtrip("a = 2026-07-12T10:30:00.5+09:00\nb = 2026-07-12\nc = 10:30:00\nd = 2026-07-12T10:30:00\n");
}

#[test]
fn nan_roundtrips_to_canonical_bits() {
    let root = parse_map("a = nan\n");
    let bytes = encode(&root);
    let decoded = decode(&bytes).unwrap();
    assert_eq!(root, decoded);
}

#[test]
fn same_value_same_bytes() {
    let a = encode(&parse_map("b = 2\na = 1\n"));
    let b = encode(&parse_map("a = 1\nb = 2\n"));
    assert_eq!(a, b, "encoding must not depend on source key order");
}

#[test]
fn rejects_bad_magic() {
    let mut bytes = encode(&Map::new());
    bytes[0] = b'X';
    assert!(decode(&bytes).is_err());
}

#[test]
fn rejects_unsupported_version() {
    let mut bytes = encode(&Map::new());
    bytes[4] = 0x02;
    assert!(decode(&bytes).is_err());
}

#[test]
fn rejects_trailing_bytes() {
    let mut bytes = encode(&Map::new());
    bytes.push(0xFF);
    assert!(decode(&bytes).is_err());
}

#[test]
fn rejects_truncated_document() {
    let bytes = encode(&parse_map("a = \"hello world\"\n"));
    for len in 0..bytes.len() {
        assert!(decode(&bytes[..len]).is_err(), "truncation at {len} should fail, not panic");
    }
}

#[test]
fn rejects_unknown_tag() {
    let mut bytes = encode(&Map::new());
    // Overwrite the empty map's tag (byte 5, right after magic+version)
    // with a reserved tag.
    bytes[5] = 0x0D;
    assert!(decode(&bytes).is_err());
}

#[test]
fn rejects_unsorted_map_keys() {
    // Hand-build a 2-entry map with keys "b" then "a" (wrong order).
    let mut bytes = super::encode::MAGIC.to_vec();
    bytes.push(super::encode::VERSION);
    bytes.push(0x0C); // map tag
    bytes.push(0x02); // 2 entries
    bytes.push(0x01); // key len 1
    bytes.push(b'b');
    bytes.push(0x00); // null value
    bytes.push(0x01); // key len 1
    bytes.push(b'a');
    bytes.push(0x00); // null value
    assert!(decode(&bytes).is_err());
}

#[test]
fn rejects_duplicate_map_keys() {
    let mut bytes = super::encode::MAGIC.to_vec();
    bytes.push(super::encode::VERSION);
    bytes.push(0x0C);
    bytes.push(0x02);
    bytes.push(0x01);
    bytes.push(b'a');
    bytes.push(0x00);
    bytes.push(0x01);
    bytes.push(b'a');
    bytes.push(0x00);
    assert!(decode(&bytes).is_err());
}

#[test]
fn rejects_invalid_utf8_string() {
    let mut bytes = super::encode::MAGIC.to_vec();
    bytes.push(super::encode::VERSION);
    bytes.push(0x0C);
    bytes.push(0x01);
    bytes.push(0x01);
    bytes.push(b'a');
    bytes.push(0x05); // string tag
    bytes.push(0x01); // len 1
    bytes.push(0xFF); // invalid UTF-8 byte
    assert!(decode(&bytes).is_err());
}

#[test]
fn rejects_out_of_range_date() {
    let mut bytes = super::encode::MAGIC.to_vec();
    bytes.push(super::encode::VERSION);
    bytes.push(0x0C);
    bytes.push(0x01);
    bytes.push(0x01);
    bytes.push(b'a');
    bytes.push(0x07); // date tag
    bytes.extend_from_slice(&2026u16.to_be_bytes());
    bytes.push(13); // invalid month
    bytes.push(1);
    assert!(decode(&bytes).is_err());
}
