use std::fmt::{self, Formatter};

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};

use crate::convert::check_depth;
use crate::convert::error::ConvertError;
use crate::value::{Int, Map, Value};

/// Imports JSON as a Tavra document root. Import-only, no export.
/// Duplicate object keys are an error.
pub fn from_json(source: &str) -> Result<Map, ConvertError> {
    let Strict(value) = serde_json::from_str(source).map_err(|e| ConvertError::new(e.to_string()))?;
    match value {
        Value::Map(m) => check_depth(m),
        _ => Err(ConvertError::new("JSON root must be an object")),
    }
}

/// Builds `Value` straight from serde so duplicate keys can be seen.
/// `serde_json::Value` keeps last one.
struct Strict(Value);

impl<'de> Deserialize<'de> for Strict {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(StrictVisitor)
    }
}

struct StrictVisitor;

impl<'de> Visitor<'de> for StrictVisitor {
    type Value = Strict;

    fn expecting(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("a JSON value")
    }

    fn visit_unit<E: de::Error>(self) -> Result<Strict, E> {
        Ok(Strict(Value::Null))
    }

    fn visit_bool<E: de::Error>(self, b: bool) -> Result<Strict, E> {
        Ok(Strict(Value::Bool(b)))
    }

    fn visit_i64<E: de::Error>(self, i: i64) -> Result<Strict, E> {
        Ok(Strict(Value::Int(Int::from_i64(i))))
    }

    fn visit_u64<E: de::Error>(self, u: u64) -> Result<Strict, E> {
        Ok(Strict(Value::Int(Int::from_u64(u))))
    }

    fn visit_f64<E: de::Error>(self, f: f64) -> Result<Strict, E> {
        Ok(Strict(Value::from(f)))
    }

    fn visit_str<E: de::Error>(self, s: &str) -> Result<Strict, E> {
        Ok(Strict(Value::String(s.to_string())))
    }

    fn visit_string<E: de::Error>(self, s: String) -> Result<Strict, E> {
        Ok(Strict(Value::String(s)))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Strict, A::Error> {
        let mut items = Vec::new();
        while let Some(Strict(v)) = seq.next_element()? {
            items.push(v);
        }
        Ok(Strict(Value::Array(items)))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Strict, A::Error> {
        let mut map = Map::new();
        while let Some(key) = access.next_key::<String>()? {
            let Strict(v) = access.next_value()?;
            if map.contains_key(&key) {
                return Err(de::Error::custom(format!("duplicate key '{key}' in JSON object")));
            }
            map.insert(key, v);
        }
        Ok(Strict(Value::Map(map)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalars_map_correctly() {
        let doc = from_json(r#"{"a": null, "b": true, "c": 42, "d": -1, "e": 3.5, "f": "hi"}"#).unwrap();
        assert_eq!(doc["a"], Value::Null);
        assert_eq!(doc["b"], Value::from(true));
        assert_eq!(doc["c"], Value::from(42i64));
        assert_eq!(doc["d"], Value::from(-1i64));
        assert_eq!(doc["e"], Value::from(3.5f64));
        assert_eq!(doc["f"], Value::from("hi"));
    }

    #[test]
    fn nested_object_and_array() {
        let doc = from_json(r#"{"server": {"host": "0.0.0.0", "ports": [80, 443]}}"#).unwrap();
        let server = doc["server"].as_map().unwrap();
        assert_eq!(server["host"], Value::from("0.0.0.0"));
        assert_eq!(server["ports"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn non_object_root_rejected() {
        assert!(from_json("[1, 2, 3]").is_err());
        assert!(from_json("42").is_err());
    }

    #[test]
    fn malformed_json_rejected() {
        assert!(from_json("{not valid json").is_err());
    }

    #[test]
    fn duplicate_keys_rejected() {
        assert!(from_json(r#"{"a": 1, "a": 2}"#).is_err());
        assert!(from_json(r#"{"m": {"x": 1, "x": 1}}"#).is_err());
        assert!(from_json(r#"{"a": {"x": 1}, "b": [{"x": 1}, {"x": 2}]}"#).is_ok());
    }

    #[test]
    fn large_u64_preserved() {
        let doc = from_json(r#"{"n": 18446744073709551615}"#).unwrap();
        assert_eq!(doc["n"], Value::Int(Int::from_u64(u64::MAX)));
    }
}
