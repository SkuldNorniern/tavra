//! `tav fmt` — canonical text rendering.
//!
//! `Map` (`BTreeMap`) does not retain source key order, so output is always
//! sorted byte-lexicographically rather than matching the input file; this
//! makes `format` idempotent and independent of how the document was
//! written.

use std::fmt::Write as _;

use crate::value::{Datetime, Time};
use crate::value::{Map, Value};

use super::parser::is_bare_key_char;
use super::string::{encode_base64, encode_hex};

/// Renders a document. `root` is the value model's root map (every document
/// root is a map).
pub fn format(root: &Map) -> String {
    let mut out = String::new();
    write_table_body(&mut out, root);
    write_nested_tables(&mut out, root, &mut Vec::new());
    out
}

fn write_table_body(out: &mut String, map: &Map) {
    for (k, v) in map.iter() {
        if matches!(v, Value::Map(_)) {
            continue;
        }
        write_key(out, k);
        out.push_str(" = ");
        write_value(out, v, 0);
        out.push('\n');
    }
}

fn write_nested_tables(out: &mut String, map: &Map, path: &mut Vec<String>) {
    for (k, v) in map.iter() {
        if let Value::Map(child) = v {
            path.push(k.clone());
            if !out.is_empty() {
                out.push('\n');
            }
            write_header(out, path);
            write_table_body(out, child);
            write_nested_tables(out, child, path);
            path.pop();
        }
    }
}

fn write_header(out: &mut String, path: &[String]) {
    out.push('[');
    for (i, segment) in path.iter().enumerate() {
        if i > 0 {
            out.push('.');
        }
        write_key(out, segment);
    }
    out.push_str("]\n");
}

fn write_key(out: &mut String, key: &str) {
    if !key.is_empty() && key.chars().all(is_bare_key_char) {
        out.push_str(key);
    } else {
        write_single_line_string(out, key);
    }
}

fn write_value(out: &mut String, v: &Value, indent: usize) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Int(i) => {
            let _ = write!(out, "{i}");
        }
        Value::Float(f) => write_float(out, f.get()),
        Value::String(s) => write_string(out, s),
        Value::Bytes(b) => write_bytes(out, b),
        Value::Datetime(dt) => write_datetime(out, dt),
        Value::Array(items) => write_array(out, items, indent),
        Value::Map(m) => write_inline_map(out, m, indent),
    }
}

fn write_indent(out: &mut String, level: usize) {
    for _ in 0..level {
        out.push_str("    ");
    }
}

fn write_array(out: &mut String, items: &[Value], indent: usize) {
    if items.is_empty() {
        out.push_str("[]");
        return;
    }
    out.push_str("[\n");
    for item in items {
        write_indent(out, indent + 1);
        write_value(out, item, indent + 1);
        out.push_str(",\n");
    }
    write_indent(out, indent);
    out.push(']');
}

fn write_inline_map(out: &mut String, map: &Map, indent: usize) {
    if map.is_empty() {
        out.push_str("{}");
        return;
    }
    out.push_str("{\n");
    for (k, v) in map.iter() {
        write_indent(out, indent + 1);
        write_key(out, k);
        out.push_str(" = ");
        write_value(out, v, indent + 1);
        out.push_str(",\n");
    }
    write_indent(out, indent);
    out.push('}');
}

fn write_float(out: &mut String, f: f64) {
    if f.is_nan() {
        out.push_str("nan");
        return;
    }
    if f.is_infinite() {
        out.push_str(if f > 0.0 { "inf" } else { "-inf" });
        return;
    }
    let s = format!("{f}");
    out.push_str(&s);
    if !s.contains('.') && !s.contains('e') && !s.contains('E') {
        out.push_str(".0");
    }
}

fn write_string(out: &mut String, s: &str) {
    if s.contains('\n') {
        write_multiline_string(out, s);
    } else {
        write_single_line_string(out, s);
    }
}

fn write_single_line_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        // A single-line string can't contain a raw line break at all (the
        // parser treats one as "unterminated string"), so \n and \r must
        // always be escaped here, unlike in the multi-line form.
        match c {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c => push_escaped_char(out, c),
        }
    }
    out.push('"');
}

fn write_multiline_string(out: &mut String, s: &str) {
    out.push_str("\"\"\"\n");
    for c in s.chars() {
        match c {
            '\n' => out.push('\n'),
            '\r' => out.push_str("\\r"),
            c => push_escaped_char(out, c),
        }
    }
    out.push_str("\"\"\"");
}

