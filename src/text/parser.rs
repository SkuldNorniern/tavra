use crate::value::{Map, Value};

use crate::text::cursor::Cursor;
use crate::text::error::Error;
use crate::text::number::scan_number_or_datetime;
use crate::text::string::{decode_base64_bytes, decode_hex_bytes, scan_basic_string, scan_raw_string};

pub fn parse_document(cur: &mut Cursor) -> Result<Value, Error> {
    let mut root = Map::new();

    skip_ws_and_comments(cur);
    while !cur.eof() {
        let key = parse_key(cur)?;
        skip_ws_line(cur);
        if cur.bump() != Some('=') {
            return Err(cur.error("expected '='"));
        }
        skip_ws_line(cur);
        let value = parse_value(cur)?;
        if root.insert(key.clone(), value).is_some() {
            return Err(cur.error(format!("duplicate key '{key}'")));
        }
        skip_ws_line(cur);
        expect_line_end(cur)?;
        skip_ws_and_comments(cur);
    }

    Ok(Value::Map(root))
}

fn parse_key(cur: &mut Cursor) -> Result<String, Error> {
    match cur.peek() {
        Some('"') => scan_basic_string(cur),
        Some('\'') => scan_raw_string(cur),
        Some(c) if is_bare_key_char(c) => {
            let mut s = String::new();
            while let Some(c) = cur.peek() {
                if is_bare_key_char(c) {
                    s.push(c);
                    cur.bump();
                } else {
                    break;
                }
            }
            Ok(s)
        }
        _ => Err(cur.error("expected a key")),
    }
}

pub(crate) fn is_bare_key_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

pub fn parse_value(cur: &mut Cursor) -> Result<Value, Error> {
    match cur.peek() {
        Some('"') => Ok(Value::String(scan_basic_string(cur)?)),
        Some('\'') => Ok(Value::String(scan_raw_string(cur)?)),
        Some('[') => parse_array(cur),
        Some('{') => parse_inline_map(cur),
        Some('b') => parse_bytes(cur),
        Some(c) if c.is_ascii_digit() => scan_number_or_datetime(cur),
        Some('-') if cur.peek_at(1).is_some_and(|c| c.is_ascii_digit()) => scan_number_or_datetime(cur),
        Some('-') if cur.peek_at(1) == Some('i') => parse_keyword(cur),
        Some(c) if c.is_ascii_alphabetic() => parse_keyword(cur),
        _ => Err(cur.error("expected a value")),
    }
}

fn parse_bytes(cur: &mut Cursor) -> Result<Value, Error> {
    cur.bump(); // 'b'
    let base64 = if cur.peek() == Some('"') {
        false
    } else if cur.peek() == Some('6') && cur.peek_at(1) == Some('4') && cur.peek_at(2) == Some('"') {
        cur.bump();
        cur.bump();
        true
    } else {
        return Err(cur.error("invalid bytes literal"));
    };
    cur.bump(); // opening '"'
    let mut body = String::new();
    loop {
        match cur.peek() {
            Some('"') => {
                cur.bump();
                break;
            }
            Some(c) if c != '\n' && c != '\r' => {
                body.push(c);
                cur.bump();
            }
            _ => return Err(cur.error("unterminated bytes literal")),
        }
    }
    let bytes = if base64 { decode_base64_bytes(cur, &body)? } else { decode_hex_bytes(cur, &body)? };
    Ok(Value::Bytes(bytes))
}

fn parse_keyword(cur: &mut Cursor) -> Result<Value, Error> {
    let mut s = String::new();
    if cur.peek() == Some('-') {
        s.push('-');
        cur.bump();
    }
    while let Some(c) = cur.peek() {
        if c.is_ascii_alphabetic() {
            s.push(c);
            cur.bump();
        } else {
            break;
        }
    }
    match s.as_str() {
        "true" => Ok(Value::from(true)),
        "false" => Ok(Value::from(false)),
        "null" => Ok(Value::Null),
        "inf" => Ok(Value::from(f64::INFINITY)),
        "-inf" => Ok(Value::from(f64::NEG_INFINITY)),
        "nan" => Ok(Value::from(f64::NAN)),
        other => Err(cur.error(format!("unknown literal '{other}'"))),
    }
}

