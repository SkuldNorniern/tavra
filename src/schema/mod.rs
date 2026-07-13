//! Minimal optional schema validation. A schema is itself a `.tav`
//! document (`spec::compile`) checked against another document
//! (`validate::validate_root`). Validating never mutates either side.

mod error;
mod spec;
mod validate;

pub use error::{SchemaError, Violation};

use crate::value::Map;

/// Compiles `schema` and validates `doc` against it. Returns every
/// violation found (never stops at the first) — an empty vec means `doc`
/// conforms. `Err` means `schema` itself is malformed, which is reported
/// separately from document violations.
pub fn validate(schema: &Map, doc: &Map) -> Result<Vec<Violation>, SchemaError> {
    let mut path = Vec::new();
    let compiled = spec::compile(schema, &mut path)?;
    if compiled.type_name != "map" {
        return Err(SchemaError::new("", "root schema 'type' must be \"map\""));
    }
    Ok(validate::validate_root(&compiled, doc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text;
    use crate::value::Value;

    fn parse_map(src: &str) -> Map {
        match text::parse(src).unwrap() {
            Value::Map(m) => m,
            _ => unreachable!("document root is always a map"),
        }
    }

    #[test]
    fn end_to_end_valid_document() {
        let schema = parse_map("type = \"map\"\nfields = { name = { type = \"string\" } }\n");
        let doc = parse_map("name = \"demo\"\n");
        assert_eq!(validate(&schema, &doc).unwrap(), Vec::new());
    }

    #[test]
    fn non_map_root_type_is_schema_error() {
        let schema = parse_map("type = \"string\"\n");
        let doc = parse_map("name = \"demo\"\n");
        assert!(validate(&schema, &doc).is_err());
    }

    #[test]
    fn malformed_schema_is_schema_error_not_violation() {
        let schema = parse_map("type = \"not-a-real-type\"\n");
        let doc = parse_map("name = \"demo\"\n");
        assert!(validate(&schema, &doc).is_err());
    }
}
