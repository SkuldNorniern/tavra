use std::collections::BTreeMap;

use crate::value::{Map, Value};

use crate::schema::error::SchemaError;

const VALID_TYPES: &[&str] = &["null", "bool", "int", "float", "string", "bytes", "datetime", "array", "map"];
const KNOWN_KEYS: &[&str] = &["type", "optional", "enum", "fields", "closed", "items"];

/// A compiled field spec. `type_name` matches exactly what
/// `Value::type_name()` returns, so validation is a direct string compare.
#[derive(Clone, Debug)]
pub struct FieldSpec {
    pub type_name: String,
    pub optional: bool,
    pub enum_values: Option<Vec<Value>>,
    pub fields: Option<BTreeMap<String, FieldSpec>>,
    pub closed: bool,
    pub items: Option<Box<FieldSpec>>,
}

pub fn compile(raw: &Map, path: &mut Vec<String>) -> Result<FieldSpec, SchemaError> {
    for key in raw.keys() {
        if !KNOWN_KEYS.contains(&key.as_str()) {
            return Err(err(path, format!("unknown schema key '{key}'")));
        }
    }

    let type_name = match raw.get("type") {
        Some(Value::String(s)) => s.clone(),
        Some(_) => return Err(err(path, "'type' must be a string")),
        None => return Err(err(path, "missing required 'type'")),
    };
    if !VALID_TYPES.contains(&type_name.as_str()) {
        return Err(err(path, format!("unknown type '{type_name}'")));
    }

    let optional = match raw.get("optional") {
        None => false,
        Some(Value::Bool(b)) => *b,
        Some(_) => return Err(err(path, "'optional' must be a bool")),
    };

    let closed = match raw.get("closed") {
        None => false,
        Some(Value::Bool(b)) => *b,
        Some(_) => return Err(err(path, "'closed' must be a bool")),
    };

    let enum_values = match raw.get("enum") {
        None => None,
        Some(Value::Array(items)) => Some(items.clone()),
        Some(_) => return Err(err(path, "'enum' must be an array")),
    };

    let fields = match raw.get("fields") {
        None => None,
        Some(Value::Map(m)) => {
            let mut compiled = BTreeMap::new();
            for (key, spec_value) in m.iter() {
                let Value::Map(field_raw) = spec_value else {
                    return Err(err(path, format!("field '{key}' spec must be a map")));
                };
                path.push(format!("fields.{key}"));
                let spec = compile(field_raw, path)?;
                path.pop();
                compiled.insert(key.clone(), spec);
            }
            Some(compiled)
        }
        Some(_) => return Err(err(path, "'fields' must be a map")),
    };

    let items = match raw.get("items") {
        None => None,
        Some(Value::Map(item_raw)) => {
            path.push("items".to_string());
            let spec = compile(item_raw, path)?;
            path.pop();
            Some(Box::new(spec))
        }
        Some(_) => return Err(err(path, "'items' must be a map")),
    };

    Ok(FieldSpec { type_name, optional, enum_values, fields, closed, items })
}

fn err(path: &[String], message: impl Into<String>) -> SchemaError {
    SchemaError::new(path.join("."), message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text;

    fn compile_schema(src: &str) -> Result<FieldSpec, SchemaError> {
        let root = match text::parse(src).unwrap() {
            Value::Map(m) => m,
            _ => unreachable!("document root is always a map"),
        };
        compile(&root, &mut Vec::new())
    }

    #[test]
    fn minimal_schema_compiles() {
        let spec = compile_schema("type = \"map\"\n").unwrap();
        assert_eq!(spec.type_name, "map");
        assert!(!spec.optional);
        assert!(!spec.closed);
    }

    #[test]
    fn missing_type_rejected() {
        assert!(compile_schema("optional = true\n").is_err());
    }

    #[test]
    fn unknown_type_rejected() {
        assert!(compile_schema("type = \"integer\"\n").is_err());
    }

    #[test]
    fn unknown_key_rejected() {
        assert!(compile_schema("type = \"string\"\nregex = \"a+\"\n").is_err());
    }

    #[test]
    fn nested_fields_compile() {
        let spec = compile_schema(
            "type = \"map\"\nfields = {\n    name = { type = \"string\" }\n    port = { type = \"int\", optional = true }\n}\n",
        )
        .unwrap();
        let fields = spec.fields.unwrap();
        assert_eq!(fields["name"].type_name, "string");
        assert!(fields["port"].optional);
    }

    #[test]
    fn items_compile() {
        let spec = compile_schema("type = \"array\"\nitems = { type = \"string\" }\n").unwrap();
        assert_eq!(spec.items.unwrap().type_name, "string");
    }

    #[test]
    fn error_path_points_at_nested_field() {
        let err = compile_schema("type = \"map\"\nfields = {\n    server = { type = \"map\", fields = { port = { } } }\n}\n").unwrap_err();
        assert_eq!(err.path, "fields.server.fields.port");
    }
}
