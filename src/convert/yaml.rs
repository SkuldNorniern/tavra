use saphyr::{LoadableYamlNode, ScalarOwned, YamlOwned};

use crate::convert::error::ConvertError;
use crate::value::{Int, Map, Value};

/// Imports a YAML document as a Tavra document root. Import-only — there is
/// no export back to YAML.
///
/// Scalars map to exactly Tavra's null/bool/int/float/string — YAML's
/// implicit `!!timestamp` typing isn't specially recognized, so a bare
/// date/time in YAML source comes through as a string, not a `Datetime`.
/// Only the first YAML document in the source is used (YAML allows several
/// `---`-separated documents per file; multiple documents is treated as
/// "use the first," matching the common case).
///
/// Duplicate mapping keys are resolved last-wins by `saphyr`'s loader
/// (a plain overwrite-on-insert, same as `serde_json`) before this function
/// ever sees the result — the explicit duplicate check below only catches
/// cases that somehow survive to here, and won't fire for a plain literal
/// duplicate key in the source.
pub fn from_yaml(source: &str) -> Result<Map, ConvertError> {
    let docs = YamlOwned::load_from_str(source).map_err(|e| ConvertError::new(e.to_string()))?;
    let Some(root) = docs.into_iter().next() else {
        return Err(ConvertError::new("YAML source has no documents"));
    };
    match yaml_to_value(root)? {
        Value::Map(m) => Ok(m),
        _ => Err(ConvertError::new("YAML root must be a mapping")),
    }
}

fn yaml_to_value(y: YamlOwned) -> Result<Value, ConvertError> {
    Ok(match y {
        YamlOwned::Value(scalar) => scalar_to_value(scalar),
        YamlOwned::Sequence(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(yaml_to_value(item)?);
            }
            Value::Array(out)
        }
        YamlOwned::Mapping(mapping) => {
            let mut out = Map::new();
            for (k, v) in mapping {
                let key = match k {
                    YamlOwned::Value(ScalarOwned::String(s)) => s,
                    _ => return Err(ConvertError::new("YAML mapping keys must be strings")),
                };
                let value = yaml_to_value(v)?;
                if out.insert(key.clone(), value).is_some() {
                    return Err(ConvertError::new(format!("duplicate key '{key}' in YAML mapping")));
                }
            }
            Value::Map(out)
        }
        // A tag doesn't change the resolved value in Tavra's simpler type
        // system — unwrap and convert what it tags.
        YamlOwned::Tagged(_, inner) => yaml_to_value(*inner)?,
        YamlOwned::Alias(_) => return Err(ConvertError::new("YAML aliases/anchors are not supported")),
        YamlOwned::Representation(..) | YamlOwned::BadValue => {
            return Err(ConvertError::new("unresolved or invalid YAML node"));
        }
    })
}

fn scalar_to_value(s: ScalarOwned) -> Value {
    match s {
        ScalarOwned::Null => Value::Null,
        ScalarOwned::Boolean(b) => Value::Bool(b),
        ScalarOwned::Integer(i) => Value::Int(Int::from_i64(i)),
        ScalarOwned::FloatingPoint(f) => Value::from(f.into_inner()),
        ScalarOwned::String(s) => Value::String(s),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalars_map_correctly() {
        let doc = from_yaml("a: null\nb: true\nc: 42\nd: 3.5\ne: hi\n").unwrap();
        assert_eq!(doc["a"], Value::Null);
        assert_eq!(doc["b"], Value::from(true));
        assert_eq!(doc["c"], Value::from(42i64));
        assert_eq!(doc["d"], Value::from(3.5f64));
        assert_eq!(doc["e"], Value::from("hi"));
    }

    #[test]
    fn nested_mapping_and_sequence() {
        let doc = from_yaml("server:\n  host: 0.0.0.0\n  ports:\n    - 80\n    - 443\n").unwrap();
        let server = doc["server"].as_map().unwrap();
        assert_eq!(server["host"], Value::from("0.0.0.0"));
        assert_eq!(server["ports"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn non_mapping_root_rejected() {
        assert!(from_yaml("- 1\n- 2\n").is_err());
        assert!(from_yaml("42\n").is_err());
    }

    #[test]
    fn non_string_key_rejected() {
        assert!(from_yaml("1: one\n2: two\n").is_err());
    }

    #[test]
    fn malformed_yaml_rejected() {
        assert!(from_yaml("a: [unclosed\n").is_err());
    }

    #[test]
    fn duplicate_keys_resolve_last_wins() {
        // saphyr's loader overwrites on insert, like serde_json — this
        // documents the observed behavior rather than asserting we can
        // catch something already resolved upstream.
        let doc = from_yaml("a: 1\na: 2\n").unwrap();
        assert_eq!(doc["a"], Value::from(2i64));
    }
}
