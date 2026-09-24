use std::collections::HashSet;

use saphyr::{LoadableYamlNode, ScalarOwned, YamlOwned};
use saphyr_parser::{Event, EventReceiver, Parser, Tag};

use crate::convert::check_depth;
use crate::convert::error::ConvertError;
use crate::value::{Int, Map, MAX_DEPTH, Value};

/// Imports YAML as a Tavra document root. Import-only, no export.
///
/// No `!!timestamp` auto-detection — bare dates come through as strings.
/// Anything that would lose data on import is an error: more than one
/// document, duplicate keys, aliases, and tags other than core ones.
pub fn from_yaml(source: &str) -> Result<Map, ConvertError> {
    prescan(source)?;
    let docs = YamlOwned::load_from_str(source).map_err(|e| ConvertError::new(e.to_string()))?;
    let Some(root) = docs.into_iter().next() else {
        return Err(ConvertError::new("YAML source has no documents"));
    };
    match yaml_to_value(root)? {
        Value::Map(m) => check_depth(m),
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
        YamlOwned::Tagged(tag, _) => return Err(ConvertError::new(format!("unsupported YAML tag {tag}"))),
        YamlOwned::Alias(_) => return Err(ConvertError::new("YAML aliases/anchors are not supported")),
        YamlOwned::Representation(..) | YamlOwned::BadValue => {
            return Err(ConvertError::new("unresolved or invalid YAML node"));
        }
    })
}

/// saphyr's loader drops duplicate keys, resolves aliases and discards
/// tags, so check raw events first.
fn prescan(source: &str) -> Result<(), ConvertError> {
    let mut scan = Prescan::default();
    Parser::new_from_str(source).load(&mut scan, true).map_err(|e| ConvertError::new(e.to_string()))?;
    scan.error.map_or(Ok(()), Err)
}

enum Frame {
    Seq,
    Map { keys: HashSet<String>, expect_key: bool },
}

#[derive(Default)]
struct Prescan {
    docs: usize,
    frames: Vec<Frame>,
    error: Option<ConvertError>,
}

impl Prescan {
    fn check_tag(tag: Option<&Tag>) -> Result<(), ConvertError> {
        const CORE: &[&str] = &["null", "bool", "int", "float", "str", "seq", "map"];
        match tag {
            Some(t) if !(t.is_yaml_core_schema() && CORE.contains(&t.suffix.as_str())) => {
                Err(ConvertError::new(format!("unsupported YAML tag {t}")))
            }
            _ => Ok(()),
        }
    }

    /// Called when a node ends. `key` is its text if it was a scalar.
    fn node_done(&mut self, key: Option<&str>) -> Result<(), ConvertError> {
        if let Some(Frame::Map { keys, expect_key }) = self.frames.last_mut() {
            if *expect_key {
                if let Some(k) = key {
                    if !keys.insert(k.to_string()) {
                        return Err(ConvertError::new(format!("duplicate key '{k}' in YAML mapping")));
                    }
                }
            }
            *expect_key = !*expect_key;
        }
        Ok(())
    }

    fn handle(&mut self, ev: Event<'_>) -> Result<(), ConvertError> {
        let is_seq = matches!(ev, Event::SequenceStart(..));
        match ev {
            Event::DocumentStart(_) => {
                self.docs += 1;
                if self.docs > 1 {
                    return Err(ConvertError::new("YAML source has more than one document"));
                }
            }
            Event::Alias(_) => return Err(ConvertError::new("YAML aliases/anchors are not supported")),
            Event::Scalar(value, _, _, tag) => {
                Self::check_tag(tag.as_deref())?;
                self.node_done(Some(&value))?;
            }
            Event::SequenceStart(_, tag) | Event::MappingStart(_, tag) => {
                Self::check_tag(tag.as_deref())?;
                // root map is frame 1, so allow one more than MAX_DEPTH
                if self.frames.len() > MAX_DEPTH as usize {
                    return Err(ConvertError::new(format!("nested deeper than {MAX_DEPTH}")));
                }
                self.frames.push(if is_seq { Frame::Seq } else { Frame::Map { keys: HashSet::new(), expect_key: true } });
            }
            Event::SequenceEnd | Event::MappingEnd => {
                self.frames.pop();
                self.node_done(None)?;
            }
            _ => {}
        }
        Ok(())
    }
}

impl<'input> EventReceiver<'input> for Prescan {
    fn on_event(&mut self, ev: Event<'input>) {
        if self.error.is_none() {
            if let Err(e) = self.handle(ev) {
                self.error = Some(e);
            }
        }
    }
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
    fn duplicate_keys_rejected() {
        assert!(from_yaml("a: 1\na: 2\n").is_err());
        assert!(from_yaml("a: 1\n\"a\": 2\n").is_err());
        assert!(from_yaml("m:\n  x: 1\n  x: 2\n").is_err());
        assert!(from_yaml("{a: 1, a: 2}\n").is_err());
        // same key in different maps is fine
        assert!(from_yaml("a: {x: 1}\nb: {x: 1}\nc: [{x: 1}, {x: 2}]\n").is_ok());
    }

    #[test]
    fn multiple_documents_rejected() {
        assert!(from_yaml("a: 1\n---\nb: 2\n").is_err());
        assert!(from_yaml("---\na: 1\n").is_ok());
    }

    #[test]
    fn aliases_rejected() {
        assert!(from_yaml("a: &x 1\nb: *x\n").is_err());
    }

    #[test]
    fn non_core_tags_rejected() {
        assert!(from_yaml("a: !custom 1\n").is_err());
        assert!(from_yaml("a: !!binary aGk=\n").is_err());
        assert!(from_yaml("a: !thing {x: 1}\n").is_err());
        assert_eq!(from_yaml("a: !!str 1\n").unwrap()["a"], Value::from("1"));
    }
}
