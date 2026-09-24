use std::collections::BTreeMap;

use crate::value::{Map, Value};

use crate::schema::error::Violation;
use crate::schema::spec::FieldSpec;

enum PathSegment {
    Key(String),
    Index(usize),
}

fn render(path: &[PathSegment]) -> String {
    if path.is_empty() {
        return "<root>".to_string();
    }
    let mut s = String::new();
    for seg in path {
        match seg {
            PathSegment::Key(k) => {
                if !s.is_empty() {
                    s.push('.');
                }
                s.push_str(k);
            }
            PathSegment::Index(i) => {
                s.push('[');
                s.push_str(&i.to_string());
                s.push(']');
            }
        }
    }
    s
}

/// Validates the document root directly against a root schema (avoids
/// needing to wrap `doc` in a `Value::Map` just to satisfy `validate`'s
/// generic signature).
pub fn validate_root(spec: &FieldSpec, doc: &Map) -> Vec<Violation> {
    let mut out = Vec::new();
    let mut path = Vec::new();
    if let Some(allowed) = &spec.enum_values {
        if !allowed.iter().any(|v| matches!(v, Value::Map(m) if m == doc)) {
            out.push(Violation::new(render(&path), "value not in enum"));
        }
    }
    validate_map_body(spec, doc, &mut path, &mut out);
    out
}

fn validate(spec: &FieldSpec, value: &Value, path: &mut Vec<PathSegment>, out: &mut Vec<Violation>) {
    let actual_type = value.type_name();
    if actual_type != spec.type_name {
        out.push(Violation::new(render(path), format!("expected {}, found {actual_type}", spec.type_name)));
        return;
    }

    if let Some(allowed) = &spec.enum_values {
        if !allowed.contains(value) {
            out.push(Violation::new(render(path), "value not in enum"));
        }
    }

    match value {
        Value::Map(m) => validate_map_body(spec, m, path, out),
        Value::Array(items) => {
            if let Some(item_spec) = &spec.items {
                for (i, item) in items.iter().enumerate() {
                    path.push(PathSegment::Index(i));
                    validate(item_spec, item, path, out);
                    path.pop();
                }
            }
        }
        _ => {}
    }
}

