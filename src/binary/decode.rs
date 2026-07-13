use crate::value::{Date, Datetime, Float, Int, LocalDateTime, Map, OffsetDateTime, Time, Value};

use crate::binary::encode::{MAGIC, VERSION};
use crate::binary::error::Error;
use crate::binary::varint::decode_varint;

pub fn decode_document(bytes: &[u8]) -> Result<Map, Error> {
    let mut pos = 0;
    let magic = read_bytes(bytes, &mut pos, 4)?;
    if magic != MAGIC {
        return Err(Error::new(0, "bad magic: not a .tavb document"));
    }
    let version = read_u8(bytes, &mut pos)?;
    if version != VERSION {
        return Err(Error::new(pos - 1, format!("unsupported version {version}")));
    }
    let tag = read_u8(bytes, &mut pos)?;
    if tag != 0x0C {
        return Err(Error::new(pos - 1, format!("expected map tag 0x0C at document root, found 0x{tag:02X}")));
    }
    let root = decode_map(bytes, &mut pos)?;
    if pos != bytes.len() {
        return Err(Error::new(pos, "trailing bytes after document"));
    }
    Ok(root)
}

/// Decodes `value_bytes` — a document's root map with no magic/version
/// prefix (the form used by envelope payloads and document hashing).
pub fn decode_value_bytes(bytes: &[u8]) -> Result<Map, Error> {
    let mut pos = 0;
    let tag = read_u8(bytes, &mut pos)?;
    if tag != 0x0C {
        return Err(Error::new(pos - 1, format!("expected map tag 0x0C at document root, found 0x{tag:02X}")));
    }
    let root = decode_map(bytes, &mut pos)?;
    if pos != bytes.len() {
        return Err(Error::new(pos, "trailing bytes after document"));
    }
    Ok(root)
}

fn read_u8(bytes: &[u8], pos: &mut usize) -> Result<u8, Error> {
    let b = *bytes.get(*pos).ok_or_else(|| Error::new(*pos, "unexpected end of input"))?;
    *pos += 1;
    Ok(b)
}

fn read_bytes<'a>(bytes: &'a [u8], pos: &mut usize, n: usize) -> Result<&'a [u8], Error> {
    let end = pos.checked_add(n).ok_or_else(|| Error::new(*pos, "length overflow"))?;
    let slice = bytes.get(*pos..end).ok_or_else(|| Error::new(*pos, "unexpected end of input"))?;
    *pos = end;
    Ok(slice)
}

/// Reads exactly `N` bytes as a fixed-size array. `read_bytes` guarantees
/// the returned slice has length `N`, so `copy_from_slice` cannot panic.
fn read_array<const N: usize>(bytes: &[u8], pos: &mut usize) -> Result<[u8; N], Error> {
    let slice = read_bytes(bytes, pos, N)?;
    let mut arr = [0u8; N];
    arr.copy_from_slice(slice);
    Ok(arr)
}

fn decode_value(bytes: &[u8], pos: &mut usize) -> Result<Value, Error> {
    let tag = read_u8(bytes, pos)?;
    match tag {
        0x00 => Ok(Value::Null),
        0x01 => Ok(Value::Bool(false)),
        0x02 => Ok(Value::Bool(true)),
        0x03 => decode_int(bytes, pos),
        0x04 => decode_float(bytes, pos),
        0x05 => decode_string(bytes, pos).map(Value::String),
        0x06 => decode_bytes_value(bytes, pos),
        0x07 => decode_date(bytes, pos).map(|d| Value::Datetime(Datetime::Date(d))),
        0x08 => decode_time(bytes, pos).map(|t| Value::Datetime(Datetime::Time(t))),
        0x09 => decode_local_datetime(bytes, pos).map(|ldt| Value::Datetime(Datetime::Local(ldt))),
        0x0A => decode_offset_datetime(bytes, pos).map(|odt| Value::Datetime(Datetime::Offset(odt))),
        0x0B => decode_array(bytes, pos),
        0x0C => decode_map(bytes, pos).map(Value::Map),
        other => Err(Error::new(*pos - 1, format!("unknown or reserved tag 0x{other:02X}"))),
    }
}

