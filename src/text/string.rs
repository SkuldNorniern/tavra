use crate::text::cursor::Cursor;
use crate::text::error::Error;

fn is_disallowed_control(c: char) -> bool {
    // Tab and line breaks are always allowed; every other control char
    // (including DEL) is rejected in every string form.
    (c.is_control()) && c != '\t' && c != '\n' && c != '\r'
}

/// Scans a basic string: `"..."` or `"""..."""`. Cursor must be positioned
/// at the opening `"`.
pub fn scan_basic_string(cur: &mut Cursor) -> Result<String, Error> {
    let triple = cur.peek() == Some('"') && cur.peek_at(1) == Some('"');
    if triple {
        cur.bump();
        cur.bump();
        cur.bump();
        skip_leading_newline(cur);
    } else {
        cur.bump();
    }

    let mut out = String::new();
    loop {
        if triple {
            if cur.peek() == Some('"') && cur.peek_at(1) == Some('"') && cur.peek_at(2) == Some('"') {
                cur.bump();
                cur.bump();
                cur.bump();
                return Ok(out);
            }
        } else if cur.peek() == Some('"') {
            cur.bump();
            return Ok(out);
        } else if !triple && matches!(cur.peek(), Some('\n') | Some('\r') | None) {
            return Err(cur.error("unterminated string"));
        }

        let Some(c) = cur.bump() else {
            return Err(cur.error("unterminated string"));
        };

        if c == '\\' {
            out.push(scan_escape(cur)?);
        } else if is_disallowed_control(c) {
            return Err(cur.error("literal control character in string"));
        } else {
            out.push(c);
        }
    }
}

/// Scans a raw string: `'...'` or `'''...'''`. Cursor must be positioned at
/// the opening `'`.
pub fn scan_raw_string(cur: &mut Cursor) -> Result<String, Error> {
    let triple = cur.peek() == Some('\'') && cur.peek_at(1) == Some('\'');
    if triple {
        cur.bump();
        cur.bump();
        cur.bump();
        skip_leading_newline(cur);
    } else {
        cur.bump();
    }

    let mut out = String::new();
    loop {
        if triple {
            if cur.peek() == Some('\'') && cur.peek_at(1) == Some('\'') && cur.peek_at(2) == Some('\'') {
                cur.bump();
                cur.bump();
                cur.bump();
                return Ok(out);
            }
        } else if cur.peek() == Some('\'') {
            cur.bump();
            return Ok(out);
        } else if !triple && matches!(cur.peek(), Some('\n') | Some('\r') | None) {
            return Err(cur.error("unterminated string"));
        }

        let Some(c) = cur.bump() else {
            return Err(cur.error("unterminated string"));
        };

        if is_disallowed_control(c) {
            return Err(cur.error("literal control character in string"));
        }
        out.push(c);
    }
}

fn skip_leading_newline(cur: &mut Cursor) {
    if cur.peek() == Some('\r') && cur.peek_at(1) == Some('\n') {
        cur.bump();
        cur.bump();
    } else if cur.peek() == Some('\n') {
        cur.bump();
    }
}

fn scan_escape(cur: &mut Cursor) -> Result<char, Error> {
    let Some(c) = cur.bump() else {
        return Err(cur.error("unterminated escape sequence"));
    };
    match c {
        'n' => Ok('\n'),
        't' => Ok('\t'),
        'r' => Ok('\r'),
        '\\' => Ok('\\'),
        '"' => Ok('"'),
        'u' => scan_unicode_escape(cur),
        other => Err(cur.error(format!("unknown escape '\\{other}'"))),
    }
}

fn scan_unicode_escape(cur: &mut Cursor) -> Result<char, Error> {
    if cur.bump() != Some('{') {
        return Err(cur.error("expected '{' after \\u"));
    }
    let mut digits = String::new();
    while let Some(c) = cur.peek() {
        if c == '}' {
            break;
        }
        digits.push(c);
        cur.bump();
    }
    if cur.bump() != Some('}') {
        return Err(cur.error("unterminated unicode escape"));
    }
    if digits.is_empty() || digits.len() > 6 {
        return Err(cur.error("unicode escape must have 1-6 hex digits"));
    }
    let code = u32::from_str_radix(&digits, 16).map_err(|_| cur.error("invalid hex digits in unicode escape"))?;
    char::from_u32(code).ok_or_else(|| cur.error("unicode escape is not a valid scalar value"))
}

/// Decodes a `b"hex"` literal body: even-length, case-insensitive hex.
pub fn decode_hex_bytes(cur: &Cursor, digits: &str) -> Result<Vec<u8>, Error> {
    if digits.len() % 2 != 0 {
        return Err(cur.error("hex byte literal must have an even number of digits"));
    }
    let mut out = Vec::with_capacity(digits.len() / 2);
    let bytes = digits.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let hi = hex_val(bytes[i]).ok_or_else(|| cur.error("invalid hex digit in byte literal"))?;
        let lo = hex_val(bytes[i + 1]).ok_or_else(|| cur.error("invalid hex digit in byte literal"))?;
        out.push((hi << 4) | lo);
        i += 2;
    }
    Ok(out)
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Encodes bytes as lowercase hex, for `b"…"` literals.
pub fn encode_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

