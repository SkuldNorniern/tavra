use crate::text;
use crate::value::{Map, Value};

use super::*;

fn parse_map(src: &str) -> Map {
    match text::parse(src).unwrap() {
        Value::Map(m) => m,
        _ => unreachable!("document root is always a map"),
    }
}

#[test]
fn unencrypted_roundtrip() {
    let root = parse_map("a = 1\nb = \"hello\"\n");
    let bytes = seal(&root, &SealOptions { mode: SealMode::None, compress: false, sign_with: None });
    let decoded = open(&bytes, &OpenMode::None, None).unwrap();
    assert_eq!(root, decoded);
}

#[test]
fn key_mode_roundtrip() {
    let root = parse_map("secret = \"value\"\n");
    let key = generate_key();
    let bytes = seal(&root, &SealOptions { mode: SealMode::Key(&key), compress: false, sign_with: None });
    let decoded = open(&bytes, &OpenMode::Key(&key), None).unwrap();
    assert_eq!(root, decoded);
}

#[test]
fn password_mode_roundtrip() {
    let root = parse_map("secret = \"value\"\n");
    let password = b"correct horse battery staple";
    let bytes = seal(&root, &SealOptions { mode: SealMode::Password(password), compress: true, sign_with: None });
    let decoded = open(&bytes, &OpenMode::Password(password), None).unwrap();
    assert_eq!(root, decoded);
}

#[test]
fn wrong_password_fails() {
    let root = parse_map("secret = \"value\"\n");
    let bytes = seal(&root, &SealOptions { mode: SealMode::Password(b"right"), compress: false, sign_with: None });
    assert!(open(&bytes, &OpenMode::Password(b"wrong"), None).is_err());
}

#[test]
fn wrong_key_fails() {
    let root = parse_map("secret = \"value\"\n");
    let key = generate_key();
    let other_key = generate_key();
    let bytes = seal(&root, &SealOptions { mode: SealMode::Key(&key), compress: false, sign_with: None });
    assert!(open(&bytes, &OpenMode::Key(&other_key), None).is_err());
}

#[test]
fn compressed_roundtrip_and_smaller() {
    let src = format!("a = \"{}\"\n", "hello world ".repeat(50));
    let root = parse_map(&src);
    let uncompressed = seal(&root, &SealOptions { mode: SealMode::None, compress: false, sign_with: None });
    let compressed = seal(&root, &SealOptions { mode: SealMode::None, compress: true, sign_with: None });
    assert!(compressed.len() < uncompressed.len());
    assert_eq!(open(&compressed, &OpenMode::None, None).unwrap(), root);
}

#[test]
fn signed_and_verified_roundtrip() {
    let root = parse_map("a = 1\n");
    let (sk, pk) = generate_signing_key();
    let bytes = seal(&root, &SealOptions { mode: SealMode::None, compress: false, sign_with: Some(&sk) });
    let decoded = open(&bytes, &OpenMode::None, Some(&pk)).unwrap();
    assert_eq!(root, decoded);
}

#[test]
fn signed_with_wrong_key_fails_verification() {
    let root = parse_map("a = 1\n");
    let (sk, _) = generate_signing_key();
    let (_, other_pk) = generate_signing_key();
    let bytes = seal(&root, &SealOptions { mode: SealMode::None, compress: false, sign_with: Some(&sk) });
    assert!(open(&bytes, &OpenMode::None, Some(&other_pk)).is_err());
}

#[test]
fn signed_and_encrypted_roundtrip() {
    let root = parse_map("a = 1\nb = [1, 2, 3]\n");
    let key = generate_key();
    let (sk, pk) = generate_signing_key();
    let bytes = seal(&root, &SealOptions { mode: SealMode::Key(&key), compress: true, sign_with: Some(&sk) });
    let decoded = open(&bytes, &OpenMode::Key(&key), Some(&pk)).unwrap();
    assert_eq!(root, decoded);
}

#[test]
fn signature_verification_is_optional_on_open() {
    // A SIGNED document can still be opened without checking the
    // signature, if the caller passes verify_with = None.
    let root = parse_map("a = 1\n");
    let (sk, _pk) = generate_signing_key();
    let bytes = seal(&root, &SealOptions { mode: SealMode::None, compress: false, sign_with: Some(&sk) });
    assert_eq!(open(&bytes, &OpenMode::None, None).unwrap(), root);
}

#[test]
fn rejects_bad_magic() {
    let root = parse_map("a = 1\n");
    let mut bytes = seal(&root, &SealOptions { mode: SealMode::None, compress: false, sign_with: None });
    bytes[0] = b'X';
    assert!(open(&bytes, &OpenMode::None, None).is_err());
}

#[test]
fn rejects_encrypted_document_opened_as_none() {
    let root = parse_map("a = 1\n");
    let key = generate_key();
    let bytes = seal(&root, &SealOptions { mode: SealMode::Key(&key), compress: false, sign_with: None });
    assert!(open(&bytes, &OpenMode::None, None).is_err());
}

#[test]
fn rejects_reserved_flag_bits() {
    let root = parse_map("a = 1\n");
    let mut bytes = seal(&root, &SealOptions { mode: SealMode::None, compress: false, sign_with: None });
    bytes[5] |= 0b0001_0000; // flags byte is at offset 5 (after 4-byte magic + 1-byte version)
    assert!(open(&bytes, &OpenMode::None, None).is_err());
}

#[test]
fn rejects_tampered_ciphertext() {
    let root = parse_map("a = 1\n");
    let key = generate_key();
    let mut bytes = seal(&root, &SealOptions { mode: SealMode::Key(&key), compress: false, sign_with: None });
    let last = bytes.len() - 1;
    bytes[last] ^= 0x01;
    assert!(open(&bytes, &OpenMode::Key(&key), None).is_err());
}

#[test]
fn unsigned_document_fails_when_verification_requested() {
    let root = parse_map("a = 1\n");
    let (_, pk) = generate_signing_key();
    let bytes = seal(&root, &SealOptions { mode: SealMode::None, compress: false, sign_with: None });
    assert!(open(&bytes, &OpenMode::None, Some(&pk)).is_err());
}

#[test]
fn stripped_signature_fails_verification() {
    // clear SIGNED and drop signature bytes; unencrypted header has no auth
    let root = parse_map("a = 1\n");
    let (sk, pk) = generate_signing_key();
    let signed = seal(&root, &SealOptions { mode: SealMode::None, compress: false, sign_with: Some(&sk) });
    let mut stripped = signed[..6].to_vec();
    stripped[5] = Flags { encrypted: false, signed: false, compressed: false, password: false }.to_byte();
    stripped.extend_from_slice(&signed[6 + sign::SIGNATURE_LEN..]);
    assert_eq!(open(&stripped, &OpenMode::None, None).unwrap(), root);
    assert!(open(&stripped, &OpenMode::None, Some(&pk)).is_err());
}