fn decode_int(bytes: &[u8], pos: &mut usize) -> Result<Value, Error> {
    let kind = read_u8(bytes, pos)?;
    let magnitude = decode_varint(bytes, pos)?;
    match kind {
        0x00 => Ok(Value::Int(Int::from_u64(magnitude))),
        0x01 => {
            let v = -1i128 - i128::from(magnitude);
            let i = Int::from_i128(v).ok_or_else(|| Error::new(*pos, "int magnitude out of range"))?;
            Ok(Value::Int(i))
        }
        other => Err(Error::new(*pos - 1, format!("invalid int kind byte 0x{other:02X}"))),
    }
}

fn decode_float(bytes: &[u8], pos: &mut usize) -> Result<Value, Error> {
    let bits = u64::from_be_bytes(read_array(bytes, pos)?);
    let f = Float::from_bits_checked(bits).ok_or_else(|| Error::new(*pos - 8, "non-canonical NaN encoding"))?;
    Ok(Value::Float(f))
}

fn decode_string(bytes: &[u8], pos: &mut usize) -> Result<String, Error> {
    let len = decode_varint(bytes, pos)?;
    let len: usize = len.try_into().map_err(|_| Error::new(*pos, "string length too large"))?;
    let raw = read_bytes(bytes, pos, len)?;
    String::from_utf8(raw.to_vec()).map_err(|_| Error::new(*pos - len, "invalid UTF-8 in string"))
}

fn decode_bytes_value(bytes: &[u8], pos: &mut usize) -> Result<Value, Error> {
    let len = decode_varint(bytes, pos)?;
    let len: usize = len.try_into().map_err(|_| Error::new(*pos, "bytes length too large"))?;
    let raw = read_bytes(bytes, pos, len)?;
    Ok(Value::Bytes(raw.to_vec()))
}

fn decode_date(bytes: &[u8], pos: &mut usize) -> Result<Date, Error> {
    let year = u16::from_be_bytes(read_array(bytes, pos)?);
    let month = read_u8(bytes, pos)?;
    let day = read_u8(bytes, pos)?;
    Date::new(year, month, day).map_err(|_| Error::new(*pos - 4, "date out of range"))
}

fn decode_time(bytes: &[u8], pos: &mut usize) -> Result<Time, Error> {
    let start = *pos;
    let hour = read_u8(bytes, pos)?;
    let minute = read_u8(bytes, pos)?;
    let second = read_u8(bytes, pos)?;
    let nanosecond = u32::from_be_bytes(read_array(bytes, pos)?);
    Time::new(hour, minute, second, nanosecond).map_err(|_| Error::new(start, "time out of range"))
}

fn decode_local_datetime(bytes: &[u8], pos: &mut usize) -> Result<LocalDateTime, Error> {
    let date = decode_date(bytes, pos)?;
    let time = decode_time(bytes, pos)?;
    Ok(LocalDateTime { date, time })
}

fn decode_offset_datetime(bytes: &[u8], pos: &mut usize) -> Result<OffsetDateTime, Error> {
    let start = *pos;
    let datetime = decode_local_datetime(bytes, pos)?;
    let offset_minutes = i16::from_be_bytes(read_array(bytes, pos)?);
    OffsetDateTime::new(datetime, offset_minutes).map_err(|_| Error::new(start, "offset out of range"))
}

fn decode_array(bytes: &[u8], pos: &mut usize) -> Result<Value, Error> {
    let count = decode_varint(bytes, pos)?;
    let mut items = Vec::new();
    for _ in 0..count {
        items.push(decode_value(bytes, pos)?);
    }
    Ok(Value::Array(items))
}

fn decode_map(bytes: &[u8], pos: &mut usize) -> Result<Map, Error> {
    let count = decode_varint(bytes, pos)?;
    let mut map = Map::new();
    let mut last_key: Option<String> = None;
    for _ in 0..count {
        let key_start = *pos;
        let key = decode_string(bytes, pos)?;
        if let Some(prev) = &last_key {
            if key.as_bytes() <= prev.as_bytes() {
                return Err(Error::new(key_start, "map keys out of canonical order or duplicated"));
            }
        }
        let value = decode_value(bytes, pos)?;
        last_key = Some(key.clone());
        map.insert(key, value);
    }
    Ok(map)
}