fn validate_map_body(spec: &FieldSpec, map: &Map, path: &mut Vec<PathSegment>, out: &mut Vec<Violation>) {
    // closed with no fields = empty map only
    let no_fields = BTreeMap::new();
    let fields = spec.fields.as_ref().unwrap_or(&no_fields);
    for (key, field_spec) in fields.iter() {
        path.push(PathSegment::Key(key.clone()));
        match map.get(key) {
            Some(v) => validate(field_spec, v, path, out),
            None if !field_spec.optional => out.push(Violation::new(render(path), "missing required field")),
            None => {}
        }
        path.pop();
    }
    if spec.closed {
        for key in map.keys() {
            if !fields.contains_key(key) {
                path.push(PathSegment::Key(key.clone()));
                out.push(Violation::new(render(path), "unexpected key (map is closed)"));
                path.pop();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::spec::compile;
    use crate::text;

    fn check(schema_src: &str, doc_src: &str) -> Vec<Violation> {
        let schema_root = match text::parse(schema_src).unwrap() {
            Value::Map(m) => m,
            _ => unreachable!(),
        };
        let doc_root = match text::parse(doc_src).unwrap() {
            Value::Map(m) => m,
            _ => unreachable!(),
        };
        let spec = compile(&schema_root, &mut Vec::new()).unwrap();
        validate_root(&spec, &doc_root)
    }

    #[test]
    fn conforming_document_has_no_violations() {
        let schema = "type = \"map\"\nfields = {\n    name = { type = \"string\" }\n    port = { type = \"int\" }\n}\n";
        let doc = "name = \"demo\"\nport = 8080\n";
        assert_eq!(check(schema, doc), Vec::new());
    }

    #[test]
    fn missing_required_field_reported() {
        let schema = "type = \"map\"\nfields = {\n    name = { type = \"string\" }\n}\n";
        let violations = check(schema, "a = 1\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].path, "name");
    }

    #[test]
    fn optional_field_may_be_absent() {
        let schema = "type = \"map\"\nfields = {\n    name = { type = \"string\", optional = true }\n}\n";
        assert_eq!(check(schema, "a = 1\n"), Vec::new());
    }

    #[test]
    fn type_mismatch_reported() {
        let schema = "type = \"map\"\nfields = {\n    port = { type = \"int\" }\n}\n";
        let violations = check(schema, "port = \"8080\"\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].path, "port");
    }

    #[test]
    fn unlisted_key_allowed_when_open() {
        let schema = "type = \"map\"\nfields = {\n    name = { type = \"string\" }\n}\n";
        assert_eq!(check(schema, "name = \"demo\"\nextra = 1\n"), Vec::new());
    }

    #[test]
    fn unlisted_key_rejected_when_closed() {
        let schema = "type = \"map\"\nclosed = true\nfields = {\n    name = { type = \"string\" }\n}\n";
        let violations = check(schema, "name = \"demo\"\nextra = 1\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].path, "extra");
    }

    #[test]
    fn closed_without_fields_accepts_only_empty_map() {
        let schema = "type = \"map\"\nfields = {\n    m = { type = \"map\", closed = true }\n}\n";
        assert_eq!(check(schema, "m = {}\n"), Vec::new());
        let violations = check(schema, "m = { anything = 1 }\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].path, "m.anything");

        let root = "type = \"map\"\nclosed = true\n";
        assert_eq!(check(root, "x = 1\n").len(), 1);
    }

    #[test]
    fn root_enum_is_checked() {
        let schema = "type = \"map\"\nenum = [{ mode = \"a\" }, { mode = \"b\" }]\n";
        assert_eq!(check(schema, "mode = \"a\"\n"), Vec::new());
        let violations = check(schema, "mode = \"c\"\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].path, "<root>");
    }

    #[test]
    fn enum_violation_reported() {
        let schema = "type = \"map\"\nfields = {\n    level = { type = \"string\", enum = [\"debug\", \"info\"] }\n}\n";
        assert_eq!(check(schema, "level = \"debug\"\n"), Vec::new());
        let violations = check(schema, "level = \"trace\"\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].path, "level");
    }

    #[test]
    fn enum_is_type_strict() {
        // enum = [1] (int) must not match the float 1.0 — same strictness
        // as the value model's own 1 != 1.0 rule.
        let schema = "type = \"map\"\nfields = {\n    n = { type = \"float\", enum = [1] }\n}\n";
        let violations = check(schema, "n = 1.0\n");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn nested_map_violations_use_dotted_path() {
        let schema = "type = \"map\"\nfields = {\n    server = { type = \"map\", fields = { host = { type = \"string\" } } }\n}\n";
        let violations = check(schema, "server = { port = 1 }\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].path, "server.host");
    }

    #[test]
    fn array_item_violations_use_indexed_path() {
        let schema = "type = \"map\"\nfields = {\n    tags = { type = \"array\", items = { type = \"string\" } }\n}\n";
        let violations = check(schema, "tags = [\"a\", 2, \"c\"]\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].path, "tags[1]");
    }

    #[test]
    fn multiple_violations_all_collected() {
        let schema = "type = \"map\"\nfields = {\n    a = { type = \"string\" }\n    b = { type = \"int\" }\n}\n";
        let violations = check(schema, "a = 1\nb = \"x\"\n");
        assert_eq!(violations.len(), 2);
    }

    #[test]
    fn unshaped_map_and_array_accept_anything() {
        let schema = "type = \"map\"\nfields = {\n    anything = { type = \"map\" }\n}\n";
        assert_eq!(check(schema, "anything = { whatever = 1, another = \"x\" }\n"), Vec::new());
    }
}