fn parse_array(cur: &mut Cursor) -> Result<Value, Error> {
    cur.enter()?;
    let parsed = parse_array_body(cur);
    cur.leave();
    parsed
}

fn parse_array_body(cur: &mut Cursor) -> Result<Value, Error> {
    cur.bump(); // '['
    let mut items = Vec::new();
    skip_separators(cur);
    if cur.peek() == Some(']') {
        cur.bump();
        return Ok(Value::Array(items));
    }
    loop {
        items.push(parse_value(cur)?);
        let had_sep = skip_separators(cur);
        match cur.peek() {
            Some(']') => {
                cur.bump();
                return Ok(Value::Array(items));
            }
            Some(_) if had_sep => continue,
            _ => return Err(cur.error("expected ',', a line break, or ']'")),
        }
    }
}

fn parse_inline_map(cur: &mut Cursor) -> Result<Value, Error> {
    cur.enter()?;
    let parsed = parse_inline_map_body(cur);
    cur.leave();
    parsed
}

fn parse_inline_map_body(cur: &mut Cursor) -> Result<Value, Error> {
    cur.bump(); // '{'
    let mut map = Map::new();
    skip_separators(cur);
    if cur.peek() == Some('}') {
        cur.bump();
        return Ok(Value::Map(map));
    }
    loop {
        let key = parse_key(cur)?;
        skip_ws_and_comments(cur);
        if cur.bump() != Some('=') {
            return Err(cur.error("expected '='"));
        }
        skip_ws_and_comments(cur);
        let value = parse_value(cur)?;
        if map.insert(key.clone(), value).is_some() {
            return Err(cur.error(format!("duplicate key '{key}'")));
        }
        let had_sep = skip_separators(cur);
        match cur.peek() {
            Some('}') => {
                cur.bump();
                return Ok(Value::Map(map));
            }
            Some(_) if had_sep => continue,
            _ => return Err(cur.error("expected ',', a line break, or '}'")),
        }
    }
}

fn skip_ws_line(cur: &mut Cursor) {
    while matches!(cur.peek(), Some(' ') | Some('\t')) {
        cur.bump();
    }
}

fn skip_ws_and_comments(cur: &mut Cursor) {
    loop {
        match cur.peek() {
            Some(' ') | Some('\t') | Some('\n') | Some('\r') => {
                cur.bump();
            }
            Some('#') => {
                while cur.peek().is_some() && cur.peek() != Some('\n') {
                    cur.bump();
                }
            }
            _ => break,
        }
    }
}

/// Consumes whitespace, comments, commas, and line breaks inside `[]`/`{}`
/// (any mixture, in any order), returning whether at least one comma or
/// line break was seen — spaces/tabs/comments alone don't count as an
/// entry separator, only accompany one.
fn skip_separators(cur: &mut Cursor) -> bool {
    let mut found = false;
    loop {
        match cur.peek() {
            Some(' ') | Some('\t') => {
                cur.bump();
            }
            Some('#') => {
                while cur.peek().is_some() && cur.peek() != Some('\n') {
                    cur.bump();
                }
            }
            Some('\n') | Some('\r') | Some(',') => {
                cur.bump();
                found = true;
            }
            _ => break,
        }
    }
    found
}

