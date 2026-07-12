use std::collections::HashSet;

use crate::value::{Map, Value};

use super::cursor::Cursor;
use super::error::Error;
use super::number::scan_number_or_datetime;
use super::string::{decode_base64_bytes, decode_hex_bytes, scan_basic_string, scan_raw_string};

pub fn parse_document(cur: &mut Cursor) -> Result<Value, Error> {
    let mut root = Map::new();
    let mut current_path: Vec<String> = Vec::new();
    let mut explicit_tables: HashSet<Vec<String>> = HashSet::new();

    skip_ws_and_comments(cur);
    while !cur.eof() {
        if cur.peek() == Some('[') {
            current_path = parse_header(cur)?;
            navigate(cur, &mut root, &current_path)?;
            if !explicit_tables.insert(current_path.clone()) {
                return Err(cur.error(format!("table '{}' redefined", current_path.join("."))));
            }
        } else {
            let key = parse_key(cur)?;
            skip_ws_line(cur);
            if cur.bump() != Some('=') {
                return Err(cur.error("expected '='"));
            }
            skip_ws_line(cur);
            let value = parse_value(cur)?;
            let target = navigate(cur, &mut root, &current_path)?;
            if target.insert(key.clone(), value).is_some() {
                return Err(cur.error(format!("duplicate key '{key}'")));
            }
        }
        skip_ws_line(cur);
        expect_line_end(cur)?;
        skip_ws_and_comments(cur);
    }

    Ok(Value::Map(root))
}

fn navigate<'a>(cur: &Cursor, root: &'a mut Map, path: &[String]) -> Result<&'a mut Map, Error> {
    let mut current = root;
    for key in path {
        let entry = current.entry(key.clone()).or_insert_with(|| Value::Map(Map::new()));
        match entry {
            Value::Map(m) => current = m,
            _ => return Err(cur.error(format!("key '{key}' is not a table"))),
        }
    }
    Ok(current)
}

fn parse_header(cur: &mut Cursor) -> Result<Vec<String>, Error> {
    cur.bump(); // '['
    skip_ws_line(cur);
    let mut path = vec![parse_key(cur)?];
    loop {
        skip_ws_line(cur);
        if cur.peek() == Some('.') {
            cur.bump();
            skip_ws_line(cur);
            path.push(parse_key(cur)?);
        } else {
            break;
        }
    }
    skip_ws_line(cur);
    if cur.bump() != Some(']') {
        return Err(cur.error("expected ']'"));
    }
    Ok(path)
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

fn is_bare_key_char(c: char) -> bool {
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
    cur.bump(); // '['
    let mut items = Vec::new();
    skip_ws_and_comments(cur);
    if cur.peek() == Some(']') {
        cur.bump();
        return Ok(Value::Array(items));
    }
    loop {
        items.push(parse_value(cur)?);
        skip_ws_and_comments(cur);
        match cur.peek() {
            Some(',') => {
                cur.bump();
                skip_ws_and_comments(cur);
                if cur.peek() == Some(']') {
                    cur.bump();
                    return Ok(Value::Array(items));
                }
            }
            Some(']') => {
                cur.bump();
                return Ok(Value::Array(items));
            }
            _ => return Err(cur.error("expected ',' or ']'")),
        }
    }
}

fn parse_inline_map(cur: &mut Cursor) -> Result<Value, Error> {
    cur.bump(); // '{'
    let mut map = Map::new();
    skip_ws_and_comments(cur);
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
        skip_ws_and_comments(cur);
        match cur.peek() {
            Some(',') => {
                cur.bump();
                skip_ws_and_comments(cur);
                if cur.peek() == Some('}') {
                    cur.bump();
                    return Ok(Value::Map(map));
                }
            }
            Some('}') => {
                cur.bump();
                return Ok(Value::Map(map));
            }
            _ => return Err(cur.error("expected ',' or '}'")),
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
    fn nested_headers() {
        let doc = parse(
            "[server]\nhost = \"0.0.0.0\"\n\n[server.tls]\ncert = b\"deadbeef\"\n",
        )
        .unwrap();
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
    fn table_redefinition_rejected() {
        assert!(parse("[server]\na = 1\n[server]\nb = 2\n").is_err());
    }

    #[test]
    fn header_collides_with_scalar() {
        assert!(parse("a = 1\n[a]\nb = 2\n").is_err());
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
    fn dotted_key_in_assignment_rejected() {
        assert!(parse("a.b = 1\n").is_err());
    }

    #[test]
    fn quoted_key_with_spaces() {
        let doc = parse("\"key with spaces\" = 1\n").unwrap();
        assert_eq!(get(&doc, &["key with spaces"]), &Value::from(1i64));
    }
}
