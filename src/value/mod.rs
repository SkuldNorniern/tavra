//! The Tavra value model: the single in-memory representation shared by the
//! text format, canonical binary, and envelope layers.

mod datetime;
mod float;
mod int;

pub use datetime::{Date, Datetime, DatetimeError, LocalDateTime, OffsetDateTime, Time};
pub use float::Float;
pub use int::Int;

use std::collections::BTreeMap;

/// Map keys are strings; `BTreeMap`'s `str` ordering is byte-lexicographic
/// over UTF-8, which is exactly the canonical key order.
pub type Map = BTreeMap<String, Value>;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Value {
    Null,
    Bool(bool),
    Int(Int),
    Float(Float),
    String(String),
    Bytes(Vec<u8>),
    Datetime(Datetime),
    Array(Vec<Value>),
    Map(Map),
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Bytes(_) => "bytes",
            Value::Datetime(_) => "datetime",
            Value::Array(_) => "array",
            Value::Map(_) => "map",
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<Int> {
        match self {
            Value::Int(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(v) => Some(v.get()),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::Bytes(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_datetime(&self) -> Option<&Datetime> {
        match self {
            Value::Datetime(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&Map> {
        match self {
            Value::Map(v) => Some(v),
            _ => None,
        }
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Value {
        Value::Bool(v)
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Value {
        Value::Int(Int::from_i64(v))
    }
}

impl From<u64> for Value {
    fn from(v: u64) -> Value {
        Value::Int(Int::from_u64(v))
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Value {
        Value::Float(Float::new(v))
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Value {
        Value::String(v.to_owned())
    }
}

impl From<String> for Value {
    fn from(v: String) -> Value {
        Value::String(v)
    }
}

impl From<Vec<u8>> for Value {
    fn from(v: Vec<u8>) -> Value {
        Value::Bytes(v)
    }
}

impl From<Datetime> for Value {
    fn from(v: Datetime) -> Value {
        Value::Datetime(v)
    }
}

impl From<Vec<Value>> for Value {
    fn from(v: Vec<Value>) -> Value {
        Value::Array(v)
    }
}

impl From<Map> for Value {
    fn from(v: Map) -> Value {
        Value::Map(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_and_float_never_equal() {
        assert_ne!(Value::from(1i64), Value::from(1.0));
    }

    #[test]
    fn nan_equals_nan_in_values() {
        assert_eq!(Value::from(f64::NAN), Value::from(-f64::NAN));
    }

    #[test]
    fn map_iterates_byte_lexicographic() {
        let mut map = Map::new();
        map.insert("b".into(), Value::Null);
        map.insert("a".into(), Value::Null);
        map.insert("aa".into(), Value::Null);
        // "é" is 0xC3 0xA9 in UTF-8 — sorts after all ASCII keys.
        map.insert("é".into(), Value::Null);
        let keys: Vec<&str> = map.keys().map(String::as_str).collect();
        assert_eq!(keys, ["a", "aa", "b", "é"]);
    }

    #[test]
    fn map_equality_ignores_insertion_order() {
        let mut m1 = Map::new();
        m1.insert("x".into(), Value::from(1i64));
        m1.insert("y".into(), Value::from(2i64));
        let mut m2 = Map::new();
        m2.insert("y".into(), Value::from(2i64));
        m2.insert("x".into(), Value::from(1i64));
        assert_eq!(Value::Map(m1), Value::Map(m2));
    }
}