fn expect_line_end(cur: &mut Cursor) -> Result<(), Error> {
    if cur.peek() == Some('#') {
        while cur.peek().is_some() && cur.peek() != Some('\n') {
            cur.bump();
        }
    }
    match cur.peek() {
        None => Ok(()),
        Some('\n') | Some('\r') => Ok(()),
        _ => Err(cur.error("expected end of line")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nesting_deeper_than_the_limit_is_refused() {
        use crate::text::parse;
        let inside_limit = format!("a = {}{}", "[".repeat(120), "]".repeat(120));
        assert!(parse(&inside_limit).is_ok());

        let past_limit = format!("a = {}{}", "[".repeat(200), "]".repeat(200));
        let error = parse(&past_limit).unwrap_err();
        assert!(error.to_string().contains("nested deeper"), "{error}");

        let maps = format!("a = {}{}", "{b = ".repeat(200), "}".repeat(200));
        assert!(parse(&maps).is_err());

        let unclosed = "[".repeat(50_000);
        assert!(parse(&unclosed).is_err());
    }
    use crate::text::parse;

    fn get<'a>(doc: &'a Value, path: &[&str]) -> &'a Value {
        let mut cur = doc;
        for key in path {
            cur = cur.as_map().and_then(|m| m.get(*key)).expect("missing key");
        }
        cur
    }

    #[test]
    fn root_level_assignments() {
        let doc = parse("name = \"demo\"\nport = 8080\n").unwrap();
        assert_eq!(get(&doc, &["name"]), &Value::from("demo"));
        assert_eq!(get(&doc, &["port"]), &Value::from(8080i64));
    }

    #[test]
    fn nested_inline_maps() {
        let doc = parse("server = {\n    host = \"0.0.0.0\"\n\n    tls = {\n        cert = b\"deadbeef\"\n    }\n}\n").unwrap();
        assert_eq!(get(&doc, &["server", "host"]), &Value::from("0.0.0.0"));
        assert_eq!(get(&doc, &["server", "tls", "cert"]), &Value::Bytes(vec![0xde, 0xad, 0xbe, 0xef]));
    }

    #[test]
    fn comments_and_blank_lines_ignored() {
        let doc = parse("# comment\n\na = 1  # trailing\n\n").unwrap();
        assert_eq!(get(&doc, &["a"]), &Value::from(1i64));
    }

    #[test]
    fn duplicate_key_rejected() {
        assert!(parse("a = 1\na = 2\n").is_err());
    }

    #[test]
    fn arrays_and_inline_maps() {
        let doc = parse(
            "servers = [\n    { name = \"alpha\", port = 8001 },\n    { name = \"beta\", port = 8002 },\n]\n",
        )
        .unwrap();
        let arr = get(&doc, &["servers"]).as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(get(&arr[0], &["name"]), &Value::from("alpha"));
    }

    #[test]
    fn inline_map_duplicate_key_rejected() {
        assert!(parse("a = { x = 1, x = 2 }\n").is_err());
    }

    #[test]
    fn trailing_comma_allowed() {
        let doc = parse("a = [1, 2, 3,]\n").unwrap();
        assert_eq!(get(&doc, &["a"]).as_array().unwrap().len(), 3);
    }

    #[test]
    fn newline_separated_array_no_commas() {
        let doc = parse("a = [\n    1\n    2\n    3\n]\n").unwrap();
        let arr = get(&doc, &["a"]).as_array().unwrap();
        assert_eq!(arr, &[Value::from(1i64), Value::from(2i64), Value::from(3i64)]);
    }

    #[test]
    fn newline_separated_inline_map_no_commas() {
        let doc = parse("a = {\n    x = 1\n    y = 2\n}\n").unwrap();
        assert_eq!(get(&doc, &["a", "x"]), &Value::from(1i64));
        assert_eq!(get(&doc, &["a", "y"]), &Value::from(2i64));
    }

    #[test]
    fn missing_separator_rejected() {
        assert!(parse("a = [1 2]\n").is_err());
        assert!(parse("a = { x = 1 y = 2 }\n").is_err());
    }

    #[test]
    fn dotted_key_in_assignment_rejected() {
        assert!(parse("a.b = 1\n").is_err());
    }

    #[test]
    fn quoted_key_with_spaces() {
        let doc = parse("\"key with spaces\" = 1\n").unwrap();
        assert_eq!(get(&doc, &["key with spaces"]), &Value::from(1i64));
    }

    #[test]
    fn header_syntax_rejected() {
        assert!(parse("[server]\nhost = \"x\"\n").is_err());
    }
}