/// Escapes shared by single-line and multi-line basic strings, for
/// characters other than the line-break handling that differs between the
/// two forms. `"` and `\` are always escaped, so multi-line bodies can
/// never contain a literal `"""` that would close the string early.
fn push_escaped_char(out: &mut String, c: char) {
    match c {
        '\\' => out.push_str("\\\\"),
        '"' => out.push_str("\\\""),
        '\t' => out.push('\t'),
        c if c.is_control() => {
            let _ = write!(out, "\\u{{{:x}}}", c as u32);
        }
        c => out.push(c),
    }
}

fn write_bytes(out: &mut String, bytes: &[u8]) {
    if bytes.len() <= 16 {
        out.push_str("b\"");
        out.push_str(&encode_hex(bytes));
        out.push('"');
    } else {
        out.push_str("b64\"");
        out.push_str(&encode_base64(bytes));
        out.push('"');
    }
}

fn write_datetime(out: &mut String, dt: &Datetime) {
    match dt {
        Datetime::Date(d) => {
            let _ = write!(out, "{:04}-{:02}-{:02}", d.year(), d.month(), d.day());
        }
        Datetime::Time(t) => write_time(out, t),
        Datetime::Local(ldt) => {
            let _ = write!(out, "{:04}-{:02}-{:02}", ldt.date.year(), ldt.date.month(), ldt.date.day());
            out.push('T');
            write_time(out, &ldt.time);
        }
        Datetime::Offset(odt) => {
            let d = odt.datetime.date;
            let _ = write!(out, "{:04}-{:02}-{:02}", d.year(), d.month(), d.day());
            out.push('T');
            write_time(out, &odt.datetime.time);
            write_offset(out, odt.offset_minutes());
        }
    }
}

fn write_time(out: &mut String, t: &Time) {
    let _ = write!(out, "{:02}:{:02}:{:02}", t.hour(), t.minute(), t.second());
    if t.nanosecond() != 0 {
        let digits = format!("{:09}", t.nanosecond());
        let trimmed = digits.trim_end_matches('0');
        out.push('.');
        out.push_str(trimmed);
    }
}

fn write_offset(out: &mut String, minutes: i16) {
    if minutes == 0 {
        out.push('Z');
        return;
    }
    let sign = if minutes < 0 { '-' } else { '+' };
    let abs = minutes.unsigned_abs();
    let _ = write!(out, "{sign}{:02}:{:02}", abs / 60, abs % 60);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::parse;

    fn roundtrip(input: &str) -> Value {
        let doc = parse(input).unwrap();
        let formatted = format(doc.as_map().unwrap());
        let reparsed = parse(&formatted).unwrap();
        assert_eq!(doc, reparsed, "not idempotent:\n---\n{formatted}\n---");
        reparsed
    }

    #[test]
    fn float_always_has_decimal_point() {
        let doc = parse("a = 3.0\n").unwrap();
        let out = format(doc.as_map().unwrap());
        assert!(out.contains("3.0"), "{out}");
    }

    #[test]
    fn nested_tables_round_trip() {
        roundtrip("[server]\nhost = \"0.0.0.0\"\n\n[server.tls]\ncert = b\"deadbeef\"\n");
    }

    #[test]
    fn arrays_of_inline_maps_round_trip() {
        roundtrip("servers = [\n    { name = \"alpha\", port = 8001 },\n    { name = \"beta\", port = 8002 },\n]\n");
    }

    #[test]
    fn bytes_threshold_hex_vs_base64() {
        let doc = parse("a = b\"00112233445566778899aabbccddeeff00\"\n").unwrap();
        let out = format(doc.as_map().unwrap());
        assert!(out.contains("b64\""), "{out}");

        let doc = parse("a = b\"deadbeef\"\n").unwrap();
        let out = format(doc.as_map().unwrap());
        assert!(out.contains("b\"deadbeef\""), "{out}");
    }

    #[test]
    fn datetimes_round_trip() {
        roundtrip("a = 2026-07-12T10:30:00.5+09:00\nb = 2026-07-12\nc = 10:30:00\nd = 2026-07-12T10:30:00\n");
    }

    #[test]
    fn multiline_string_round_trips() {
        roundtrip("a = \"line one\\nline two\"\n");
    }

    #[test]
    fn quoted_key_round_trips() {
        roundtrip("\"key with spaces\" = 1\n");
    }

    #[test]
    fn format_is_idempotent() {
        let doc = parse("b = 1\na = [1, 2, { x = 1 }]\n[t]\nk = \"v\"\n").unwrap();
        let once = format(doc.as_map().unwrap());
        let twice = format(parse(&once).unwrap().as_map().unwrap());
        assert_eq!(once, twice);
    }
}