const BASE64_ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Encodes bytes as standard padded base64, for `b64"…"` literals.
pub fn encode_base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied();
        let b2 = chunk.get(2).copied();
        out.push(BASE64_ALPHABET[(b0 >> 2) as usize] as char);
        let idx1 = ((b0 & 0x03) << 4) | (b1.unwrap_or(0) >> 4);
        out.push(BASE64_ALPHABET[idx1 as usize] as char);
        match (b1, b2) {
            (Some(b1), Some(b2)) => {
                out.push(BASE64_ALPHABET[(((b1 & 0x0F) << 2) | (b2 >> 6)) as usize] as char);
                out.push(BASE64_ALPHABET[(b2 & 0x3F) as usize] as char);
            }
            (Some(b1), None) => {
                out.push(BASE64_ALPHABET[((b1 & 0x0F) << 2) as usize] as char);
                out.push('=');
            }
            (None, _) => {
                out.push('=');
                out.push('=');
            }
        }
    }
    out
}

/// Decodes a `b64"..."` literal body: standard RFC 4648 alphabet, required
/// padding, no embedded whitespace.
pub fn decode_base64_bytes(cur: &Cursor, text: &str) -> Result<Vec<u8>, Error> {
    let bytes = text.as_bytes();
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    if bytes.len() % 4 != 0 {
        return Err(cur.error("base64 byte literal length must be a multiple of 4"));
    }

    fn val(b: u8) -> Option<u8> {
        BASE64_ALPHABET.iter().position(|&a| a == b).map(|p| p as u8)
    }

    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    let mut chunks = bytes.chunks_exact(4).peekable();
    while let Some(chunk) = chunks.next() {
        let is_last = chunks.peek().is_none();
        let pad = chunk.iter().filter(|&&b| b == b'=').count();
        if pad > 0 && !is_last {
            return Err(cur.error("base64 padding may only appear at the end"));
        }
        match pad {
            0 => {
                let vals = [val(chunk[0]), val(chunk[1]), val(chunk[2]), val(chunk[3])];
                let [a, b, c, d] = vals.map(|v| v.ok_or_else(|| cur.error("invalid base64 character")));
                let (a, b, c, d) = (a?, b?, c?, d?);
                out.push((a << 2) | (b >> 4));
                out.push((b << 4) | (c >> 2));
                out.push((c << 6) | d);
            }
            1 => {
                if chunk[3] != b'=' {
                    return Err(cur.error("invalid base64 padding"));
                }
                let a = val(chunk[0]).ok_or_else(|| cur.error("invalid base64 character"))?;
                let b = val(chunk[1]).ok_or_else(|| cur.error("invalid base64 character"))?;
                let c = val(chunk[2]).ok_or_else(|| cur.error("invalid base64 character"))?;
                out.push((a << 2) | (b >> 4));
                out.push((b << 4) | (c >> 2));
            }
            2 => {
                if chunk[2] != b'=' || chunk[3] != b'=' {
                    return Err(cur.error("invalid base64 padding"));
                }
                let a = val(chunk[0]).ok_or_else(|| cur.error("invalid base64 character"))?;
                let b = val(chunk[1]).ok_or_else(|| cur.error("invalid base64 character"))?;
                out.push((a << 2) | (b >> 4));
            }
            _ => return Err(cur.error("invalid base64 padding")),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(input: &str, f: impl FnOnce(&mut Cursor) -> Result<String, Error>) -> Result<String, Error> {
        let mut cur = Cursor::new(input);
        f(&mut cur)
    }

    #[test]
    fn basic_escapes() {
        assert_eq!(run(r#""line\nbreak""#, scan_basic_string).unwrap(), "line\nbreak");
        assert_eq!(run(r#""tab\there""#, scan_basic_string).unwrap(), "tab\there");
    }

    #[test]
    fn unicode_escape() {
        assert_eq!(run(r#""\u{1F600}""#, scan_basic_string).unwrap(), "\u{1F600}");
    }

    #[test]
    fn unknown_escape_errors() {
        assert!(run(r#""\q""#, scan_basic_string).is_err());
    }

    #[test]
    fn unterminated_single_line_errors() {
        assert!(run("\"abc\ndef\"", scan_basic_string).is_err());
    }

    #[test]
    fn triple_drops_leading_newline() {
        assert_eq!(run("\"\"\"\nhello\"\"\"", scan_basic_string).unwrap(), "hello");
    }

    #[test]
    fn raw_no_escapes() {
        assert_eq!(run(r"'C:\path\no\escapes'", scan_raw_string).unwrap(), r"C:\path\no\escapes");
    }

    #[test]
    fn hex_bytes_roundtrip() {
        let cur = Cursor::new("");
        assert_eq!(decode_hex_bytes(&cur, "deadbeef").unwrap(), vec![0xde, 0xad, 0xbe, 0xef]);
        assert!(decode_hex_bytes(&cur, "abc").is_err());
    }

    #[test]
    fn base64_roundtrip() {
        let cur = Cursor::new("");
        assert_eq!(decode_base64_bytes(&cur, "aGVsbG8gd29ybGQ=").unwrap(), b"hello world");
        assert_eq!(decode_base64_bytes(&cur, "").unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn hex_encode_matches_decode() {
        let cur = Cursor::new("");
        let bytes = vec![0xde, 0xad, 0xbe, 0xef];
        assert_eq!(encode_hex(&bytes), "deadbeef");
        assert_eq!(decode_hex_bytes(&cur, &encode_hex(&bytes)).unwrap(), bytes);
    }

    #[test]
    fn base64_encode_matches_decode() {
        let cur = Cursor::new("");
        for bytes in [b"hello world".to_vec(), vec![1, 2, 3], vec![1, 2], vec![1], vec![]] {
            let encoded = encode_base64(&bytes);
            assert_eq!(decode_base64_bytes(&cur, &encoded).unwrap(), bytes);
        }
    }
}
