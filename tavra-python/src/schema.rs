use pyo3::exceptions::{PyValueError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use tavra::schema;
use tavra::Value;

use crate::value::py_to_value;

fn extract_map(value: &Bound<'_, PyAny>, what: &str) -> PyResult<tavra::Map> {
    match py_to_value(value)? {
        Value::Map(m) => Ok(m),
        _ => Err(PyTypeError::new_err(format!("{what} must be a dict"))),
    }
}

/// Validates `doc` against `schema` (also a dict, describing the expected
/// shape). Returns a list of violation dicts
/// (`{"path": ..., "message": ...}`); an empty list means `doc` conforms.
/// Raises `ValueError` if `schema` itself is malformed, distinct from
/// `doc` merely not matching a well-formed schema.
#[pyfunction]
pub fn validate(py: Python<'_>, schema_value: &Bound<'_, PyAny>, doc: &Bound<'_, PyAny>) -> PyResult<Py<PyList>> {
    let schema_map = extract_map(schema_value, "schema")?;
    let doc_map = extract_map(doc, "doc")?;

    let violations = schema::validate(&schema_map, &doc_map).map_err(|e| PyValueError::new_err(e.to_string()))?;

    let list = PyList::empty(py);
    for v in violations {
        let d = PyDict::new(py);
        d.set_item("path", &v.path)?;
        d.set_item("message", &v.message)?;
        list.append(d)?;
    }
    Ok(list.unbind())
}
