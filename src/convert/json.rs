use crate::convert::error::ConvertError;
use crate::value::{Int, Map, Value};

/// Imports JSON as a Tavra document root. Import-only, no export.
///
/// Duplicate object keys resolve last-wins (`serde_json`'s own behavior,
/// happens before we see the result).
pub fn from_json(source: &str) -> Result<Map, ConvertError> {
    let value: serde_json::Value = serde_json::from_str(source).map_err(|e| ConvertError::new(e.to_string()))?;
    match json_to_value(value) {
        Value::Map(m) => Ok(m),
        _ => Err(ConvertError::new("JSON root must be an object")),
    }
}

fn json_to_value(v: serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(b),
        serde_json::Value::Number(n) => json_number_to_value(&n),
        serde_json::Value::String(s) => Value::String(s),
        serde_json::Value::Array(items) => Value::Array(items.into_iter().map(json_to_value).collect()),
        serde_json::Value::Object(obj) => Value::Map(obj.into_iter().map(|(k, v)| (k, json_to_value(v))).collect()),
    }
}

fn json_number_to_value(n: &serde_json::Number) -> Value {
    if let Some(i) = n.as_i64() {
        Value::Int(Int::from_i64(i))
    } else if let Some(u) = n.as_u64() {
        Value::Int(Int::from_u64(u))
    } else {
        Value::from(n.as_f64().unwrap_or(f64::NAN))
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
    fn large_u64_preserved() {
        let doc = from_json(r#"{"n": 18446744073709551615}"#).unwrap();
        assert_eq!(doc["n"], Value::Int(Int::from_u64(u64::MAX)));
    }
}
