use crate::value::{Datetime, Map, Time, Value};

use crate::binary::varint::encode_varint;

pub const MAGIC: &[u8; 4] = b"TAVB";
pub const VERSION: u8 = 0x01;

pub fn encode_document(root: &Map) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    encode_map(&mut out, root);
    out
}

/// Encodes `root` without the magic/version prefix — this is the
/// `value_bytes` the document hash and Ed25519 signature are computed over.
pub fn encode_value_bytes(root: &Map) -> Vec<u8> {
    let mut out = Vec::new();
    encode_map(&mut out, root);
    out
}

fn encode_value(out: &mut Vec<u8>, v: &Value) {
    match v {
        Value::Null => out.push(0x00),
        Value::Bool(false) => out.push(0x01),
        Value::Bool(true) => out.push(0x02),
        Value::Int(i) => {
            out.push(0x03);
            let magnitude = i.as_i128();
            if magnitude >= 0 {
                out.push(0x00);
                encode_varint(out, magnitude as u64);
            } else {
                out.push(0x01);
                encode_varint(out, (-1 - magnitude) as u64);
            }
        }
        Value::Float(f) => {
            out.push(0x04);
            out.extend_from_slice(&f.to_bits().to_be_bytes());
        }
        Value::String(s) => {
            out.push(0x05);
            encode_varint(out, s.len() as u64);
            out.extend_from_slice(s.as_bytes());
        }
        Value::Bytes(b) => {
            out.push(0x06);
            encode_varint(out, b.len() as u64);
            out.extend_from_slice(b);
        }
        Value::Datetime(dt) => encode_datetime(out, dt),
        Value::Array(items) => {
            out.push(0x0B);
            encode_varint(out, items.len() as u64);
            for item in items {
                encode_value(out, item);
            }
        }
        Value::Map(m) => encode_map(out, m),
    }
}

fn encode_map(out: &mut Vec<u8>, map: &Map) {
    out.push(0x0C);
    encode_varint(out, map.len() as u64);
    // BTreeMap<String, _> iterates in byte-lexicographic key order already,
    // so no explicit sort is needed here.
    for (k, v) in map.iter() {
        encode_varint(out, k.len() as u64);
        out.extend_from_slice(k.as_bytes());
        encode_value(out, v);
    }
}

fn encode_datetime(out: &mut Vec<u8>, dt: &Datetime) {
    match dt {
        Datetime::Date(d) => {
            out.push(0x07);
            out.extend_from_slice(&d.year().to_be_bytes());
            out.push(d.month());
            out.push(d.day());
        }
        Datetime::Time(t) => {
            out.push(0x08);
            encode_time(out, t);
        }
        Datetime::Local(ldt) => {
            out.push(0x09);
            out.extend_from_slice(&ldt.date.year().to_be_bytes());
            out.push(ldt.date.month());
            out.push(ldt.date.day());
            encode_time(out, &ldt.time);
        }
        Datetime::Offset(odt) => {
            out.push(0x0A);
            let date = odt.datetime.date;
            out.extend_from_slice(&date.year().to_be_bytes());
            out.push(date.month());
            out.push(date.day());
            encode_time(out, &odt.datetime.time);
            out.extend_from_slice(&odt.offset_minutes().to_be_bytes());
        }
    }
}

fn encode_time(out: &mut Vec<u8>, t: &Time) {
    out.push(t.hour());
    out.push(t.minute());
    out.push(t.second());
    out.extend_from_slice(&t.nanosecond().to_be_bytes());
}
