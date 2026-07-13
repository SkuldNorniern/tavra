use crate::binary::error::Error;

/// Appends the canonical (minimal-length) unsigned LEB128 encoding of `v`.
pub fn encode_varint(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let byte = (v & 0x7F) as u8;
        v >>= 7;
        if v == 0 {
            out.push(byte);
            break;
        }
        out.push(byte | 0x80);
    }
}

/// Decodes a canonical unsigned LEB128 varint starting at `*pos`, advancing
/// it past the consumed bytes. Rejects non-minimal encodings and overflow.
pub fn decode_varint(bytes: &[u8], pos: &mut usize) -> Result<u64, Error> {
    let start = *pos;
    let mut result: u64 = 0;
    let mut shift: u32 = 0;
    loop {
        let Some(&b) = bytes.get(*pos) else {
            return Err(Error::new(*pos, "unexpected end of input in varint"));
        };
        *pos += 1;
        let payload = u64::from(b & 0x7F);
        if shift == 63 && payload > 1 {
            return Err(Error::new(*pos, "varint overflows 64 bits"));
        }
        if shift >= 64 {
            return Err(Error::new(*pos, "varint too long"));
        }
        result |= payload << shift;
        if b & 0x80 == 0 {
            break;
        }
        shift += 7;
    }

    let mut check = Vec::new();
    encode_varint(&mut check, result);
    if check.len() != *pos - start {
        return Err(Error::new(start, "non-canonical varint encoding"));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(v: u64) {
        let mut buf = Vec::new();
        encode_varint(&mut buf, v);
        let mut pos = 0;
        assert_eq!(decode_varint(&buf, &mut pos).unwrap(), v);
        assert_eq!(pos, buf.len());
    }

    #[test]
    fn small_values_one_byte() {
        let mut buf = Vec::new();
        encode_varint(&mut buf, 5);
        assert_eq!(buf, vec![0x05]);
        roundtrip(0);
        roundtrip(127);
    }

    #[test]
    fn boundary_and_extreme_values() {
        roundtrip(128);
        roundtrip(300);
        roundtrip(u64::MAX);
        roundtrip(u64::MAX - 1);
    }

    #[test]
    fn rejects_non_canonical_padding() {
        // 5 encoded as two bytes (0x85, 0x00) instead of canonical (0x05).
        let mut pos = 0;
        assert!(decode_varint(&[0x85, 0x00], &mut pos).is_err());
    }

    #[test]
    fn rejects_truncated_input() {
        let mut pos = 0;
        assert!(decode_varint(&[0x80], &mut pos).is_err());
        assert!(decode_varint(&[], &mut pos).is_err());
    }

    #[test]
    fn rejects_overflow() {
        // 10 continuation bytes then a stray 11th byte overflows 64 bits.
        let bytes = [0xFFu8; 10];
        let mut buf = bytes.to_vec();
        buf.push(0x02);
        let mut pos = 0;
        assert!(decode_varint(&buf, &mut pos).is_err());
    }
}
